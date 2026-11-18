//! Storefront desks. Each page lists what is in that mode's database and
//! runs the same store calls the CLI and MCP tools use.

use craft_biz::Money;
use craft_biz_ui::app::{Frame, SuiteApp};
use craft_biz_ui::theme::Tone;
use craft_biz_ui::widgets::{self, Kpi};
use egui::{Grid, RichText, ScrollArea, Ui};
use shopcraft_core::modes::{amazon, digital, rental, subscription, ticket, web, wholesale};
use shopcraft_core::suite::ShopSuite;

use super::ShopApp;

const RENTAL_SLOTS: &[(&str, &str)] = &[
    ("2026-12-01", "2026-12-03"),
    ("2026-12-08", "2026-12-10"),
    ("2026-12-15", "2026-12-17"),
    ("2026-12-22", "2026-12-24"),
    ("2027-01-05", "2027-01-07"),
    ("2027-01-12", "2027-01-14"),
    ("2027-01-19", "2027-01-21"),
    ("2027-01-26", "2027-01-28"),
];

pub struct Desk {
    pub web_status: Option<web::ShopStatus>,
    pub web_products: Vec<web::Product>,
    pub web_orders: Vec<web::Order>,
    pub web_zone: String,
    pub amazon_status: Option<amazon::AmazonStatus>,
    pub amazon_listings: Vec<amazon::Listing>,
    pub amazon_orders: Vec<amazon::AmazonOrder>,
    pub amazon_alerts: Vec<amazon::ReplenishmentAlert>,
    pub amazon_margin: Vec<(String, i64)>,
    pub sub_status: Option<subscription::SubStatus>,
    pub plans: Vec<subscription::SubscriptionPlan>,
    pub subs: Vec<subscription::Subscription>,
    pub digital_status: Option<digital::DigitalStatus>,
    pub digital_products: Vec<digital::DigitalProduct>,
    pub digital_orders: Vec<digital::DigitalOrder>,
    pub digital_licenses: Vec<digital::IssuedLicense>,
    pub wholesale_status: Option<wholesale::WholesaleStatus>,
    pub accounts: Vec<wholesale::TradeAccount>,
    pub ws_products: Vec<wholesale::WholesaleProduct>,
    pub ws_orders: Vec<wholesale::PurchaseOrder>,
    pub ticket_status: Option<ticket::TicketStatus>,
    pub events: Vec<ticket::EventTier>,
    pub tickets: Vec<ticket::TicketOrder>,
    pub rental_status: Option<rental::RentalStatus>,
    pub gear: Vec<rental::RentalItem>,
    pub bookings: Vec<rental::RentalBooking>,
}

impl Desk {
    pub fn empty() -> Self {
        Self {
            web_status: None,
            web_products: Vec::new(),
            web_orders: Vec::new(),
            web_zone: "AT".into(),
            amazon_status: None,
            amazon_listings: Vec::new(),
            amazon_orders: Vec::new(),
            amazon_alerts: Vec::new(),
            amazon_margin: Vec::new(),
            sub_status: None,
            plans: Vec::new(),
            subs: Vec::new(),
            digital_status: None,
            digital_products: Vec::new(),
            digital_orders: Vec::new(),
            digital_licenses: Vec::new(),
            wholesale_status: None,
            accounts: Vec::new(),
            ws_products: Vec::new(),
            ws_orders: Vec::new(),
            ticket_status: None,
            events: Vec::new(),
            tickets: Vec::new(),
            rental_status: None,
            gear: Vec::new(),
            bookings: Vec::new(),
        }
    }

    pub fn load(suite: &ShopSuite) -> Result<Self, String> {
        let web_products = suite
            .web
            .store
            .list_products(None, false)
            .map_err(|e| e.to_string())?;
        let zones = suite
            .web
            .store
            .list_shipping_zones()
            .map_err(|e| e.to_string())?;
        let listings = suite
            .amazon
            .store
            .list_listings(None)
            .map_err(|e| e.to_string())?;
        let mut amazon_margin = Vec::new();
        for listing in &listings {
            if let Ok(econ) = suite.amazon.store.unit_economics(&listing.asin) {
                amazon_margin.push((listing.asin.clone(), econ.net_profit_cents));
            }
        }
        Ok(Self {
            web_status: Some(suite.web.store.status().map_err(|e| e.to_string())?),
            web_products,
            web_orders: suite
                .web
                .store
                .list_orders(None)
                .map_err(|e| e.to_string())?,
            web_zone: zones
                .first()
                .map(|z| z.zone_code.clone())
                .unwrap_or_else(|| "AT".into()),
            amazon_status: Some(suite.amazon.store.status().map_err(|e| e.to_string())?),
            amazon_listings: listings,
            amazon_orders: suite
                .amazon
                .store
                .list_orders()
                .map_err(|e| e.to_string())?,
            amazon_alerts: suite
                .amazon
                .store
                .replenishment_alerts(14)
                .map_err(|e| e.to_string())?,
            amazon_margin,
            sub_status: Some(
                suite
                    .subscription
                    .store
                    .status()
                    .map_err(|e| e.to_string())?,
            ),
            plans: suite
                .subscription
                .store
                .list_plans()
                .map_err(|e| e.to_string())?,
            subs: suite
                .subscription
                .store
                .list_subscriptions()
                .map_err(|e| e.to_string())?,
            digital_status: Some(suite.digital.store.status().map_err(|e| e.to_string())?),
            digital_products: suite
                .digital
                .store
                .list_products()
                .map_err(|e| e.to_string())?,
            digital_orders: suite
                .digital
                .store
                .list_orders()
                .map_err(|e| e.to_string())?,
            digital_licenses: suite
                .digital
                .store
                .list_licenses()
                .map_err(|e| e.to_string())?,
            wholesale_status: Some(suite.wholesale.store.status().map_err(|e| e.to_string())?),
            accounts: suite
                .wholesale
                .store
                .list_accounts()
                .map_err(|e| e.to_string())?,
            ws_products: suite
                .wholesale
                .store
                .list_products(None)
                .map_err(|e| e.to_string())?,
            ws_orders: suite
                .wholesale
                .store
                .list_orders()
                .map_err(|e| e.to_string())?,
            ticket_status: Some(suite.ticket.store.status().map_err(|e| e.to_string())?),
            events: suite
                .ticket
                .store
                .list_events()
                .map_err(|e| e.to_string())?,
            tickets: suite
                .ticket
                .store
                .list_orders()
                .map_err(|e| e.to_string())?,
            rental_status: Some(suite.rental.store.status().map_err(|e| e.to_string())?),
            gear: suite.rental.store.list_items().map_err(|e| e.to_string())?,
            bookings: suite
                .rental
                .store
                .list_bookings()
                .map_err(|e| e.to_string())?,
        })
    }
}

