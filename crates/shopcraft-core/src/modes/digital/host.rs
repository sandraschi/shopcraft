use std::path::PathBuf;

use craft_biz::bus::Bus;
use craft_biz::mcp::{normalize_tool_name, tool, ToolHost};
use serde_json::{json, Value};

use super::error::Result;
use super::models::DigitalOrderInput;
use super::store::Store;

pub struct DigitalShopHost {
    pub store: Store,
    pub storefront_dir: PathBuf,
    pub bus: Bus,
}

impl DigitalShopHost {
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

impl ToolHost for DigitalShopHost {
    fn server_name(&self) -> &str {
        "digitalshopcraft"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<Value> {
        vec![
            tool(
                "digital_status",
                "Return active digital products, orders, licenses, downloads, gross revenue, and EU OSS VAT collected in cents.",
                json!({}),
                &[],
            ),
            tool(
                "digital_seed",
                "Seed EU/CH VAT OSS rates and 6 demo digital products (licenses, ebooks, audio, CAD, courses, fonts).",
                json!({}),
                &[],
            ),
            tool(
                "digital_catalog",
                "List active digital products.",
                json!({}),
                &[],
            ),
            tool(
                "digital_vat_rates",
                "List EU VAT OSS and export country rates.",
                json!({}),
                &[],
            ),
            tool(
                "digital_checkout",
                "Complete an instant digital checkout, calculate EU VAT OSS, issue license keys & download tokens, and emit Domino Bus events.",
                json!({
                    "buyer_name": { "type": "string" },
                    "buyer_email": { "type": "string" },
                    "country_code": { "type": "string" },
                    "items": { "type": "array" }
                }),
                &["buyer_name", "buyer_email", "country_code", "items"],
            ),
            tool(
                "digital_redeem",
                "Redeem a download token (increments download count and enforces limits/revocation).",
                json!({ "download_token": { "type": "string" } }),
                &["download_token"],
            ),
            tool(
                "digital_revoke",
                "Revoke an issued license key.",
                json!({ "license_key": { "type": "string" } }),
                &["license_key"],
            ),
            tool(
                "digital_orders",
                "List digital orders and their issued licenses.",
                json!({}),
                &[],
            ),
            tool(
                "digital_export_storefront",
                "Export storefront/catalog.json for the digital storefront.",
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
            "digital_status" => self
                .store
                .status()
                .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "digital_seed" => {
                let (rates, products) = self.store.seed_demo().map_err(|e| e.to_string())?;
                Ok(json!({ "vat_rates_inserted": rates, "products_inserted": products }))
            }
            "digital_catalog" => self
                .store
                .list_products()
                .and_then(|p| serde_json::to_value(p).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "digital_vat_rates" => self
                .store
                .list_vat_rates()
                .and_then(|v| serde_json::to_value(v).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "digital_checkout" => {
                let input: DigitalOrderInput =
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
                let order = self.store.checkout(&input).map_err(|e| e.to_string())?;
                let payload = json!({
                    "order_number": order.order_number,
                    "country_code": order.country_code,
                    "vat_pct": order.vat_pct,
                    "vat_cents": order.vat_cents,
                    "total_cents": order.total_cents,
                    "licenses": order.licenses.len(),
                });
                let _ = self.bus.emit(
                    "digitalshopcraft",
                    "billcraft",
                    "order.paid",
                    payload.clone(),
                );
                let _ = self.bus.emit(
                    "digitalshopcraft",
                    "buchcraft",
                    "digital.oss_vat_booked",
                    payload,
                );
                serde_json::to_value(order).map_err(|e| e.to_string())
            }
            "digital_redeem" => {
                let tok = args
                    .get("download_token")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing download_token".to_string())?;
                self.store
                    .redeem_download(tok)
                    .and_then(|l| serde_json::to_value(l).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "digital_revoke" => {
                let key = args
                    .get("license_key")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing license_key".to_string())?;
                self.store.revoke_license(key).map_err(|e| e.to_string())?;
                Ok(json!({ "revoked": key }))
            }
            "digital_orders" => self
                .store
                .list_orders()
                .and_then(|o| serde_json::to_value(o).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "digital_export_storefront" => {
                let store_name = args
                    .get("store_name")
                    .and_then(Value::as_str)
                    .unwrap_or("KRAFT DIGITAL VAULT");
                let tagline = args
                    .get("tagline")
                    .and_then(Value::as_str)
                    .unwrap_or("Instant License Keys, Downloads & EU VAT OSS");
                let out = self
                    .export_storefront_to_disk(store_name, tagline)
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "exported_path": out.display().to_string() }))
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}
