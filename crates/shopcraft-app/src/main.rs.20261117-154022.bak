#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use craft_biz::bus::Bus;
use craft_biz::mcp::ToolHost;
use craft_biz::Money;
use craft_biz_ui::app::{main_with, Frame, NavEntry, SuiteApp, Wiring};
use craft_biz_ui::theme::Tone;
use craft_biz_ui::widgets::{self, Kpi};
use egui::{Grid, RichText, ScrollArea, Ui};
use serde_json::{json, Value};
use shopcraft_core::models::{PosItem, PosStatus, Shift};
use shopcraft_core::suite::ShopSuite;

struct ShopApp {
    suite: ShopSuite,
    items: Vec<PosItem>,
    status: Option<PosStatus>,
    active_shift: Option<Shift>,
    last_receipt_text: String,
    mode_status: String,
    loaded_view: usize,
}

struct ModePage {
    title: &'static str,
    status_tool: &'static str,
    seed_tool: &'static str,
}

const PAGES: &[ModePage] = &[
    ModePage {
        title: "Retail POS",
        status_tool: "shop_status",
        seed_tool: "shop_seed",
    },
    ModePage {
        title: "Web",
        status_tool: "webshop_status",
        seed_tool: "webshop_seed",
    },
    ModePage {
        title: "Amazon",
        status_tool: "amazon_status",
        seed_tool: "amazon_seed",
    },
    ModePage {
        title: "Subscription",
        status_tool: "sub_status",
        seed_tool: "sub_seed",
    },
    ModePage {
        title: "Digital",
        status_tool: "digital_status",
        seed_tool: "digital_seed",
    },
    ModePage {
        title: "Wholesale",
        status_tool: "wholesale_status",
        seed_tool: "wholesale_seed",
    },
    ModePage {
        title: "Ticket",
        status_tool: "ticket_status",
        seed_tool: "ticket_seed",
    },
    ModePage {
        title: "Rental",
        status_tool: "rental_status",
        seed_tool: "rental_seed",
    },
];

impl ShopApp {
    fn open(path: &Path) -> Result<Self, String> {
        let suite = ShopSuite::open(path)?;
        let mut app = Self {
            suite,
            items: Vec::new(),
            status: None,
            active_shift: None,
            last_receipt_text: String::new(),
            mode_status: String::new(),
            loaded_view: usize::MAX,
        };
        app.reload()?;
        Ok(app)
    }

    fn refresh_mode(&mut self, view: usize) {
        let Some(page) = PAGES.get(view) else {
            return;
        };
        self.mode_status = match self.suite.call(page.status_tool, &json!({})) {
            Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
            Err(err) => err,
        };
    }

    fn view_register(&mut self, ui: &mut Ui, f: &mut Frame) {
        if let Some(st) = &self.status {
            widgets::kpi_row(
                ui,
                &[
                    Kpi::new("Active Items", st.active_items.to_string()),
                    Kpi::new(
                        "Open Shift",
                        st.open_shift.clone().unwrap_or_else(|| "None".into()),
                    ),
                    Kpi::new("Receipts", st.total_receipts.to_string()),
                    Kpi::new(
                        "Gross Sales",
                        Money::from_cents(st.gross_sales_cents).to_string(),
                    ),
                ],
            );
            ui.add_space(10.0);
        }

        ui.horizontal(|ui| {
            if self.active_shift.is_none() {
                if ui.button("Open Shift (Lukas, EUR 100.00)").clicked() {
                    match self.suite.call(
                        "shop_shift_open",
                        &json!({ "clerk_name": "Lukas", "opening_float_cents": 10000 }),
                    ) {
                        Ok(_) => {
                            let _ = self.reload();
                            f.toast("Shift opened", Tone::Teal);
                        }
                        Err(e) => f.toast(e, Tone::Danger),
                    }
                }
            } else if let Some(sh) = self.active_shift.clone() {
                if ui.button("Close Shift & Print Z-Bon").clicked() {
                    match self.suite.call(
                        "shop_shift_close",
                        &json!({ "shift_id": sh.shift_id, "counted_cash_cents": sh.expected_cash_cents }),
                    ) {
                        Ok(_) => {
                            let _ = self.reload();
                            f.toast("Shift closed + Z-Bon emitted to buchcraft", Tone::Teal);
                        }
                        Err(e) => f.toast(e, Tone::Danger),
                    }
                }
            }
        });

        ui.add_space(8.0);
        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("pos_items_grid").striped(true).show(ui, |ui| {
                ui.strong("SKU");
                ui.strong("EAN-13");
                ui.strong("Title");
                ui.strong("Price");
                ui.strong("Stock");
                ui.strong("Quick Sale (Card)");
                ui.end_row();

                let items = self.items.clone();
                for it in items {
                    ui.label(&it.sku);
                    ui.label(&it.barcode);
                    ui.label(&it.title);
                    ui.label(Money::from_cents(it.price_cents).to_string());
                    ui.label(it.stock_qty.to_string());
                    let can_sell = self.active_shift.is_some() && it.stock_qty > 0;
                    if ui
                        .add_enabled(can_sell, egui::Button::new("Sell 1x"))
                        .clicked()
                    {
                        if let Some(sh) = &self.active_shift {
                            match self.suite.call(
                                "shop_checkout",
                                &json!({
                                    "shift_id": sh.shift_id,
                                    "items": [{ "code": it.sku, "qty": 1 }],
                                    "cash_cents": 0,
                                    "card_cents": it.price_cents,
                                    "voucher_cents": 0
                                }),
                            ) {
                                Ok(val) => {
                                    if let Some(rno) = val.get("receipt_no").and_then(Value::as_str)
                                    {
                                        self.last_receipt_text = self
                                            .suite
                                            .retail
                                            .store
                                            .format_receipt_text(rno)
                                            .unwrap_or_default();
                                    }
                                    let _ = self.reload();
                                    f.toast("Sale completed + bus emitted", Tone::Teal);
                                }
                                Err(e) => f.toast(e, Tone::Danger),
                            }
                        }
                    }
                    ui.end_row();
                }
            });
            ui.add_space(12.0);
            ui.label(RichText::new("42-Col ESC/POS Receipt Preview").strong());
            let text = if self.last_receipt_text.is_empty() {
                "Complete a sale to preview the thermal receipt."
            } else {
                self.last_receipt_text.as_str()
            };
            ui.monospace(text);
        });
    }

    fn view_mode(&mut self, ui: &mut Ui, f: &mut Frame, view: usize) {
        let Some(page) = PAGES.get(view) else {
            return;
        };
        ui.heading(page.title);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Refresh status").clicked() {
                self.refresh_mode(view);
            }
            if ui.button("Seed demo").clicked() {
                match self.suite.call(page.seed_tool, &json!({})) {
                    Ok(_) => {
                        self.refresh_mode(view);
                        f.toast(format!("{} seeded", page.title), Tone::Teal);
                    }
                    Err(err) => f.toast(err, Tone::Danger),
                }
            }
        });
        ui.add_space(8.0);
        ScrollArea::vertical().show(ui, |ui| {
            ui.monospace(&self.mode_status);
        });
    }
}