fn eur(cents: i64) -> String {
    Money::from_cents(cents).to_string()
}

fn kpis(ui: &mut Ui, items: &[Kpi]) {
    widgets::kpi_row(ui, items);
    ui.add_space(8.0);
}

fn block_title(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).strong());
    ui.add_space(4.0);
}

fn notice(ui: &mut Ui, text: &str) {
    if !text.is_empty() {
        ui.label(RichText::new(text).monospace());
        ui.add_space(6.0);
    }
}

fn finish(app: &mut ShopApp, f: &mut Frame, result: Result<String, String>) {
    match result {
        Ok(msg) => {
            app.notice = msg.clone();
            match app.reload() {
                Ok(()) => f.toast(msg, Tone::Teal),
                Err(err) => f.toast(err, Tone::Danger),
            }
        }
        Err(err) => {
            app.notice = err.clone();
            f.toast(err, Tone::Danger);
        }
    }
}

fn seed_button(ui: &mut Ui, label: &str) -> bool {
    ui.button(label).clicked()
}

pub fn web(app: &mut ShopApp, ui: &mut Ui, f: &mut Frame, section: usize) {
    ui.heading(match section {
        0 => f.t("Web Kennzahlen", "Web stats"),
        1 => f.t("Web Katalog", "Web catalog"),
        2 => f.t("Web Bestellungen", "Web orders"),
        _ => f.t("Web Versand", "Web shipping"),
    });
    ui.add_space(6.0);
    if section == 0 {
        if let Some(st) = &app.desk.web_status {
            kpis(
                ui,
                &[
                    Kpi::new(f.t("Artikel", "Products"), st.active_products.to_string()),
                    Kpi::new(f.t("Bestand", "Stock"), st.total_stock_units.to_string()),
                    Kpi::new(
                        f.t("Offene Aufträge", "Open orders"),
                        st.open_orders.to_string(),
                    ),
                    Kpi::new(f.t("Umsatz", "Revenue"), eur(st.total_revenue_cents)),
                ],
            );
        }
        ui.label(f.t(
            "Diese Zahlen gehören zu diesem Webshop.",
            "Counts are this web shop's own database.",
        ));
    }
    let (seed, open_shop) = ui
        .horizontal(|ui| {
            (
                seed_button(ui, f.t("Demo-Katalog laden", "Seed demo catalog")),
                seed_button(ui, f.t("Webshop öffnen", "Open webshop")),
            )
        })
        .inner;
    notice(ui, &app.notice.clone());
    if open_shop {
        app.open_webshop(f);
    }
    if seed {
        let result = app
            .suite
            .web
            .store
            .seed_demo()
            .map(|n| format!("Web catalog seeded ({n} new rows, existing SKUs kept)"))
            .map_err(|e| e.to_string());
        finish(app, f, result);
        return;
    }
    if section == 0 {
        return;
    }
    if section == 3 {
        match app.suite.web.store.list_shipping_zones() {
            Ok(zones) => {
                if zones.is_empty() {
                    ui.label(f.t(
                        "Noch keine Versandzonen. Katalog laden.",
                        "No shipping zones yet. Seed the catalog.",
                    ));
                }
                Grid::new("web_zones").striped(true).show(ui, |ui| {
                    ui.strong(f.t("Zone", "Zone"));
                    ui.strong(f.t("Bezeichnung", "Label"));
                    ui.strong(f.t("Satz", "Rate"));
                    ui.strong(f.t("Frei ab", "Free over"));
                    ui.strong(f.t("Lieferzeit", "ETA"));
                    ui.end_row();
                    for zone in &zones {
                        ui.label(&zone.zone_code);
                        ui.label(&zone.label);
                        ui.label(eur(zone.base_rate_cents));
                        ui.label(eur(zone.free_over_cents));
                        ui.label(&zone.eta_days);
                        ui.end_row();
                    }
                });
            }
            Err(err) => {
                ui.label(err.to_string());
            }
        }
        return;
    }

    let products = app.desk.web_products.clone();
    let orders = app.desk.web_orders.clone();
    let zone = app.desk.web_zone.clone();
    ScrollArea::vertical().id_salt("web_desk").show(ui, |ui| {
        if section == 1 {
            block_title(ui, f.t("Katalog", "Catalog"));
            if products.is_empty() {
                ui.label(f.t(
                    "Noch keine Artikel. Demo-Katalog laden.",
                    "No products yet. Seed the demo catalog.",
                ));
            }
            Grid::new("web_products").striped(true).show(ui, |ui| {
                ui.strong("SKU");
                ui.strong(f.t("Titel", "Title"));
                ui.strong(f.t("Preis", "Price"));
                ui.strong(f.t("Bestand", "Stock"));
                ui.strong(f.t("Verkauf", "Sell"));
                ui.end_row();
                for product in &products {
                    ui.label(&product.sku);
                    ui.label(&product.title);
                    ui.label(eur(product.price_cents));
                    ui.label(product.stock_qty.to_string());
                    let can = product.active && product.stock_qty > 0;
                    if ui.add_enabled(can, egui::Button::new("Order 1")).clicked() {
                        let result = app
                            .suite
                            .web
                            .store
                            .create_order(&web::CreateOrderInput {
                                customer_name: "Nora Berg".into(),
                                customer_email: "nora@atelier.example".into(),
                                shipping_address: "Neubaugasse 12, 1070 Wien".into(),
                                zone_code: zone.clone(),
                                promo_code: None,
                                items: vec![web::OrderItemInput {
                                    sku: product.sku.clone(),
                                    qty: 1,
                                }],
                            })
                            .map(|order| {
                                format!(
                                    "Web order {} for {} total {}",
                                    order.order_no,
                                    order.customer_name,
                                    eur(order.total_cents)
                                )
                            })
                            .map_err(|e| e.to_string());
                        finish(app, f, result);
                    }
                    ui.end_row();
                }
            });
        }

        if section == 2 {
            block_title(ui, f.t("Bestellungen", "Orders"));
            if orders.is_empty() {
                ui.label(f.t("Noch keine Web-Bestellungen.", "No web orders yet."));
            }
            Grid::new("web_orders").striped(true).show(ui, |ui| {
                ui.strong(f.t("Auftrag", "Order"));
                ui.strong(f.t("Kunde", "Customer"));
                ui.strong(f.t("Status", "Status"));
                ui.strong(f.t("Summe", "Total"));
                ui.strong(f.t("Aktion", "Action"));
                ui.end_row();
                for order in &orders {
                    ui.label(&order.order_no);
                    ui.label(&order.customer_name);
                    ui.label(&order.status);
                    ui.label(eur(order.total_cents));
                    ui.horizontal(|ui| {
                        if order.status == "NEW" && ui.button(f.t("Bezahlt", "Mark paid")).clicked()
                        {
                            let result = app
                                .suite
                                .web
                                .store
                                .mark_paid(&order.order_no)
                                .map(|o| format!("{} is {}", o.order_no, o.status))
                                .map_err(|e| e.to_string());
                            finish(app, f, result);
                        }
                        if (order.status == "PAID" || order.status == "PICKING")
                            && ui.button(f.t("Versenden", "Ship")).clicked()
                        {
                            let result = app
                                .suite
                                .web
                                .store
                                .ship_order(&order.order_no, "Post AT", "AT-DEMO-1")
                                .map(|o| {
                                    format!(
                                        "{} shipped {} {}",
                                        o.order_no,
                                        o.carrier.unwrap_or_default(),
                                        o.tracking_no.unwrap_or_default()
                                    )
                                })
                                .map_err(|e| e.to_string());
                            finish(app, f, result);
                        }
                    });
                    ui.end_row();
                }
            });
        }
    });
}

