pub mod error;
pub mod models;
pub mod modes;
pub mod store;
pub mod suite;

#[cfg(test)]
mod mode_rules;

use craft_biz::bus::Bus;
use craft_biz::mcp::{normalize_tool_name, tool, ToolHost};
use serde_json::{json, Value};

use crate::models::CheckoutInput;
use crate::store::PosStore;

pub struct ShopHost {
    pub store: PosStore,
    pub bus: Bus,
}

impl ShopHost {
    pub fn new(store: PosStore) -> Self {
        Self {
            store,
            bus: Bus::new(Bus::default_root()),
        }
    }
}

impl ToolHost for ShopHost {
    fn server_name(&self) -> &str {
        "shopcraft"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<Value> {
        vec![
            tool(
                "shop_status",
                "Return POS item count, open shift ID, receipt count, and gross sales.",
                json!({}),
                &[],
            ),
            tool(
                "shop_seed",
                "Seed demo retail items with EAN-13 barcodes.",
                json!({}),
                &[],
            ),
            tool(
                "shop_catalog_list",
                "List active POS items, optionally filtered by category.",
                json!({ "category": { "type": "string" } }),
                &[],
            ),
            tool(
                "shop_shift_open",
                "Open a register shift with clerk name and opening cash float in cents.",
                json!({
                    "clerk_name": { "type": "string" },
                    "opening_float_cents": { "type": "integer" }
                }),
                &["clerk_name", "opening_float_cents"],
            ),
            tool(
                "shop_checkout",
                "Complete a POS sale by SKU or EAN barcode with split tender (cash, card, voucher).",
                json!({
                    "shift_id": { "type": "string" },
                    "items": { "type": "array" },
                    "discount_bps": { "type": "integer" },
                    "cash_cents": { "type": "integer" },
                    "card_cents": { "type": "integer" },
                    "voucher_cents": { "type": "integer" }
                }),
                &["shift_id", "items", "cash_cents", "card_cents", "voucher_cents"],
            ),
            tool(
                "shop_storno",
                "Issue an immutable STORNO counter-receipt and restore item stock.",
                json!({ "receipt_no": { "type": "string" } }),
                &["receipt_no"],
            ),
            tool(
                "shop_shift_close",
                "Close the active shift with counted cash and generate an immutable Z-Bon.",
                json!({
                    "shift_id": { "type": "string" },
                    "counted_cash_cents": { "type": "integer" }
                }),
                &["shift_id", "counted_cash_cents"],
            ),
            tool(
                "shop_receipt_print",
                "Render a 42-column ESC/POS thermal receipt preview.",
                json!({ "receipt_no": { "type": "string" } }),
                &["receipt_no"],
            ),
        ]
    }

    fn call(&mut self, name: &str, args: &Value) -> std::result::Result<Value, String> {
        let normalized = normalize_tool_name(name);
        match normalized.as_str() {
            "shop_status" => self
                .store
                .status()
                .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "shop_seed" => self
                .store
                .seed_demo()
                .map(|c| json!({ "seeded_items": c }))
                .map_err(|e| e.to_string()),
            "shop_catalog_list" => {
                let category = args.get("category").and_then(Value::as_str);
                self.store
                    .list_items(category)
                    .and_then(|i| serde_json::to_value(i).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "shop_shift_open" => {
                let clerk = args
                    .get("clerk_name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing clerk_name".to_string())?;
                let float_cents = args
                    .get("opening_float_cents")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| "missing opening_float_cents".to_string())?;
                self.store
                    .open_shift(clerk, float_cents)
                    .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "shop_checkout" => {
                let input: CheckoutInput =
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
                let receipt = self.store.checkout(&input).map_err(|e| e.to_string())?;
                let _ = self.bus.emit(
                    "shopcraft",
                    "stockcraft",
                    "channel.sale_committed",
                    json!({ "receipt_no": receipt.receipt_no, "lines": receipt.lines }),
                );
                serde_json::to_value(receipt).map_err(|e| e.to_string())
            }
            "shop_storno" => {
                let receipt_no = args
                    .get("receipt_no")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing receipt_no".to_string())?;
                let storno = self
                    .store
                    .storno_receipt(receipt_no)
                    .map_err(|e| e.to_string())?;
                let _ = self.bus.emit(
                    "shopcraft",
                    "stockcraft",
                    "channel.return_restocked",
                    json!({ "receipt_no": storno.receipt_no, "storno_of": storno.storno_of, "lines": storno.lines }),
                );
                serde_json::to_value(storno).map_err(|e| e.to_string())
            }
            "shop_shift_close" => {
                let shift_id = args
                    .get("shift_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing shift_id".to_string())?;
                let counted = args
                    .get("counted_cash_cents")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| "missing counted_cash_cents".to_string())?;
                let zbon = self
                    .store
                    .close_shift(shift_id, counted)
                    .map_err(|e| e.to_string())?;
                let _ = self.bus.emit(
                    "shopcraft",
                    "buchcraft",
                    "pos.zbon_closed",
                    serde_json::to_value(&zbon).unwrap_or(Value::Null),
                );
                serde_json::to_value(zbon).map_err(|e| e.to_string())
            }
            "shop_receipt_print" => {
                let receipt_no = args
                    .get("receipt_no")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing receipt_no".to_string())?;
                self.store
                    .format_receipt_text(receipt_no)
                    .map(|text| json!({ "receipt_no": receipt_no, "text": text }))
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
    use crate::models::CheckoutLineInput;

    #[test]
    fn split_tender_checkout_storno_and_zbon() {
        let mut store = PosStore::open_in_memory().expect("store");
        store.seed_demo().expect("seed");

        let shift = store.open_shift("Lukas", 10_000).expect("open shift");

        // Scan by EAN-13 barcode: 9001234000018 (CER-VASE-01, 6800c) + WRK-NOTE-06 (2400c, 10% VAT) = 9200c
        // Pay 5000 cash + 5000 card = 10000 -> change 800c -> net cash in drawer +4200c (14200c expected)
        let r1 = store
            .checkout(&CheckoutInput {
                shift_id: shift.shift_id.clone(),
                items: vec![
                    CheckoutLineInput {
                        code: "9001234000018".into(),
                        qty: 1,
                    },
                    CheckoutLineInput {
                        code: "WRK-NOTE-06".into(),
                        qty: 1,
                    },
                ],
                discount_bps: None,
                cash_cents: 5000,
                card_cents: 5000,
                voucher_cents: 0,
            })
            .expect("checkout");

        assert_eq!(r1.total_cents, 9200);
        assert_eq!(r1.change_cents, 800);
        assert!(r1.vat_20_cents > 0);
        assert!(r1.vat_10_cents > 0);

        let text = store.format_receipt_text(&r1.receipt_no).expect("print");
        assert!(text.contains("ATELIER NORD VIENNA"));
        assert!(text.contains(&r1.rksv_sig));

        // Storno restores stock and reverses net cash
        let storno = store.storno_receipt(&r1.receipt_no).expect("storno");
        assert_eq!(storno.kind, "STORNO");
        assert_eq!(storno.total_cents, -9200);

        let zbon = store.close_shift(&shift.shift_id, 10_000).expect("close");
        assert_eq!(zbon.receipt_count, 1);
        assert_eq!(zbon.storno_count, 1);
        assert_eq!(zbon.gross_total_cents, 0);
        assert_eq!(zbon.cash_diff_cents, 0);
    }
}
