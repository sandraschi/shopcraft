use std::path::PathBuf;

use craft_biz::bus::Bus;
use craft_biz::mcp::{normalize_tool_name, tool, ToolHost};
use serde_json::{json, Value};

use super::error::Result;
use super::models::SubscribeInput;
use super::store::Store;

pub struct SubShopHost {
    pub store: Store,
    pub storefront_dir: PathBuf,
    pub bus: Bus,
}

impl SubShopHost {
    pub fn new(store: Store, storefront_dir: PathBuf) -> Self {
        Self {
            store,
            storefront_dir,
            bus: Bus::new(Bus::default_root()),
        }
    }

    pub fn export_storefront_to_disk(&self, store_name: &str, tagline: &str) -> Result<PathBuf> {
        let export = self.store.export_storefront(store_name, tagline)?;
        std::fs::create_dir_all(&self.storefront_dir)?;
        let out_path = self.storefront_dir.join("catalog.json");
        let json_str = serde_json::to_string_pretty(&export)?;
        std::fs::write(&out_path, json_str)?;
        Ok(out_path)
    }
}

impl ToolHost for SubShopHost {
    fn server_name(&self) -> &str {
        "subshopcraft"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<Value> {
        vec![
            tool(
                "sub_status",
                "Return active subscription plans, active/paused subscribers, MRR in cents, cycles billed, and skips.",
                json!({}),
                &[],
            ),
            tool("sub_seed", "Seed 6 demo recurring subscription box plans.", json!({}), &[]),
            tool("sub_plans", "List active subscription plans.", json!({}), &[]),
            tool("sub_list", "List customer subscriptions.", json!({}), &[]),
            tool(
                "sub_subscribe",
                "Create a new SEPA pain.008 subscription and emit Domino Bus events.",
                json!({
                    "customer_name": { "type": "string" },
                    "customer_email": { "type": "string" },
                    "iban": { "type": "string" },
                    "plan_sku": { "type": "string" }
                }),
                &["customer_name", "customer_email", "iban", "plan_sku"],
            ),
            tool(
                "sub_skip",
                "Skip the next renewal box and advance the next billing date.",
                json!({ "sub_number": { "type": "string" } }),
                &["sub_number"],
            ),
            tool(
                "sub_bill",
                "Bill the next renewal cycle and emit SEPA pain.008 / Domino Bus events.",
                json!({ "sub_number": { "type": "string" } }),
                &["sub_number"],
            ),
            tool(
                "sub_set_status",
                "Pause, resume (Active), or Cancel a subscription.",
                json!({
                    "sub_number": { "type": "string" },
                    "status": { "type": "string" }
                }),
                &["sub_number", "status"],
            ),
            tool(
                "sub_swap_plan",
                "Swap a subscription to a different plan SKU.",
                json!({
                    "sub_number": { "type": "string" },
                    "plan_sku": { "type": "string" }
                }),
                &["sub_number", "plan_sku"],
            ),
            tool(
                "sub_export_storefront",
                "Export storefront/catalog.json for the subscription portal.",
                json!({
                    "store_name": { "type": "string" },
                    "tagline": { "type": "string" }
                }),
                &[],
            ),
        ]
    }

    fn call(&mut self, name: &str, args: &Value) -> std::result::Result<Value, String> {
        match normalize_tool_name(name).as_str() {
            "sub_status" => self
                .store
                .status()
                .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "sub_seed" => {
                let n = self.store.seed_demo().map_err(|e| e.to_string())?;
                Ok(json!({ "plans_inserted": n }))
            }
            "sub_plans" => self
                .store
                .list_plans()
                .and_then(|p| serde_json::to_value(p).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "sub_list" => self
                .store
                .list_subscriptions()
                .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "sub_subscribe" => {
                let input: SubscribeInput =
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
                let sub = self.store.subscribe(&input).map_err(|e| e.to_string())?;
                let payload = json!({
                    "sub_number": sub.sub_number,
                    "mandate_ref": sub.mandate_ref,
                    "plan_sku": sub.plan_sku,
                    "amount_cents": sub.net_price_cents,
                });
                let _ = self.bus.emit(
                    "subshopcraft",
                    "billcraft",
                    "sub.mandate_created",
                    payload.clone(),
                );
                let _ = self.bus.emit(
                    "subshopcraft",
                    "stockcraft",
                    "channel.sale_committed",
                    payload,
                );
                serde_json::to_value(sub).map_err(|e| e.to_string())
            }
            "sub_skip" => {
                let sub_no = args
                    .get("sub_number")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing sub_number".to_string())?;
                self.store
                    .skip_next(sub_no)
                    .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "sub_bill" => {
                let sub_no = args
                    .get("sub_number")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing sub_number".to_string())?;
                let sub = self.store.bill_renewal(sub_no).map_err(|e| e.to_string())?;
                let payload = json!({
                    "sub_number": sub.sub_number,
                    "mandate_ref": sub.mandate_ref,
                    "plan_sku": sub.plan_sku,
                    "amount_cents": sub.net_price_cents,
                });
                let _ = self
                    .bus
                    .emit("subshopcraft", "billcraft", "order.paid", payload.clone());
                let _ = self
                    .bus
                    .emit("subshopcraft", "buchcraft", "sub.renewal_billed", payload);
                serde_json::to_value(sub).map_err(|e| e.to_string())
            }
            "sub_set_status" => {
                let sub_no = args
                    .get("sub_number")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing sub_number".to_string())?;
                let status = args
                    .get("status")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing status".to_string())?;
                self.store
                    .set_status(sub_no, status)
                    .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "sub_swap_plan" => {
                let sub_no = args
                    .get("sub_number")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing sub_number".to_string())?;
                let plan_sku = args
                    .get("plan_sku")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing plan_sku".to_string())?;
                self.store
                    .swap_plan(sub_no, plan_sku)
                    .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "sub_export_storefront" => {
                let store_name = args
                    .get("store_name")
                    .and_then(Value::as_str)
                    .unwrap_or("ALPINE REPLENISH CLUB");
                let tagline = args
                    .get("tagline")
                    .and_then(Value::as_str)
                    .unwrap_or("Local-First Subscriptions, SEPA pain.008 & Skip/Swap Control");
                let out = self
                    .export_storefront_to_disk(store_name, tagline)
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "exported_path": out.display().to_string() }))
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}
