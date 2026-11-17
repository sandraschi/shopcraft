use std::path::PathBuf;

use craft_biz::bus::Bus;
use craft_biz::mcp::{normalize_tool_name, tool, ToolHost};
use serde_json::{json, Value};

use super::error::Result;
use super::models::CreateBookingInput;
use super::store::Store;

pub struct RentalShopHost {
    pub store: Store,
    pub storefront_dir: PathBuf,
    pub bus: Bus,
}

impl RentalShopHost {
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

impl ToolHost for RentalShopHost {
    fn server_name(&self) -> &str {
        "rentalshopcraft"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<Value> {
        vec![
            tool(
                "rental_status",
                "Return fleet SKUs, total units, active bookings, checked-out units, deposits held, and revenue in cents.",
                json!({}),
                &[],
            ),
            tool("rental_seed", "Seed 6 demo rental gear SKUs.", json!({}), &[]),
            tool("rental_catalog", "List active rental gear.", json!({}), &[]),
            tool("rental_bookings", "List rental bookings.", json!({}), &[]),
            tool(
                "rental_book",
                "Reserve a rental unit for a date range, checking overlap capacity and holding deposit.",
                json!({
                    "customer_name": { "type": "string" },
                    "customer_email": { "type": "string" },
                    "sku": { "type": "string" },
                    "start_date": { "type": "string" },
                    "end_date": { "type": "string" }
                }),
                &["customer_name", "customer_email", "sku", "start_date", "end_date"],
            ),
            tool(
                "rental_checkout",
                "Check out a reserved booking to the customer.",
                json!({ "booking_number": { "type": "string" } }),
                &["booking_number"],
            ),
            tool(
                "rental_checkin",
                "Check in a returned unit, compute late/damage fees and deposit refund, and emit Domino Bus events.",
                json!({
                    "booking_number": { "type": "string" },
                    "return_date": { "type": "string" },
                    "damage_fee_cents": { "type": "integer" }
                }),
                &["booking_number", "return_date"],
            ),
            tool(
                "rental_export_storefront",
                "Export storefront/catalog.json for the rental storefront.",
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
            "rental_status" => self
                .store
                .status()
                .and_then(|s| serde_json::to_value(s).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "rental_seed" => {
                let n = self.store.seed_demo().map_err(|e| e.to_string())?;
                Ok(json!({ "items_inserted": n }))
            }
            "rental_catalog" => self
                .store
                .list_items()
                .and_then(|i| serde_json::to_value(i).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "rental_bookings" => self
                .store
                .list_bookings()
                .and_then(|b| serde_json::to_value(b).map_err(Into::into))
                .map_err(|e| e.to_string()),
            "rental_book" => {
                let input: CreateBookingInput =
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
                let booking = self.store.book(&input).map_err(|e| e.to_string())?;
                let payload = json!({
                    "booking_number": booking.booking_number,
                    "sku": booking.sku,
                    "rental_fee_cents": booking.rental_fee_cents,
                    "deposit_cents": booking.deposit_cents,
                });
                let _ = self
                    .bus
                    .emit("rentalshopcraft", "billcraft", "rental.booked", payload);
                serde_json::to_value(booking).map_err(|e| e.to_string())
            }
            "rental_checkout" => {
                let num = args
                    .get("booking_number")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing booking_number".to_string())?;
                self.store
                    .checkout(num)
                    .and_then(|b| serde_json::to_value(b).map_err(Into::into))
                    .map_err(|e| e.to_string())
            }
            "rental_checkin" => {
                let num = args
                    .get("booking_number")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing booking_number".to_string())?;
                let ret_date = args
                    .get("return_date")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing return_date".to_string())?;
                let dmg = args
                    .get("damage_fee_cents")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                let booking = self
                    .store
                    .checkin(num, ret_date, dmg)
                    .map_err(|e| e.to_string())?;
                let payload = json!({
                    "booking_number": booking.booking_number,
                    "total_fee_cents": booking.rental_fee_cents + booking.late_fee_cents + booking.damage_fee_cents,
                    "deposit_refund_cents": booking.deposit_refund_cents,
                });
                let _ = self.bus.emit(
                    "rentalshopcraft",
                    "billcraft",
                    "order.paid",
                    payload.clone(),
                );
                let _ = self
                    .bus
                    .emit("rentalshopcraft", "buchcraft", "rental.settled", payload);
                serde_json::to_value(booking).map_err(|e| e.to_string())
            }
            "rental_export_storefront" => {
                let store_name = args
                    .get("store_name")
                    .and_then(Value::as_str)
                    .unwrap_or("ALPINE GEAR RENTAL");
                let tagline = args
                    .get("tagline")
                    .and_then(Value::as_str)
                    .unwrap_or("Local-First Equipment, Bike & Studio Gear Rental");
                let out = self
                    .export_storefront_to_disk(store_name, tagline)
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "exported_path": out.display().to_string() }))
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}