impl SuiteApp for ShopApp {
    fn name(&self) -> &'static str {
        "ShopCraft"
    }
    fn id(&self) -> &'static str {
        "shopcraft"
    }
    fn tone(&self) -> Tone {
        Tone::Warn
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn tools(&self) -> Vec<Value> {
        self.suite.tools()
    }
    fn call(&mut self, tool: &str, args: &Value) -> Result<Value, String> {
        let out = self.suite.call(tool, args)?;
        let _ = self.reload();
        if self.loaded_view != 0 && self.loaded_view != usize::MAX {
            self.refresh_mode(self.loaded_view);
        }
        Ok(out)
    }
    fn reload(&mut self) -> Result<(), String> {
        self.items = self
            .suite
            .retail
            .store
            .list_items(None)
            .map_err(|e| e.to_string())?;
        let st = self
            .suite
            .retail
            .store
            .status()
            .map_err(|e| e.to_string())?;
        self.active_shift = match &st.open_shift {
            Some(id) => self.suite.retail.store.get_shift(id).ok(),
            None => None,
        };
        self.status = Some(st);
        Ok(())
    }
    fn nav(&self, _f: &Frame) -> Vec<NavEntry> {
        vec![
            NavEntry::new("R", "Retail")
                .group("Modes")
                .badge(self.items.len(), Tone::Warn),
            NavEntry::new("W", "Web").group("Modes"),
            NavEntry::new("A", "Amazon").group("Modes"),
            NavEntry::new("S", "Subscription").group("Modes"),
            NavEntry::new("D", "Digital").group("Modes"),
            NavEntry::new("B", "Wholesale").group("Modes"),
            NavEntry::new("T", "Ticket").group("Modes"),
            NavEntry::new("N", "Rental").group("Modes"),
        ]
    }
    fn view(&mut self, ui: &mut Ui, view: usize, f: &mut Frame) {
        if view != self.loaded_view {
            self.loaded_view = view;
            if view != 0 {
                self.refresh_mode(view);
            }
        }
        if view == 0 {
            self.view_register(ui, f);
        } else {
            self.view_mode(ui, f, view);
        }
    }
    fn bus(&self) -> &Bus {
        &self.suite.retail.bus
    }
    fn wiring(&self) -> Wiring {
        (
            vec![("stock.available_changed", "Refresh local POS stock counts")],
            vec![
                ("channel.sale_committed", "Deduct GoodsOut in stockcraft"),
                ("channel.return_restocked", "Restock Storno in stockcraft"),
                ("pos.zbon_closed", "Post Z-Bon daily revenue to buchcraft"),
            ],
        )
    }
    fn smoke_tool(&self) -> &'static str {
        "shop_status"
    }
}

fn default_db() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("shopcraft")
}

fn main() -> ExitCode {
    main_with(default_db(), ShopApp::open, "shop_seed")
}
