use std::path::PathBuf;

use craft_biz::bus::Bus;
use craft_biz::mcp::{normalize_tool_name, tool, ToolHost};
use serde_json::{json, Value};

use super::error::Result;
use super::models::CreateOrderInput;
use super::store::Store;

pub struct WebshopHost {
    pub store: Store,
    pub storefront_dir: PathBuf,
    pub bus: Bus,
}

impl WebshopHost {
    pub fn new(store: Store, storefront_dir: PathBuf) -> Self {
        Self {
            store,
            storefront_dir,
            bus: Bus::new(Bus::default_root()),
        }
    }

    pub fn export_storefront_to_disk(&self, shop_name: &str, tagline: &str) -> Result<PathBuf> {
        let export = self.store.export_storefront(shop_name, tagline)?;
        std::fs::create_dir_all(&self.storefront_dir)?;
        let out_path = self.storefront_dir.join("catalog.json");
        let json_str = serde_json::to_string_pretty(&export)?;
        std::fs::write(&out_path, json_str)?;
        Ok(out_path)
    }
}

impl ToolHost for WebshopHost {
    fn server_name(&self) -> &str {
        "webshopcraft"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<Value> {
        vec![
            tool(
                "webshop_status",
                "Return store counts, stock units, open orders, and gross revenue in cents.",
                json!({}),
                &[],
            ),
            tool(
                "webshop_seed",
                "Seed demo products, promo codes, and shipping zones (AT, DE, EU).",
                json!({}),
                &[],
            ),
            tool(
                "webshop_catalog_list",
                "List active webshop products, optionally filtered by category.",
                json!({
                    "category": { "type": "string", "description": "Optional category filter" }
                }),
                &[],
            ),
            tool(
                "webshop_order_create",
                "Create a customer order with stock reservation, promo validation, and shipping.",
                json!({
                    "customer_name": { "type": "string" },
                    "customer_email": { "type": "string" },
                    "shipping_address": { "type": "string" },
                    "zone_code": { "type": "string", "description": "AT, DE, or EU" },
                    "promo_code": { "type": "string" },
                    "items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "sku": { "type": "string" },
                                "qty": { "type": "integer" }
                            },
                            "required": ["sku", "qty"]
                        }
                    }
                }),
                &[
                    "customer_name",
                    "customer_email",
                    "shipping_address",
                    "zone_code",
                    "items",
                ],
            ),
            tool(
                "webshop_order_list",
                "List orders, optionally filtered by status (NEW, PAID, PICKING, SHIPPED).",
                json!({
                    "status": { "type": "string" }
                }),
                &[],
            ),
            tool(
                "webshop_order_pay",
                "Mark a NEW order as PAID.",
                json!({
                    "order_no": { "type": "string" }
                }),
                &["order_no"],
            ),
            tool(
                "webshop_order_ship",
                "Mark a PAID/PICKING order as SHIPPED with carrier and tracking number.",
                json!({
                    "order_no": { "type": "string" },
                    "carrier": { "type": "string" },
                    "tracking_no": { "type": "string" }
                }),
                &["order_no", "carrier", "tracking_no"],
            ),
            tool(
                "webshop_export_storefront",
                "Export catalog.json into the storefront directory for the HTML shop front.",
                json!({
                    "shop_name": { "type": "string" },
                    "tagline": { "type": "string" }
                }),
                &[],
            ),
        ]
    }

    fn call(&mut self, name: &str, args: &Value) -> std::result::Result<Value, String> {
        let normalized = normalize_tool_name(name);
        match normalized.as_str() {
            "webshop_status" => self
                .store
                .status()
                .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "webshop_seed" => self
                .store
                .seed_demo()
                .map(|count| json!({ "seeded_products": count }))
                .map_err(|e| e.to_string()),
            "webshop_catalog_list" => {
                let category = args.get("category").and_then(Value::as_str);
                self.store
                    .list_products(category, true)
                    .and_then(|p| serde_json::to_value(p).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "webshop_order_create" => {
                let input: CreateOrderInput =
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
                let order = self.store.create_order(&input).map_err(|e| e.to_string())?;
                let _ = self.bus.emit(
                    "webshopcraft",
                    "stockcraft",
                    "channel.stock_reserved",
                    json!({ "order_no": order.order_no, "items": order.items }),
                );
                serde_json::to_value(order).map_err(|e| e.to_string())
            }
            "webshop_order_list" => {
                let status = args.get("status").and_then(Value::as_str);
                self.store
                    .list_orders(status)
                    .and_then(|o| serde_json::to_value(o).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "webshop_order_pay" => {
                let order_no = args
                    .get("order_no")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing order_no".to_string())?;
                let order = self.store.mark_paid(order_no).map_err(|e| e.to_string())?;
                let payload = json!({
                    "order_no": order.order_no,
                    "total_cents": order.total_cents,
                    "vat_cents": order.vat_cents,
                    "customer_name": order.customer_name,
                    "customer_email": order.customer_email,
                    "items": order.items
                });
                let _ = self.bus.emit(
                    "webshopcraft",
                    "stockcraft",
                    "channel.sale_committed",
                    payload.clone(),
                );
                let _ = self
                    .bus
                    .emit("webshopcraft", "billcraft", "order.paid", payload.clone());
                let _ = self
                    .bus
                    .emit("webshopcraft", "buchcraft", "order.paid", payload);
                serde_json::to_value(order).map_err(|e| e.to_string())
            }
            "webshop_order_ship" => {
                let order_no = args
                    .get("order_no")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing order_no".to_string())?;
                let carrier = args
                    .get("carrier")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing carrier".to_string())?;
                let tracking_no = args
                    .get("tracking_no")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing tracking_no".to_string())?;
                self.store
                    .ship_order(order_no, carrier, tracking_no)
                    .and_then(|o| serde_json::to_value(o).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "webshop_export_storefront" => {
                let shop_name = args
                    .get("shop_name")
                    .and_then(Value::as_str)
                    .unwrap_or("ATELIER NORD");
                let tagline = args.get("tagline").and_then(Value::as_str).unwrap_or(
                    "Tactile objects for quiet spaces -- crafted in small European runs.",
                );
                self.export_storefront_to_disk(shop_name, tagline)
                    .map(|p| json!({ "exported_to": p.display().to_string() }))
                    .map_err(|e| e.to_string())
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::models::OrderItemInput;
    use super::*;

    #[test]
    fn seed_and_order_lifecycle_with_promo_and_free_shipping() {
        let mut store = Store::open_in_memory().expect("in-memory store");
        let count = store.seed_demo().expect("seed");
        assert_eq!(count, 6);

        // 2x CER-VASE-01 (6800 * 2 = 13600), 10% off WELCOME10 -> 1360 discount -> 12240 net >= 9000 free AT shipping
        let order = store
            .create_order(&CreateOrderInput {
                customer_name: "Clara Novak".into(),
                customer_email: "clara@example.at".into(),
                shipping_address: "Neubaugasse 12, 1070 Wien".into(),
                zone_code: "AT".into(),
                promo_code: Some("WELCOME10".into()),
                items: vec![OrderItemInput {
                    sku: "CER-VASE-01".into(),
                    qty: 2,
                }],
            })
            .expect("create order");

        assert_eq!(order.subtotal_cents, 13600);
        assert_eq!(order.discount_cents, 1360);
        assert_eq!(order.shipping_cents, 0);
        assert_eq!(order.total_cents, 12240);
        assert_eq!(order.status, "NEW");

        let paid = store.mark_paid(&order.order_no).expect("mark paid");
        assert_eq!(paid.status, "PAID");

        let shipped = store
            .ship_order(&order.order_no, "Post.at", "AT123456789")
            .expect("ship");
        assert_eq!(shipped.status, "SHIPPED");
        assert_eq!(shipped.tracking_no.as_deref(), Some("AT123456789"));

        let products = store.list_products(Some("Ceramics"), true).expect("list");
        let vase = products
            .iter()
            .find(|p| p.sku == "CER-VASE-01")
            .expect("vase");
        assert_eq!(vase.stock_qty, 16);
    }

    #[test]
    fn rejects_oversell() {
        let mut store = Store::open_in_memory().expect("in-memory store");
        store.seed_demo().expect("seed");
        let res = store.create_order(&CreateOrderInput {
            customer_name: "Test".into(),
            customer_email: "t@example.com".into(),
            shipping_address: "Test 1".into(),
            zone_code: "AT".into(),
            promo_code: None,
            items: vec![OrderItemInput {
                sku: "LGT-LAMP-02".into(),
                qty: 99,
            }],
        });
        assert!(res.is_err());
    }
}
