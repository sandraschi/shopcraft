use std::path::Path;

use craft_biz::mcp::{normalize_tool_name, ToolHost};
use serde_json::{json, Value};

use crate::modes::amazon::AmazonHost;
use crate::modes::digital::DigitalShopHost;
use crate::modes::rental::RentalShopHost;
use crate::modes::subscription::SubShopHost;
use crate::modes::ticket::TicketShopHost;
use crate::modes::web::WebshopHost;
use crate::modes::wholesale::WholesaleHost;
use crate::store::PosStore;
use crate::ShopHost;

pub struct ShopSuite {
    pub retail: ShopHost,
    pub web: WebshopHost,
    pub amazon: AmazonHost,
    pub subscription: SubShopHost,
    pub digital: DigitalShopHost,
    pub wholesale: WholesaleHost,
    pub ticket: TicketShopHost,
    pub rental: RentalShopHost,
}

impl ShopSuite {
    pub fn open(root: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let db = |name: &str| root.join(format!("{name}.db"));
        let folder = |name: &str| root.join(name);
        Ok(Self {
            retail: ShopHost::new(PosStore::open(db("retail")).map_err(|e| e.to_string())?),
            web: WebshopHost::new(
                crate::modes::web::Store::open(db("web")).map_err(|e| e.to_string())?,
                folder("web-storefront"),
            ),
            amazon: AmazonHost::new(
                crate::modes::amazon::AmazonStore::open(db("amazon")).map_err(|e| e.to_string())?,
            ),
            subscription: SubShopHost::new(
                crate::modes::subscription::Store::open(&db("subscription"))
                    .map_err(|e| e.to_string())?,
                folder("subscription-storefront"),
            ),
            digital: DigitalShopHost::new(
                crate::modes::digital::Store::open(&db("digital")).map_err(|e| e.to_string())?,
                folder("digital-storefront"),
            ),
            wholesale: WholesaleHost::new(
                crate::modes::wholesale::Store::open(&db("wholesale"))
                    .map_err(|e| e.to_string())?,
                folder("wholesale-portal"),
            ),
            ticket: TicketShopHost::new(
                crate::modes::ticket::Store::open(&db("ticket")).map_err(|e| e.to_string())?,
                folder("ticket-storefront"),
            ),
            rental: RentalShopHost::new(
                crate::modes::rental::Store::open(&db("rental")).map_err(|e| e.to_string())?,
                folder("rental-storefront"),
            ),
        })
    }

    pub fn open_in_memory() -> Result<Self, String> {
        let scratch = std::env::temp_dir().join("shopcraft-suite-scratch");
        Ok(Self {
            retail: ShopHost::new(PosStore::open_in_memory().map_err(|e| e.to_string())?),
            web: WebshopHost::new(
                crate::modes::web::Store::open_in_memory().map_err(|e| e.to_string())?,
                scratch.join("web"),
            ),
            amazon: AmazonHost::new(
                crate::modes::amazon::AmazonStore::open_in_memory().map_err(|e| e.to_string())?,
            ),
            subscription: SubShopHost::new(
                crate::modes::subscription::Store::open_in_memory().map_err(|e| e.to_string())?,
                scratch.join("subscription"),
            ),
            digital: DigitalShopHost::new(
                crate::modes::digital::Store::open_in_memory().map_err(|e| e.to_string())?,
                scratch.join("digital"),
            ),
            wholesale: WholesaleHost::new(
                crate::modes::wholesale::Store::open_in_memory().map_err(|e| e.to_string())?,
                scratch.join("wholesale"),
            ),
            ticket: TicketShopHost::new(
                crate::modes::ticket::Store::open_in_memory().map_err(|e| e.to_string())?,
                scratch.join("ticket"),
            ),
            rental: RentalShopHost::new(
                crate::modes::rental::Store::open_in_memory().map_err(|e| e.to_string())?,
                scratch.join("rental"),
            ),
        })
    }

