use std::path::PathBuf;

use craft_biz::bus::Bus;
use craft_biz::mcp::{normalize_tool_name, tool, ToolHost};
use serde_json::{json, Value};

use super::error::Result;
use super::models::{CreateOrderInput, OrderLineInput};
use super::store::Store;

pub struct WholesaleHost {
    pub store: Store,
    pub portal_dir: PathBuf,
    pub bus: Bus,
}

impl WholesaleHost {
    pub fn new(store: Store, portal_dir: PathBuf) -> Self {
        Self {
            store,
            portal_dir,
            bus: Bus::new(Bus::default_root()),
        }
    }

    pub fn export_portal_to_disk(&self, portal_name: &str, tagline: &str) -> Result<PathBuf> {
        let export = self.store.export_portal(portal_name, tagline)?;
        std::fs::create_dir_all(&self.portal_dir)?;
        let out_path = self.portal_dir.join("catalog.json");
        let json_str = serde_json::to_string_pretty(&export)?;
        std::fs::write(&out_path, json_str)?;
        Ok(out_path)
    }
}

impl ToolHost for WholesaleHost {
    fn server_name(&self) -> &str {
        "wholesalecraft"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<Value> {
        vec![
            tool(
                "wholesale_status",
                "Return B2B catalog counts, stock units, trade accounts, open POs, credit exposure, and revenue in cents.",
                json!({}),
                &[],
            ),
            tool(
                "wholesale_seed",
                "Seed demo B2B trade accounts (AT, DE, CH, NL) and wholesale catalog SKUs with MOQ and pack sizes.",
                json!({}),
                &[],
            ),
            tool(
                "wholesale_accounts",
                "List B2B trade accounts with tier, UID verification, payment terms, credit limit, and open balance.",
                json!({}),
                &[],
            ),
            tool(
                "wholesale_catalog",
                "List wholesale SKUs with MOQ, pack size, list price, and Silver/Gold tier discounts.",
                json!({ "category": { "type": "string" } }),
                &[],
            ),
            tool(
                "wholesale_quote",
                "Calculate a B2B quote with tier discount, volume bonus, MOQ/pack validation, EU VAT reverse charge, and credit check.",
                json!({
                    "account_code": { "type": "string" },
                    "items": { "type": "array" }
                }),
                &["account_code", "items"],
            ),
            tool(
                "wholesale_order_create",
                "Create a B2B Purchase Order, reserve stock, update account credit balance, and emit Domino Bus events.",
                json!({
                    "account_code": { "type": "string" },
                    "po_reference": { "type": "string" },
                    "items": { "type": "array" }
                }),
                &["account_code", "items"],
            ),
            tool(
                "wholesale_order_transition",
                "Transition a Purchase Order status (Approved, Dispatched, Invoiced, Cancelled).",
                json!({
                    "order_number": { "type": "string" },
                    "status": { "type": "string" }
                }),
                &["order_number", "status"],
            ),
            tool(
                "wholesale_order_list",
                "List B2B Purchase Orders with line items and reverse-charge status.",
                json!({}),
                &[],
            ),
            tool(
                "wholesale_export_portal",
                "Export portal/catalog.json for the B2B Quick-Order Portal.",
                json!({
                    "portal_name": { "type": "string" },
                    "tagline": { "type": "string" }
                }),
                &[],
            ),
        ]
    }

    fn call(&mut self, name: &str, args: &Value) -> std::result::Result<Value, String> {
        match normalize_tool_name(name).as_str() {
            "wholesale_status" => self
                .store
                .status()
                .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "wholesale_seed" => {
                let (accounts, products) = self.store.seed_demo().map_err(|e| e.to_string())?;
                Ok(json!({ "accounts_inserted": accounts, "products_inserted": products }))
            }
            "wholesale_accounts" => self
                .store
                .list_accounts()
                .and_then(|a| serde_json::to_value(a).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "wholesale_catalog" => {
                let category = args.get("category").and_then(Value::as_str);
                self.store
                    .list_products(category)
                    .and_then(|p| serde_json::to_value(p).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "wholesale_quote" => {
                let account_code = args
                    .get("account_code")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing account_code".to_string())?;
                let items_val = args
                    .get("items")
                    .ok_or_else(|| "missing items".to_string())?;
                let items: Vec<OrderLineInput> =
                    serde_json::from_value(items_val.clone()).map_err(|e| e.to_string())?;
                self.store
                    .quote(account_code, &items)
                    .and_then(|q| serde_json::to_value(q).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "wholesale_order_create" => {
                let input: CreateOrderInput =
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
                let order = self.store.create_order(&input).map_err(|e| e.to_string())?;
                let payload = json!({
                    "order_number": order.order_number,
                    "account_code": order.account_code,
                    "total_cents": order.total_cents,
                    "reverse_charge": order.reverse_charge,
                    "lines": order.lines,
                });
                let _ = self.bus.emit(
                    "wholesalecraft",
                    "stockcraft",
                    "channel.stock_reserved",
                    payload.clone(),
                );
                let _ = self.bus.emit(
                    "wholesalecraft",
                    "billcraft",
                    "b2b.order_submitted",
                    payload,
                );
                serde_json::to_value(order).map_err(|e| e.to_string())
            }
            "wholesale_order_transition" => {
                let order_number = args
                    .get("order_number")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing order_number".to_string())?;
                let status = args
                    .get("status")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing status".to_string())?;
                let order = self
                    .store
                    .transition_order(order_number, status)
                    .map_err(|e| e.to_string())?;
                let payload = json!({
                    "order_number": order.order_number,
                    "account_code": order.account_code,
                    "status": order.status,
                    "total_cents": order.total_cents,
                    "vat_cents": order.vat_cents,
                    "reverse_charge": order.reverse_charge,
                });
                if order.status == "Invoiced" {
                    let _ = self.bus.emit(
                        "wholesalecraft",
                        "billcraft",
                        "b2b.invoice_due",
                        payload.clone(),
                    );
                    let _ =
                        self.bus
                            .emit("wholesalecraft", "buchcraft", "b2b.invoice_due", payload);
                }
                serde_json::to_value(order).map_err(|e| e.to_string())
            }
            "wholesale_order_list" => self
                .store
                .list_orders()
                .and_then(|o| serde_json::to_value(o).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "wholesale_export_portal" => {
                let portal_name = args
                    .get("portal_name")
                    .and_then(Value::as_str)
                    .unwrap_or("NORDIC TRADE PORTAL");
                let tagline = args
                    .get("tagline")
                    .and_then(Value::as_str)
                    .unwrap_or("Local-first B2B Wholesale & Net-30/60 Ordering");
                let out = self
                    .export_portal_to_disk(portal_name, tagline)
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "exported_path": out.display().to_string() }))
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}
