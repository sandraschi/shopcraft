use craft_biz::bus::Bus;
use craft_biz::mcp::{normalize_tool_name, tool, ToolHost};
use serde_json::{json, Value};

use super::models::RecordOrderInput;
use super::store::AmazonStore;

pub struct AmazonHost {
    pub store: AmazonStore,
    pub bus: Bus,
}

impl AmazonHost {
    pub fn new(store: AmazonStore) -> Self {
        Self {
            store,
            bus: Bus::new(Bus::default_root()),
        }
    }
}

impl ToolHost for AmazonHost {
    fn server_name(&self) -> &str {
        "amazonshopcraft"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<Value> {
        vec![
            tool(
                "amazon_status",
                "Return active ASIN count, FBA/FBM units, pending VAT invoices, and net profit.",
                json!({}),
                &[],
            ),
            tool(
                "amazon_seed",
                "Seed demo Amazon DE/EU listings with COGS, referral/FBA fees, and velocity.",
                json!({}),
                &[],
            ),
            tool(
                "amazon_listings",
                "List active Amazon listings, optionally filtered by marketplace (e.g. DE).",
                json!({ "marketplace": { "type": "string" } }),
                &[],
            ),
            tool(
                "amazon_unit_economics",
                "Compute exact net revenue, VAT, referral/FBA fees, PPC, and net profit in cents for an ASIN.",
                json!({ "asin": { "type": "string" } }),
                &["asin"],
            ),
            tool(
                "amazon_reprice",
                "Run Buy Box repricer against a competitor price clamped to [min_price, max_price].",
                json!({
                    "asin": { "type": "string" },
                    "competitor_cents": { "type": "integer" }
                }),
                &["asin", "competitor_cents"],
            ),
            tool(
                "amazon_replenishment",
                "List FBA listings below target days-of-cover with recommended FBM->FBA ship quantities.",
                json!({ "target_days": { "type": "integer" } }),
                &[],
            ),
            tool(
                "amazon_order_record",
                "Record an Amazon order and compute its net profit and VAT.",
                json!({
                    "amazon_order_id": { "type": "string" },
                    "asin": { "type": "string" },
                    "qty": { "type": "integer" },
                    "status": { "type": "string" }
                }),
                &["amazon_order_id", "asin", "qty", "status"],
            ),
            tool(
                "amazon_invoice_uploaded",
                "Mark an Amazon order's VAT invoice as uploaded to SP-API.",
                json!({ "amazon_order_id": { "type": "string" } }),
                &["amazon_order_id"],
            ),
        ]
    }

    fn call(&mut self, name: &str, args: &Value) -> std::result::Result<Value, String> {
        let normalized = normalize_tool_name(name);
        match normalized.as_str() {
            "amazon_status" => self
                .store
                .status()
                .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "amazon_seed" => self
                .store
                .seed_demo()
                .map(|c| json!({ "seeded_listings": c }))
                .map_err(|e| e.to_string()),
            "amazon_listings" => {
                let mp = args.get("marketplace").and_then(Value::as_str);
                self.store
                    .list_listings(mp)
                    .and_then(|l| serde_json::to_value(l).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "amazon_unit_economics" => {
                let asin = args
                    .get("asin")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing asin".to_string())?;
                self.store
                    .unit_economics(asin)
                    .and_then(|u| serde_json::to_value(u).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "amazon_reprice" => {
                let asin = args
                    .get("asin")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing asin".to_string())?;
                let comp = args
                    .get("competitor_cents")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| "missing competitor_cents".to_string())?;
                self.store
                    .reprice(asin, comp)
                    .and_then(|r| serde_json::to_value(r).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "amazon_replenishment" => {
                let target_days = args
                    .get("target_days")
                    .and_then(Value::as_i64)
                    .unwrap_or(14);
                self.store
                    .replenishment_alerts(target_days)
                    .and_then(|a| serde_json::to_value(a).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "amazon_order_record" => {
                let input: RecordOrderInput =
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
                let order = self.store.record_order(&input).map_err(|e| e.to_string())?;
                let _ = self.bus.emit(
                    "amazonshopcraft",
                    "stockcraft",
                    "channel.sale_committed",
                    json!({ "amazon_order_id": order.amazon_order_id, "sku": order.sku, "qty": order.qty }),
                );
                let _ = self.bus.emit(
                    "amazonshopcraft",
                    "billcraft",
                    "amazon.invoice_requested",
                    json!({
                        "amazon_order_id": order.amazon_order_id,
                        "marketplace": order.marketplace,
                        "gross_cents": order.gross_cents,
                        "vat_cents": order.vat_cents
                    }),
                );
                serde_json::to_value(order).map_err(|e| e.to_string())
            }
            "amazon_invoice_uploaded" => {
                let id = args
                    .get("amazon_order_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing amazon_order_id".to_string())?;
                self.store
                    .mark_invoice_uploaded(id)
                    .map(|()| json!({ "amazon_order_id": id, "invoice_uploaded": true }))
                    .map_err(|e| e.to_string())
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn unit_economics_repricer_floor_and_replenishment() {
        let store = AmazonStore::open_in_memory().expect("store");
        store.seed_demo().expect("seed");

        let econ = store.unit_economics("B0ATELIER1").expect("econ");
        assert!(econ.net_profit_cents > 0);
        assert_eq!(econ.gross_price_cents, 6990);

        // Competitor at 5000c -> clamps to min_price_cents (5900c), Buy Box lost
        let rep_floor = store.reprice("B0ATELIER1", 5000).expect("reprice");
        assert_eq!(rep_floor.new_price_cents, 5900);
        assert!(rep_floor.clamped_to_floor);
        assert!(!rep_floor.buy_box_won);

        // Replenishment at 14-day target flags B0ATELIER1 (8 days) and B0ATELIER3 (5 days)
        let alerts = store.replenishment_alerts(14).expect("alerts");
        assert_eq!(alerts.len(), 2);
    }
}