    pub fn status_tool(mode: &str) -> Result<&'static str, String> {
        match mode {
            "retail" => Ok("shop_status"),
            "web" => Ok("webshop_status"),
            "amazon" => Ok("amazon_status"),
            "subscription" => Ok("sub_status"),
            "digital" => Ok("digital_status"),
            "wholesale" => Ok("wholesale_status"),
            "ticket" => Ok("ticket_status"),
            "rental" => Ok("rental_status"),
            other => Err(format!("unknown mode '{other}'")),
        }
    }

    pub fn seed_tool(mode: &str) -> Result<&'static str, String> {
        match mode {
            "retail" => Ok("shop_seed"),
            "web" => Ok("webshop_seed"),
            "amazon" => Ok("amazon_seed"),
            "subscription" => Ok("sub_seed"),
            "digital" => Ok("digital_seed"),
            "wholesale" => Ok("wholesale_seed"),
            "ticket" => Ok("ticket_seed"),
            "rental" => Ok("rental_seed"),
            other => Err(format!("unknown mode '{other}'")),
        }
    }

    pub fn tool_matches_mode(mode: &str, tool_name: &str) -> bool {
        match mode {
            "retail" => tool_name.starts_with("shop_") || tool_name.starts_with("retail_"),
            "web" => tool_name.starts_with("webshop_"),
            "amazon" => tool_name.starts_with("amazon_"),
            "subscription" => tool_name.starts_with("sub_"),
            "digital" => tool_name.starts_with("digital_"),
            "wholesale" => tool_name.starts_with("wholesale_"),
            "ticket" => tool_name.starts_with("ticket_"),
            "rental" => tool_name.starts_with("rental_"),
            _ => false,
        }
    }
}

impl ToolHost for ShopSuite {
    fn server_name(&self) -> &str {
        "shopcraft"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<Value> {
        let mut out = Vec::new();
        for tool in self.retail.tools() {
            if let Some(name) = tool.get("name").and_then(Value::as_str) {
                if let Some(rest) = name.strip_prefix("shop_") {
                    let mut alias = tool.clone();
                    alias["name"] = json!(format!("retail_{rest}"));
                    out.push(alias);
                }
            }
            out.push(tool);
        }
        out.extend(self.web.tools());
        out.extend(self.amazon.tools());
        out.extend(self.subscription.tools());
        out.extend(self.digital.tools());
        out.extend(self.wholesale.tools());
        out.extend(self.ticket.tools());
        out.extend(self.rental.tools());
        out
    }

    fn call(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        let normalized = normalize_tool_name(name);
        let routed = if let Some(rest) = normalized.strip_prefix("retail_") {
            format!("shop_{rest}")
        } else {
            normalized
        };
        if routed.starts_with("shop_") {
            return self.retail.call(&routed, args);
        }
        if routed.starts_with("webshop_") {
            return self.web.call(&routed, args);
        }
        if routed.starts_with("amazon_") {
            return self.amazon.call(&routed, args);
        }
        if routed.starts_with("sub_") {
            return self.subscription.call(&routed, args);
        }
        if routed.starts_with("digital_") {
            return self.digital.call(&routed, args);
        }
        if routed.starts_with("wholesale_") {
            return self.wholesale.call(&routed, args);
        }
        if routed.starts_with("ticket_") {
            return self.ticket.call(&routed, args);
        }
        if routed.starts_with("rental_") {
            return self.rental.call(&routed, args);
        }
        Err(format!("unknown tool: {routed}"))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_status_is_callable() {
        let mut suite = ShopSuite::open_in_memory().expect("suite");
        let tools = [
            "shop_status",
            "retail_status",
            "webshop_status",
            "amazon_status",
            "sub_status",
            "digital_status",
            "wholesale_status",
            "ticket_status",
            "rental_status",
        ];
        for tool in tools {
            suite.call(tool, &json!({})).expect(tool);
        }
        let names: Vec<String> = suite
            .tools()
            .iter()
            .filter_map(|t| t.get("name").and_then(Value::as_str).map(str::to_string))
            .collect();
        assert!(names.iter().any(|n| n == "retail_seed"));
        assert!(names.iter().any(|n| n == "webshop_order_create"));
        assert!(names.iter().any(|n| n == "wholesale_quote"));
        assert!(names.iter().any(|n| n == "rental_book"));
    }
}
