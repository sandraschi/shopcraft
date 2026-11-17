use std::path::PathBuf;

use craft_biz::bus::Bus;
use craft_biz::mcp::{normalize_tool_name, tool, ToolHost};
use serde_json::{json, Value};

use super::error::Result;
use super::models::IssueTicketInput;
use super::store::Store;

pub struct TicketShopHost {
    pub store: Store,
    pub storefront_dir: PathBuf,
    pub bus: Bus,
}

impl TicketShopHost {
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

impl ToolHost for TicketShopHost {
    fn server_name(&self) -> &str {
        "ticketshopcraft"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<Value> {
        vec![
            tool(
                "ticket_status",
                "Return event count, total capacity, seats sold, checked-in seats, and gross revenue in cents.",
                json!({}),
                &[],
            ),
            tool("ticket_seed", "Seed 6 demo events & workshop tiers.", json!({}), &[]),
            tool("ticket_catalog", "List event tiers and remaining capacity.", json!({}), &[]),
            tool("ticket_orders", "List issued, checked-in, and refunded ticket orders.", json!({}), &[]),
            tool(
                "ticket_issue",
                "Issue tickets for an event tier if seats remain.",
                json!({
                    "attendee_name": { "type": "string" },
                    "attendee_email": { "type": "string" },
                    "code": { "type": "string" },
                    "quantity": { "type": "integer" }
                }),
                &["attendee_name", "attendee_email", "code", "quantity"],
            ),
            tool(
                "ticket_checkin",
                "Check in an issued ticket by ticket_code.",
                json!({
                    "ticket_code": { "type": "string" }
                }),
                &["ticket_code"],
            ),
            tool(
                "ticket_refund",
                "Refund an issued ticket and release seats back to capacity.",
                json!({
                    "ticket_code": { "type": "string" }
                }),
                &["ticket_code"],
            ),
            tool(
                "ticket_export_storefront",
                "Write storefront/catalog.json from the current SQLite database.",
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
            "ticket_status" => {
                let st = self.store.status().map_err(|e| e.to_string())?;
                serde_json::to_value(st).map_err(|e| e.to_string())
            }
            "ticket_seed" => {
                let inserted = self.store.seed_demo().map_err(|e| e.to_string())?;
                Ok(json!({ "events_inserted": inserted }))
            }
            "ticket_catalog" => {
                let items = self.store.list_events().map_err(|e| e.to_string())?;
                serde_json::to_value(items).map_err(|e| e.to_string())
            }
            "ticket_orders" => {
                let orders = self.store.list_orders().map_err(|e| e.to_string())?;
                serde_json::to_value(orders).map_err(|e| e.to_string())
            }
            "ticket_issue" => {
                let input: IssueTicketInput =
                    serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
                let order = self.store.issue_ticket(&input).map_err(|e| e.to_string())?;
                let _ = self.bus.emit(
                    "ticketshopcraft",
                    "billcraft",
                    "ticket.issued",
                    json!({
                        "ticket_code": order.ticket_code,
                        "code": order.code,
                        "quantity": order.quantity,
                        "total_cents": order.total_cents
                    }),
                );
                serde_json::to_value(order).map_err(|e| e.to_string())
            }
            "ticket_checkin" => {
                let ticket_code = args
                    .get("ticket_code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "missing ticket_code".to_string())?;
                let order = self
                    .store
                    .check_in(ticket_code)
                    .map_err(|e| e.to_string())?;
                let _ = self.bus.emit(
                    "ticketshopcraft",
                    "billcraft",
                    "ticket.checked_in",
                    json!({
                        "ticket_code": order.ticket_code,
                        "code": order.code
                    }),
                );
                serde_json::to_value(order).map_err(|e| e.to_string())
            }
            "ticket_refund" => {
                let ticket_code = args
                    .get("ticket_code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "missing ticket_code".to_string())?;
                let order = self
                    .store
                    .refund_ticket(ticket_code)
                    .map_err(|e| e.to_string())?;
                let _ = self.bus.emit(
                    "ticketshopcraft",
                    "billcraft",
                    "ticket.refunded",
                    json!({
                        "ticket_code": order.ticket_code,
                        "code": order.code
                    }),
                );
                serde_json::to_value(order).map_err(|e| e.to_string())
            }
            "ticket_export_storefront" => {
                let store_name = args
                    .get("store_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("NEUBAU BOX OFFICE");
                let tagline = args
                    .get("tagline")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Local-First Event Ticketing, Workshops & Seat Reservations");
                let path = self
                    .export_storefront_to_disk(store_name, tagline)
                    .map_err(|e| e.to_string())?;
                Ok(json!({ "exported_path": path.display().to_string() }))
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}