pub fn amazon(app: &mut ShopApp, ui: &mut Ui, f: &mut Frame, section: usize) {
    ui.heading(match section {
        0 => f.t("Amazon Kennzahlen", "Amazon stats"),
        1 => f.t("Amazon Angebote", "Amazon listings"),
        2 => f.t("Amazon Bestellungen", "Amazon orders"),
        _ => f.t("Amazon Auffüllen", "Amazon replenishment"),
    });
    ui.add_space(6.0);
    if section == 0 {
        if let Some(st) = &app.desk.amazon_status {
            kpis(
                ui,
                &[
                    Kpi::new("Listings", st.active_listings.to_string()),
                    Kpi::new("FBA units", st.fba_units.to_string()),
                    Kpi::new("FBM units", st.fbm_units.to_string()),
                    Kpi::new("Net profit", eur(st.total_net_profit_cents)),
                    Kpi::new("VAT invoices open", st.pending_vat_invoices.to_string()),
                ],
            );
            ui.label(f.t(
                "Der Preis wird nur auf den Wert geklemmt, den Sie angeben. Amazon wird nicht aufgerufen.",
                "Reprice clamps a price you supply. It does not call Amazon.",
            ));
        }
    }
    let seed = ui
        .horizontal(|ui| seed_button(ui, f.t("Demo-Angebote laden", "Seed demo listings")))
        .inner;
    notice(ui, &app.notice.clone());
    if seed {
        let result = app
            .suite
            .amazon
            .store
            .seed_demo()
            .map(|n| format!("Amazon listings seeded ({n} new rows)"))
            .map_err(|e| e.to_string());
        finish(app, f, result);
        return;
    }
    if section == 0 {
        return;
    }

    let listings = app.desk.amazon_listings.clone();
    let orders = app.desk.amazon_orders.clone();
    let alerts = app.desk.amazon_alerts.clone();
    let margin = app.desk.amazon_margin.clone();
    ScrollArea::vertical()
        .id_salt("amazon_desk")
        .show(ui, |ui| {
            if section == 1 {
                block_title(ui, f.t("Angebote", "Listings"));
                if listings.is_empty() {
                    ui.label(f.t(
                        "Noch keine Angebote. Demo laden.",
                        "No listings yet. Seed the demo listings.",
                    ));
                }
                Grid::new("amazon_listings").striped(true).show(ui, |ui| {
                    ui.strong("ASIN");
                    ui.strong(f.t("Titel", "Title"));
                    ui.strong(f.t("Preis", "Price"));
                    ui.strong("FBA");
                    ui.strong("FBM");
                    ui.strong(f.t("Gewinn / Stück", "Profit / unit"));
                    ui.strong(f.t("Buy Box", "Buy box"));
                    ui.strong(f.t("Aktion", "Action"));
                    ui.end_row();
                    for listing in &listings {
                        let profit = margin
                            .iter()
                            .find(|(asin, _)| asin == &listing.asin)
                            .map(|(_, cents)| eur(*cents))
                            .unwrap_or_else(|| "-".into());
                        ui.label(&listing.asin);
                        ui.label(&listing.title);
                        ui.label(eur(listing.price_cents));
                        ui.label(listing.stock_fba.to_string());
                        ui.label(listing.stock_fbm.to_string());
                        ui.label(profit);
                        ui.label(if listing.buy_box_won {
                            f.t("Gewonnen", "Won")
                        } else {
                            f.t("Verloren", "Lost")
                        });
                        ui.horizontal(|ui| {
                            if ui
                                .button(f.t("Unterbieten 1,00", "Undercut 1.00"))
                                .clicked()
                            {
                                let competitor = (listing.price_cents - 100).max(1);
                                let result = app
                                    .suite
                                    .amazon
                                    .store
                                    .reprice(&listing.asin, competitor)
                                    .map(|r| {
                                        format!(
                                            "{} {} -> {} (floor {} ceiling {})",
                                            r.asin,
                                            eur(r.old_price_cents),
                                            eur(r.new_price_cents),
                                            if r.clamped_to_floor { "yes" } else { "no" },
                                            if r.clamped_to_ceiling { "yes" } else { "no" }
                                        )
                                    })
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                            if ui.button(f.t("Verkauf buchen", "Record sale")).clicked() {
                                let id = format!("UI-{}-{}", listing.asin, orders.len() + 1);
                                let result = app
                                    .suite
                                    .amazon
                                    .store
                                    .record_order(&amazon::RecordOrderInput {
                                        amazon_order_id: id,
                                        asin: listing.asin.clone(),
                                        qty: 1,
                                        status: "UNSHIPPED".into(),
                                    })
                                    .map(|order| {
                                        format!(
                                            "Amazon {} {} profit {}",
                                            order.amazon_order_id,
                                            order.sku,
                                            eur(order.net_profit_cents)
                                        )
                                    })
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                        });
                        ui.end_row();
                    }
                });
            }

            if section == 3 {
                block_title(
                    ui,
                    f.t(
                        "Nachschub unter 14 Tagen Reichweite",
                        "Replenishment under 14 days of cover",
                    ),
                );
                if alerts.is_empty() {
                    ui.label(f.t(
                        "Keine FBA-Nachschubwarnung.",
                        "No FBA replenishment alerts.",
                    ));
                }
                Grid::new("amazon_alerts").striped(true).show(ui, |ui| {
                    ui.strong("SKU");
                    ui.strong(f.t("Titel", "Title"));
                    ui.strong(f.t("Tage Reichweite", "Days cover"));
                    ui.strong(f.t("Versandmenge", "Ship qty"));
                    ui.end_row();
                    for alert in &alerts {
                        ui.label(&alert.sku);
                        ui.label(&alert.title);
                        ui.label(alert.days_of_cover.to_string());
                        ui.label(alert.recommended_ship_qty.to_string());
                        ui.end_row();
                    }
                });
            }

            if section == 2 {
                block_title(ui, f.t("Erfasste Bestellungen", "Recorded orders"));
                if orders.is_empty() {
                    ui.label(f.t(
                        "Keine Amazon-Bestellungen erfasst.",
                        "No Amazon orders recorded.",
                    ));
                }
                Grid::new("amazon_orders").striped(true).show(ui, |ui| {
                    ui.strong(f.t("Auftrag", "Order"));
                    ui.strong("SKU");
                    ui.strong(f.t("Menge", "Qty"));
                    ui.strong(f.t("Status", "Status"));
                    ui.strong(f.t("Gewinn", "Profit"));
                    ui.strong(f.t("Rechnung", "Invoice"));
                    ui.end_row();
                    for order in &orders {
                        ui.label(&order.amazon_order_id);
                        ui.label(&order.sku);
                        ui.label(order.qty.to_string());
                        ui.label(&order.status);
                        ui.label(eur(order.net_profit_cents));
                        if order.invoice_uploaded {
                            ui.label(f.t("Hochgeladen", "Uploaded"));
                        } else if ui
                            .button(f.t("Rechnung markieren", "Mark invoice uploaded"))
                            .clicked()
                        {
                            let result = app
                                .suite
                                .amazon
                                .store
                                .mark_invoice_uploaded(&order.amazon_order_id)
                                .map(|()| format!("Invoice marked for {}", order.amazon_order_id))
                                .map_err(|e| e.to_string());
                            finish(app, f, result);
                        }
                        ui.end_row();
                    }
                });
            }
        });
}

pub fn subscription(app: &mut ShopApp, ui: &mut Ui, f: &mut Frame, section: usize) {
    ui.heading(match section {
        0 => f.t("Abo Kennzahlen", "Subscription stats"),
        1 => f.t("Pläne", "Plans"),
        _ => f.t("Abonnenten", "Subscribers"),
    });
    ui.add_space(6.0);
    if section == 0 {
        if let Some(st) = &app.desk.sub_status {
            kpis(
                ui,
                &[
                    Kpi::new("Plans", st.active_plans.to_string()),
                    Kpi::new(f.t("Aktiv", "Active"), st.active_subscriptions.to_string()),
                    Kpi::new("Paused", st.paused_subscriptions.to_string()),
                    Kpi::new("MRR", eur(st.mrr_cents)),
                ],
            );
            ui.label(f.t(
                "Eine Mandatsreferenz wird gespeichert. Es wird keine pain.008-Datei geschrieben.",
                "A mandate reference is stored. No pain.008 file is written.",
            ));
        }
    }
    let seed = ui
        .horizontal(|ui| seed_button(ui, f.t("Demo-Pläne laden", "Seed demo plans")))
        .inner;
    notice(ui, &app.notice.clone());
    if seed {
        let result = app
            .suite
            .subscription
            .store
            .seed_demo()
            .map(|n| format!("Subscription plans seeded ({n} new rows)"))
            .map_err(|e| e.to_string());
        finish(app, f, result);
        return;
    }
    if section == 0 {
        return;
    }
    let plans = app.desk.plans.clone();
    let subs = app.desk.subs.clone();
    ScrollArea::vertical().id_salt("sub_desk").show(ui, |ui| {
        if section == 1 {
            block_title(ui, f.t("Pläne", "Plans"));
            if plans.is_empty() {
                ui.label(f.t(
                    "Noch keine Pläne. Demo laden.",
                    "No plans yet. Seed the demo plans.",
                ));
            }
            Grid::new("sub_plans").striped(true).show(ui, |ui| {
                ui.strong("SKU");
                ui.strong(f.t("Plan", "Plan"));
                ui.strong(f.t("Alle", "Every"));
                ui.strong(f.t("Preis", "Price"));
                ui.strong(f.t("Abonnieren", "Subscribe"));
                ui.end_row();
                for plan in &plans {
                    ui.label(&plan.sku);
                    ui.label(&plan.name);
                    ui.label(format!("{} {}", plan.cadence_days, f.t("Tage", "days")));
                    ui.label(eur(plan.net_price_cents));
                    if ui
                        .add_enabled(plan.active, egui::Button::new("Subscribe Nora"))
                        .clicked()
                    {
                        let result = app
                            .suite
                            .subscription
                            .store
                            .subscribe(&subscription::SubscribeInput {
                                customer_name: "Nora Berg".into(),
                                customer_email: "nora@atelier.example".into(),
                                iban: "AT611904300234573201".into(),
                                plan_sku: plan.sku.clone(),
                            })
                            .map(|sub| {
                                format!(
                                    "{} {} next bill {} {}",
                                    sub.sub_number,
                                    sub.plan_name,
                                    sub.next_billing_date,
                                    eur(sub.net_price_cents)
                                )
                            })
                            .map_err(|e| e.to_string());
                        finish(app, f, result);
                    }
                    ui.end_row();
                }
            });
        }

        if section == 2 {
            block_title(ui, f.t("Abonnenten", "Subscribers"));
            if subs.is_empty() {
                ui.label(f.t("Noch keine Abonnenten.", "No subscribers yet."));
            }
            Grid::new("sub_list").striped(true).show(ui, |ui| {
                ui.strong(f.t("Nummer", "Number"));
                ui.strong(f.t("Kunde", "Customer"));
                ui.strong(f.t("Plan", "Plan"));
                ui.strong(f.t("Status", "Status"));
                ui.strong(f.t("Nächste Rechnung", "Next bill"));
                ui.strong(f.t("Zyklen", "Cycles"));
                ui.strong(f.t("Aktion", "Action"));
                ui.end_row();
                for sub in &subs {
                    ui.label(&sub.sub_number);
                    ui.label(&sub.customer_name);
                    ui.label(&sub.plan_sku);
                    ui.label(&sub.status);
                    ui.label(&sub.next_billing_date);
                    ui.label(sub.cycles_billed.to_string());
                    ui.horizontal(|ui| {
                        if sub.status == "Active" && ui.button(f.t("Abbuchen", "Bill")).clicked() {
                            let result = app
                                .suite
                                .subscription
                                .store
                                .bill_renewal(&sub.sub_number)
                                .map(|s| {
                                    format!("Billed {} cycle {}", s.sub_number, s.cycles_billed)
                                })
                                .map_err(|e| e.to_string());
                            finish(app, f, result);
                        }
                        if sub.status == "Active" && ui.button(f.t("Aussetzen", "Skip")).clicked() {
                            let result = app
                                .suite
                                .subscription
                                .store
                                .skip_next(&sub.sub_number)
                                .map(|s| {
                                    format!("Skipped {} next {}", s.sub_number, s.next_billing_date)
                                })
                                .map_err(|e| e.to_string());
                            finish(app, f, result);
                        }
                        if sub.status == "Active" && ui.button(f.t("Pausieren", "Pause")).clicked()
                        {
                            let result = app
                                .suite
                                .subscription
                                .store
                                .set_status(&sub.sub_number, "Paused")
                                .map(|s| format!("{} is {}", s.sub_number, s.status))
                                .map_err(|e| e.to_string());
                            finish(app, f, result);
                        }
                        if sub.status == "Paused"
                            && ui.button(f.t("Fortsetzen", "Resume")).clicked()
                        {
                            let result = app
                                .suite
                                .subscription
                                .store
                                .set_status(&sub.sub_number, "Active")
                                .map(|s| format!("{} is {}", s.sub_number, s.status))
                                .map_err(|e| e.to_string());
                            finish(app, f, result);
                        }
                    });
                    ui.end_row();
                }
            });
        }
    });
}

pub fn digital(app: &mut ShopApp, ui: &mut Ui, f: &mut Frame, section: usize) {
    ui.heading(match section {
        0 => f.t("Digital Kennzahlen", "Digital stats"),
        1 => f.t("Digital Katalog", "Digital catalog"),
        2 => f.t("Digital Bestellungen", "Digital orders"),
        _ => f.t("Lizenzen", "Licenses"),
    });
    ui.add_space(6.0);
    if section == 0 {
        if let Some(st) = &app.desk.digital_status {
            kpis(
                ui,
                &[
                    Kpi::new(f.t("Artikel", "Products"), st.active_products.to_string()),
                    Kpi::new("Orders", st.total_orders.to_string()),
                    Kpi::new("Licenses", st.active_licenses.to_string()),
                    Kpi::new("Gross", eur(st.gross_revenue_cents)),
                ],
            );
            ui.label(f.t(
                "Die USt kommt aus der lokalen Tabelle. Es wird keine OSS-Meldung abgegeben.",
                "VAT is the local rate table. Nothing is filed as an OSS return.",
            ));
        }
    }
    let seed = ui
        .horizontal(|ui| seed_button(ui, f.t("Demo-Produkte laden", "Seed demo products")))
        .inner;
    notice(ui, &app.notice.clone());
    if seed {
        let result = app
            .suite
            .digital
            .store
            .seed_demo()
            .map(|(rates, products)| {
                format!("Digital catalog seeded ({rates} VAT rows, {products} products)")
            })
            .map_err(|e| e.to_string());
        finish(app, f, result);
        return;
    }
    if section == 0 {
        return;
    }
    let products = app.desk.digital_products.clone();
    let licenses = app.desk.digital_licenses.clone();
    ScrollArea::vertical()
        .id_salt("digital_desk")
        .show(ui, |ui| {
            if section == 1 {
                block_title(ui, f.t("Katalog", "Catalog"));
                if products.is_empty() {
                    ui.label(f.t(
                        "Noch keine Digitalartikel. Demo laden.",
                        "No digital products yet. Seed the demo catalog.",
                    ));
                }
                Grid::new("digital_products").striped(true).show(ui, |ui| {
                    ui.strong("SKU");
                    ui.strong(f.t("Titel", "Title"));
                    ui.strong(f.t("Art", "Kind"));
                    ui.strong(f.t("Netto", "Net"));
                    ui.strong(f.t("Kauf", "Buy"));
                    ui.end_row();
                    for product in &products {
                        ui.label(&product.sku);
                        ui.label(&product.title);
                        ui.label(&product.kind);
                        ui.label(eur(product.price_net_cents));
                        if ui
                            .add_enabled(product.active, egui::Button::new("Buy AT"))
                            .clicked()
                        {
                            let result = app
                                .suite
                                .digital
                                .store
                                .checkout(&digital::DigitalOrderInput {
                                    buyer_name: "Nora Berg".into(),
                                    buyer_email: "nora@atelier.example".into(),
                                    country_code: "AT".into(),
                                    items: vec![product.sku.clone()],
                                })
                                .map(|order| {
                                    let keys: Vec<&str> = order
                                        .licenses
                                        .iter()
                                        .map(|lic| lic.license_key.as_str())
                                        .collect();
                                    format!(
                                        "{} total {} keys {}",
                                        order.order_number,
                                        eur(order.total_cents),
                                        if keys.is_empty() {
                                            "(no license)".into()
                                        } else {
                                            keys.join(", ")
                                        }
                                    )
                                })
                                .map_err(|e| e.to_string());
                            finish(app, f, result);
                        }
                        ui.end_row();
                    }
                });
            }
            if section == 2 {
                block_title(ui, f.t("Bestellungen", "Orders"));
                if app.desk.digital_orders.is_empty() {
                    ui.label(f.t("Noch keine Digitalbestellungen.", "No digital orders yet."));
                }
                Grid::new("digital_orders").striped(true).show(ui, |ui| {
                    ui.strong(f.t("Auftrag", "Order"));
                    ui.strong(f.t("Käufer", "Buyer"));
                    ui.strong(f.t("Land", "Country"));
                    ui.strong(f.t("Summe", "Total"));
                    ui.end_row();
                    for order in &app.desk.digital_orders {
                        ui.label(&order.order_number);
                        ui.label(&order.buyer_name);
                        ui.label(&order.country_code);
                        ui.label(eur(order.total_cents));
                        ui.end_row();
                    }
                });
            }
            if section == 3 {
                block_title(ui, f.t("Lizenzen", "Licenses"));
                if licenses.is_empty() {
                    ui.label(f.t("Noch keine Lizenzen.", "No licenses issued yet."));
                }
                Grid::new("digital_licenses").striped(true).show(ui, |ui| {
                    ui.strong(f.t("Schlüssel", "Key"));
                    ui.strong("SKU");
                    ui.strong(f.t("Downloads", "Downloads"));
                    ui.strong(f.t("Stand", "State"));
                    ui.strong(f.t("Aktion", "Action"));
                    ui.end_row();
                    for lic in &licenses {
                        ui.label(&lic.license_key);
                        ui.label(&lic.sku);
                        ui.label(format!("{}/{}", lic.downloads_used, lic.max_downloads));
                        ui.label(if lic.revoked {
                            f.t("Widerrufen", "Revoked")
                        } else {
                            f.t("Aktiv", "Active")
                        });
                        ui.horizontal(|ui| {
                            if !lic.revoked
                                && ui
                                    .button(f.t("Download einlösen", "Redeem download"))
                                    .clicked()
                            {
                                let result = app
                                    .suite
                                    .digital
                                    .store
                                    .redeem_download(&lic.download_token)
                                    .map(|got| {
                                        format!(
                                            "{} downloads {}/{}",
                                            got.license_key, got.downloads_used, got.max_downloads
                                        )
                                    })
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                            if !lic.revoked && ui.button(f.t("Widerrufen", "Revoke")).clicked() {
                                let result = app
                                    .suite
                                    .digital
                                    .store
                                    .revoke_license(&lic.license_key)
                                    .map(|()| format!("Revoked {}", lic.license_key))
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                        });
                        ui.end_row();
                    }
                });
            }
        });
}

pub fn wholesale(app: &mut ShopApp, ui: &mut Ui, f: &mut Frame, section: usize) {
    ui.heading(match section {
        0 => f.t("Großhandel Kennzahlen", "Wholesale stats"),
        1 => f.t("Handelspartner", "Trade accounts"),
        2 => f.t("Großhandel Katalog", "Wholesale catalog"),
        _ => f.t("Bestellungen", "Purchase orders"),
    });
    ui.add_space(6.0);
    if section == 0 {
        if let Some(st) = &app.desk.wholesale_status {
            kpis(
                ui,
                &[
                    Kpi::new("Accounts", st.trade_accounts.to_string()),
                    Kpi::new(f.t("Artikel", "Products"), st.active_products.to_string()),
                    Kpi::new(
                        f.t("Offene Aufträge", "Open orders"),
                        st.open_orders.to_string(),
                    ),
                    Kpi::new(f.t("Umsatz", "Revenue"), eur(st.total_revenue_cents)),
                ],
            );
        }
    }
    let seed = ui
        .horizontal(|ui| seed_button(ui, f.t("Demo-Handelsbuch laden", "Seed demo trade book")))
        .inner;
    notice(ui, &app.notice.clone());
    if seed {
        let result = app
            .suite
            .wholesale
            .store
            .seed_demo()
            .map(|(accounts, products)| {
                format!("Wholesale seeded ({accounts} accounts, {products} products)")
            })
            .map_err(|e| e.to_string());
        finish(app, f, result);
        return;
    }
    if section == 0 {
        return;
    }
    let accounts = app.desk.accounts.clone();
    let products = app.desk.ws_products.clone();
    let orders = app.desk.ws_orders.clone();
    let account = accounts.first().map(|a| a.account_code.clone());
    ScrollArea::vertical()
        .id_salt("wholesale_desk")
        .show(ui, |ui| {
            if section == 1 {
                block_title(ui, f.t("Konten", "Accounts"));
                if accounts.is_empty() {
                    ui.label(f.t(
                        "Noch keine Handelspartner. Demo laden.",
                        "No trade accounts yet. Seed the demo book.",
                    ));
                }
                Grid::new("ws_accounts").striped(true).show(ui, |ui| {
                    ui.strong(f.t("Code", "Code"));
                    ui.strong(f.t("Firma", "Company"));
                    ui.strong(f.t("Stufe", "Tier"));
                    ui.strong(f.t("Konditionen", "Terms"));
                    ui.strong(f.t("Saldo", "Balance"));
                    ui.end_row();
                    for account in &accounts {
                        ui.label(&account.account_code);
                        ui.label(&account.company_name);
                        ui.label(&account.tier);
                        ui.label(&account.payment_terms);
                        ui.label(eur(account.balance_cents));
                        ui.end_row();
                    }
                });
            }
            if section == 2 {
                block_title(ui, f.t("Katalog", "Catalog"));
                Grid::new("ws_products").striped(true).show(ui, |ui| {
                    ui.strong("SKU");
                    ui.strong(f.t("Name", "Name"));
                    ui.strong(f.t("Mindestmenge", "MOQ"));
                    ui.strong(f.t("Liste", "List"));
                    ui.strong(f.t("Bestand", "Stock"));
                    ui.strong(f.t("Aktion", "Action"));
                    ui.end_row();
                    for product in &products {
                        ui.label(&product.sku);
                        ui.label(&product.name);
                        ui.label(product.moq.to_string());
                        ui.label(eur(product.list_price_cents));
                        ui.label(product.stock.to_string());
                        ui.horizontal(|ui| {
                            let qty = product.moq.max(1);
                            if ui.button(f.t("MOQ anbieten", "Quote MOQ")).clicked() {
                                let Some(code) = account.clone() else {
                                    finish(app, f, Err("Seed a trade account first".into()));
                                    return;
                                };
                                let result = app
                                    .suite
                                    .wholesale
                                    .store
                                    .quote(
                                        &code,
                                        &[wholesale::OrderLineInput {
                                            sku: product.sku.clone(),
                                            qty,
                                        }],
                                    )
                                    .map(|q| {
                                        format!(
                                            "Quote {} {} qty {qty} {} total {} credit {}",
                                            q.account_code,
                                            q.tier,
                                            product.sku,
                                            eur(q.total_cents),
                                            if q.credit_ok { "ok" } else { "over limit" }
                                        )
                                    })
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                            if ui.button(f.t("MOQ bestellen", "Order MOQ")).clicked() {
                                let Some(code) = account.clone() else {
                                    finish(app, f, Err("Seed a trade account first".into()));
                                    return;
                                };
                                let po_reference = format!("PO-UI-{}", orders.len() + 1);
                                let result = app
                                    .suite
                                    .wholesale
                                    .store
                                    .create_order(&wholesale::CreateOrderInput {
                                        account_code: code,
                                        po_reference,
                                        items: vec![wholesale::OrderLineInput {
                                            sku: product.sku.clone(),
                                            qty,
                                        }],
                                    })
                                    .map(|po| {
                                        format!(
                                            "{} {} {} {}",
                                            po.order_number,
                                            po.company_name,
                                            po.status,
                                            eur(po.total_cents)
                                        )
                                    })
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                        });
                        ui.end_row();
                    }
                });
            }
            if section == 3 {
                block_title(ui, f.t("Bestellungen", "Purchase orders"));
                if orders.is_empty() {
                    ui.label(f.t("Noch keine Bestellungen.", "No purchase orders yet."));
                }
                Grid::new("ws_orders").striped(true).show(ui, |ui| {
                    ui.strong(f.t("Auftrag", "Order"));
                    ui.strong(f.t("Konto", "Account"));
                    ui.strong(f.t("Status", "Status"));
                    ui.strong(f.t("Summe", "Total"));
                    ui.strong(f.t("Aktion", "Action"));
                    ui.end_row();
                    for order in &orders {
                        ui.label(&order.order_number);
                        ui.label(&order.account_code);
                        ui.label(&order.status);
                        ui.label(eur(order.total_cents));
                        ui.horizontal(|ui| {
                            if let Some(next) = next_wholesale(&order.status) {
                                if ui.button(next).clicked() {
                                    let result = app
                                        .suite
                                        .wholesale
                                        .store
                                        .transition_order(&order.order_number, next)
                                        .map(|po| format!("{} is {}", po.order_number, po.status))
                                        .map_err(|e| e.to_string());
                                    finish(app, f, result);
                                }
                            }
                            if matches!(order.status.as_str(), "Submitted" | "Approved")
                                && ui.button(f.t("Stornieren", "Cancel")).clicked()
                            {
                                let result = app
                                    .suite
                                    .wholesale
                                    .store
                                    .transition_order(&order.order_number, "Cancelled")
                                    .map(|po| format!("{} is {}", po.order_number, po.status))
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                        });
                        ui.end_row();
                    }
                });
            }
        });
}

fn next_wholesale(status: &str) -> Option<&'static str> {
    match status {
        "Submitted" => Some("Approved"),
        "Approved" => Some("Dispatched"),
        "Dispatched" => Some("Invoiced"),
        _ => None,
    }
}

pub fn tickets(app: &mut ShopApp, ui: &mut Ui, f: &mut Frame, section: usize) {
    ui.heading(match section {
        0 => f.t("Ticket Kennzahlen", "Ticket stats"),
        1 => f.t("Veranstaltungen", "Events"),
        _ => f.t("Ausgestellte Tickets", "Issued tickets"),
    });
    ui.add_space(6.0);
    if section == 0 {
        if let Some(st) = &app.desk.ticket_status {
            kpis(
                ui,
                &[
                    Kpi::new("Events", st.events_count.to_string()),
                    Kpi::new("Capacity", st.total_capacity.to_string()),
                    Kpi::new("Sold", st.seats_sold.to_string()),
                    Kpi::new("Gross", eur(st.gross_revenue_cents)),
                ],
            );
        }
    }
    let seed = ui
        .horizontal(|ui| seed_button(ui, f.t("Demo-Veranstaltungen laden", "Seed demo events")))
        .inner;
    notice(ui, &app.notice.clone());
    if seed {
        let result = app
            .suite
            .ticket
            .store
            .seed_demo()
            .map(|n| format!("Events seeded ({n} new rows)"))
            .map_err(|e| e.to_string());
        finish(app, f, result);
        return;
    }
    if section == 0 {
        return;
    }
    let events = app.desk.events.clone();
    let tickets = app.desk.tickets.clone();
    ScrollArea::vertical()
        .id_salt("ticket_desk")
        .show(ui, |ui| {
            if section == 1 {
                block_title(ui, f.t("Veranstaltungen", "Events"));
                if events.is_empty() {
                    ui.label(f.t(
                        "Noch keine Veranstaltungen. Demo laden.",
                        "No events yet. Seed the demo events.",
                    ));
                }
                Grid::new("ticket_events").striped(true).show(ui, |ui| {
                    ui.strong(f.t("Code", "Code"));
                    ui.strong(f.t("Veranstaltung", "Event"));
                    ui.strong(f.t("Wann", "When"));
                    ui.strong(f.t("Rest", "Left"));
                    ui.strong(f.t("Preis", "Price"));
                    ui.strong(f.t("Verkauf", "Sell"));
                    ui.end_row();
                    for event in &events {
                        let left = (event.capacity - event.sold_count).max(0);
                        ui.label(&event.code);
                        ui.label(format!("{} / {}", event.title, event.tier_name));
                        ui.label(&event.event_date);
                        ui.label(left.to_string());
                        ui.label(eur(event.price_cents));
                        if ui
                            .add_enabled(
                                event.active && left > 0,
                                egui::Button::new(f.t("1 verkaufen", "Sell 1")),
                            )
                            .clicked()
                        {
                            let result = app
                                .suite
                                .ticket
                                .store
                                .issue_ticket(&ticket::IssueTicketInput {
                                    attendee_name: "Nora Berg".into(),
                                    attendee_email: "nora@atelier.example".into(),
                                    code: event.code.clone(),
                                    quantity: 1,
                                })
                                .map(|order| {
                                    format!(
                                        "{} {} {}",
                                        order.ticket_code,
                                        order.event_title,
                                        eur(order.total_cents)
                                    )
                                })
                                .map_err(|e| e.to_string());
                            finish(app, f, result);
                        }
                        ui.end_row();
                    }
                });
            }
            if section == 2 {
                block_title(ui, f.t("Ausgestellte Tickets", "Issued tickets"));
                if tickets.is_empty() {
                    ui.label(f.t("Noch keine Tickets.", "No tickets issued yet."));
                }
                Grid::new("ticket_orders").striped(true).show(ui, |ui| {
                    ui.strong(f.t("Ticket", "Ticket"));
                    ui.strong(f.t("Teilnehmer", "Attendee"));
                    ui.strong(f.t("Veranstaltung", "Event"));
                    ui.strong(f.t("Status", "Status"));
                    ui.strong(f.t("Aktion", "Action"));
                    ui.end_row();
                    for ticket in &tickets {
                        ui.label(&ticket.ticket_code);
                        ui.label(&ticket.attendee_name);
                        ui.label(&ticket.event_title);
                        ui.label(&ticket.status);
                        ui.horizontal(|ui| {
                            if ticket.status == "issued"
                                && ui.button(f.t("Einchecken", "Check in")).clicked()
                            {
                                let result = app
                                    .suite
                                    .ticket
                                    .store
                                    .check_in(&ticket.ticket_code)
                                    .map(|t| format!("{} {}", t.ticket_code, t.status))
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                            if ticket.status == "issued"
                                && ui.button(f.t("Erstatten", "Refund")).clicked()
                            {
                                let result = app
                                    .suite
                                    .ticket
                                    .store
                                    .refund_ticket(&ticket.ticket_code)
                                    .map(|t| format!("{} {}", t.ticket_code, t.status))
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                        });
                        ui.end_row();
                    }
                });
            }
        });
}

pub fn rental(app: &mut ShopApp, ui: &mut Ui, f: &mut Frame, section: usize) {
    ui.heading(match section {
        0 => f.t("Verleih Kennzahlen", "Rental stats"),
        1 => f.t("Geräte", "Gear"),
        _ => f.t("Buchungen", "Bookings"),
    });
    ui.add_space(6.0);
    if section == 0 {
        if let Some(st) = &app.desk.rental_status {
            kpis(
                ui,
                &[
                    Kpi::new("SKUs", st.fleet_skus.to_string()),
                    Kpi::new("Units", st.total_units.to_string()),
                    Kpi::new("Active bookings", st.active_bookings.to_string()),
                    Kpi::new("Deposits", eur(st.deposits_held_cents)),
                ],
            );
            ui.label(f.t(
                "Eine Buchung wird abgelehnt, wenn die Daten ein bereits verliehenes Gerät überlappen.",
                "A booking is refused when the dates overlap a unit that is already out.",
            ));
        }
    }
    let seed = ui
        .horizontal(|ui| seed_button(ui, f.t("Demo-Geräte laden", "Seed demo gear")))
        .inner;
    notice(ui, &app.notice.clone());
    if seed {
        let result = app
            .suite
            .rental
            .store
            .seed_demo()
            .map(|n| format!("Rental gear seeded ({n} new rows)"))
            .map_err(|e| e.to_string());
        finish(app, f, result);
        return;
    }
    if section == 0 {
        return;
    }
    let gear = app.desk.gear.clone();
    let bookings = app.desk.bookings.clone();
    let slot = app.rental_slot % RENTAL_SLOTS.len();
    let (start, end) = RENTAL_SLOTS[slot];
    ui.label(format!("Next booking window {start} to {end}."));
    ScrollArea::vertical()
        .id_salt("rental_desk")
        .show(ui, |ui| {
            if section == 1 {
                block_title(ui, f.t("Geräte", "Gear"));
                if gear.is_empty() {
                    ui.label(f.t(
                        "Noch keine Geräte. Demo laden.",
                        "No gear yet. Seed the demo fleet.",
                    ));
                }
                Grid::new("rental_gear").striped(true).show(ui, |ui| {
                    ui.strong("SKU");
                    ui.strong(f.t("Name", "Name"));
                    ui.strong(f.t("Tag", "Day"));
                    ui.strong(f.t("Kaution", "Deposit"));
                    ui.strong(f.t("Stück", "Units"));
                    ui.strong(f.t("Buchen", "Book"));
                    ui.end_row();
                    for item in &gear {
                        ui.label(&item.sku);
                        ui.label(&item.name);
                        ui.label(eur(item.daily_rate_cents));
                        ui.label(eur(item.deposit_cents));
                        ui.label(item.units_total.to_string());
                        if ui
                            .add_enabled(item.active, egui::Button::new("Book window"))
                            .clicked()
                        {
                            let result = app
                                .suite
                                .rental
                                .store
                                .book(&rental::CreateBookingInput {
                                    customer_name: "Max Hofer".into(),
                                    customer_email: "max@example.at".into(),
                                    sku: item.sku.clone(),
                                    start_date: start.into(),
                                    end_date: end.into(),
                                })
                                .map(|b| {
                                    format!(
                                        "{} {} {} to {} fee {} deposit {}",
                                        b.booking_number,
                                        b.item_name,
                                        b.start_date,
                                        b.end_date,
                                        eur(b.rental_fee_cents),
                                        eur(b.deposit_cents)
                                    )
                                })
                                .map_err(|e| e.to_string());
                            if result.is_ok() {
                                app.rental_slot = app.rental_slot.saturating_add(1);
                            }
                            finish(app, f, result);
                        }
                        ui.end_row();
                    }
                });
            }
            if section == 2 {
                block_title(ui, f.t("Buchungen", "Bookings"));
                if bookings.is_empty() {
                    ui.label(f.t("Noch keine Buchungen.", "No bookings yet."));
                }
                Grid::new("rental_bookings").striped(true).show(ui, |ui| {
                    ui.strong(f.t("Buchung", "Booking"));
                    ui.strong(f.t("Gerät", "Gear"));
                    ui.strong(f.t("Daten", "Dates"));
                    ui.strong(f.t("Status", "Status"));
                    ui.strong(f.t("Gebühr", "Fee"));
                    ui.strong(f.t("Aktion", "Action"));
                    ui.end_row();
                    for booking in &bookings {
                        ui.label(&booking.booking_number);
                        ui.label(&booking.item_name);
                        ui.label(format!("{} to {}", booking.start_date, booking.end_date));
                        ui.label(&booking.status);
                        ui.label(eur(booking.rental_fee_cents));
                        ui.horizontal(|ui| {
                            if booking.status == "Reserved"
                                && ui.button(f.t("Ausgeben", "Check out")).clicked()
                            {
                                let result = app
                                    .suite
                                    .rental
                                    .store
                                    .checkout(&booking.booking_number)
                                    .map(|b| format!("{} {}", b.booking_number, b.status))
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                            if booking.status == "CheckedOut"
                                && ui
                                    .button(f.t("Am Enddatum zurück", "Return on end date"))
                                    .clicked()
                            {
                                let result = app
                                    .suite
                                    .rental
                                    .store
                                    .checkin(&booking.booking_number, &booking.end_date, 0)
                                    .map(|b| {
                                        format!(
                                            "{} returned, deposit back {}",
                                            b.booking_number,
                                            eur(b.deposit_refund_cents)
                                        )
                                    })
                                    .map_err(|e| e.to_string());
                                finish(app, f, result);
                            }
                        });
                        ui.end_row();
                    }
                });
            }
        });
}
