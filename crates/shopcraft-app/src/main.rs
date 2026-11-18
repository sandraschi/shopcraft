#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod chart;
mod desk;
mod webfront;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use craft_biz::bus::Bus;
use craft_biz::mcp::ToolHost;
use craft_biz::Money;
use craft_biz_ui::app::{main_with, AppMenus, Frame, NavEntry, SuiteApp, Wiring};
use craft_biz_ui::menu::{Item, Menu};
use craft_biz_ui::theme::Tone;
use craft_biz_ui::widgets::{self, Kpi};
use egui::{Grid, RichText, ScrollArea, Ui};
use serde_json::{json, Value};
use shopcraft_core::models::{CalendarDay, Employee, OpeningHour, PosItem, PosStatus, Shift};
use shopcraft_core::sales::SalesPoint;
use shopcraft_core::suite::ShopSuite;

struct ShopApp {
    suite: ShopSuite,
    items: Vec<PosItem>,
    status: Option<PosStatus>,
    active_shift: Option<Shift>,
    last_receipt_text: String,
    hours: Vec<OpeningHour>,
    calendar: Vec<CalendarDay>,
    employees: Vec<Employee>,
    cal_date: String,
    cal_note: String,
    emp_name: String,
    emp_role: String,
    desk: desk::Desk,
    notice: String,
    rental_slot: usize,
    mode: usize,
}

impl ShopApp {
    fn open(path: &Path) -> Result<Self, String> {
        let suite = ShopSuite::open(path)?;
        let mut app = Self {
            suite,
            items: Vec::new(),
            status: None,
            active_shift: None,
            last_receipt_text: String::new(),
            hours: Vec::new(),
            calendar: Vec::new(),
            employees: Vec::new(),
            cal_date: "2026-12-25".into(),
            cal_note: "Christmas Day".into(),
            emp_name: String::new(),
            emp_role: "Clerk".into(),
            desk: desk::Desk::empty(),
            notice: String::new(),
            rental_slot: 0,
            mode: 0,
        };
        app.reload()?;
        Ok(app)
    }

    fn view_retail(&mut self, ui: &mut Ui, f: &mut Frame, section: usize) {
        let title = match section {
            0 => f.t("Kennzahlen", "Stats"),
            1 => f.t("Kasse", "Register"),
            2 => f.t("Katalog", "Catalog"),
            3 => f.t("Öffnungszeiten", "Hours"),
            4 => f.t("Kalender", "Calendar"),
            5 => f.t("Personal", "Roster"),
            _ => f.t("Schicht", "Shift"),
        };
        ui.heading(title);
        ui.add_space(6.0);
        if section == 0 {
            if let Some(st) = &self.status {
                widgets::kpi_row(
                    ui,
                    &[
                        Kpi::new(
                            f.t("Aktive Artikel", "Active Items"),
                            st.active_items.to_string(),
                        ),
                        Kpi::new(
                            f.t("Offene Schicht", "Open Shift"),
                            st.open_shift
                                .clone()
                                .unwrap_or_else(|| f.t("Keine", "None").into()),
                        ),
                        Kpi::new(f.t("Belege", "Receipts"), st.total_receipts.to_string()),
                        Kpi::new(
                            f.t("Bruttoumsatz", "Gross Sales"),
                            Money::from_cents(st.gross_sales_cents).to_string(),
                        ),
                    ],
                );
            }
            ui.add_space(8.0);
            if ui
                .button(f.t("Demo-Artikel laden", "Seed demo items"))
                .clicked()
            {
                match self.suite.call("shop_seed", &json!({})) {
                    Ok(_) => {
                        let _ = self.reload();
                        f.toast("Retail catalog seeded", Tone::Teal);
                    }
                    Err(e) => f.toast(e, Tone::Danger),
                }
            }
            ui.label(f.t(
                "Diese Zahlen gehören zu dieser Kasse. Sie sind nicht der Lagerbestand in stockcraft.",
                "These counts belong to this till. They are not stockcraft on-hand.",
            ));
            return;
        }
        if section == 3 {
            self.view_hours(ui, f);
            return;
        }
        if section == 4 {
            self.view_calendar(ui, f);
            return;
        }
        if section == 5 {
            self.view_roster(ui, f);
            return;
        }
        if section != 1 && section != 2 {
            ui.label(f.t(
                "Kasse öffnen, auf Kasse verkaufen, hier schließen. Das Schließen schreibt einen Z-Bon.",
                "Open the till, sell on Register, then close it here. The close writes a Z-Bon.",
            ));
            ui.add_space(8.0);
            if self.active_shift.is_none() {
                if ui
                    .button(f.t(
                        "Schicht öffnen (Lukas, EUR 100,00)",
                        "Open Shift (Lukas, EUR 100.00)",
                    ))
                    .clicked()
                {
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
                ui.label(format!(
                    "{} {} {} {}",
                    f.t("Offene Schicht", "Open shift"),
                    sh.shift_id,
                    f.t("Kasse", "clerk"),
                    sh.clerk_name
                ));
                if ui
                    .button(f.t(
                        "Schicht schließen und Z-Bon schreiben",
                        "Close Shift and write Z-Bon",
                    ))
                    .clicked()
                {
                    match self.suite.call(
                        "shop_shift_close",
                        &json!({ "shift_id": sh.shift_id, "counted_cash_cents": sh.expected_cash_cents }),
                    ) {
                        Ok(_) => {
                            let _ = self.reload();
                            f.toast("Shift closed. Z-Bon sent toward buchcraft.", Tone::Teal);
                        }
                        Err(e) => f.toast(e, Tone::Danger),
                    }
                }
            }
            return;
        }

        let sell = section == 1;
        if sell && self.active_shift.is_none() {
            ui.label(f.t(
                "Keine Schicht offen. Öffne eine unter Schicht, bevor du verkaufst.",
                "No shift is open. Open one on Shift before selling.",
            ));
        }
        ui.add_space(8.0);
        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("pos_items_grid").striped(true).show(ui, |ui| {
                ui.strong("SKU");
                ui.strong("EAN-13");
                ui.strong(f.t("Titel", "Title"));
                ui.strong(f.t("Preis", "Price"));
                ui.strong(f.t("Bestand", "Stock"));
                if sell {
                    ui.strong(f.t("Verkauf", "Sale"));
                }
                ui.end_row();

                let items = self.items.clone();
                for it in items {
                    ui.label(&it.sku);
                    ui.label(&it.barcode);
                    ui.label(&it.title);
                    ui.label(Money::from_cents(it.price_cents).to_string());
                    ui.label(it.stock_qty.to_string());
                    if sell {
                        let can_sell = self.active_shift.is_some() && it.stock_qty > 0;
                        if ui
                            .add_enabled(
                                can_sell,
                                egui::Button::new(f.t("1x verkaufen", "Sell 1x")),
                            )
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
                                        if let Some(rno) =
                                            val.get("receipt_no").and_then(Value::as_str)
                                        {
                                            self.last_receipt_text = self
                                                .suite
                                                .retail
                                                .store
                                                .format_receipt_text(rno)
                                                .unwrap_or_default();
                                        }
                                        let _ = self.reload();
                                        f.toast("Sale completed", Tone::Teal);
                                    }
                                    Err(e) => f.toast(e, Tone::Danger),
                                }
                            }
                        }
                    }
                    ui.end_row();
                }
            });
            if sell {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(f.t(
                        "Belegvorschau, 42 Spalten ESC/POS",
                        "42-Col ESC/POS Receipt Preview",
                    ))
                    .strong(),
                );
                let empty = f.t(
                    "Schließe einen Verkauf ab, dann erscheint der Beleg.",
                    "Complete a sale to preview the thermal receipt.",
                );
                let text = if self.last_receipt_text.is_empty() {
                    empty
                } else {
                    self.last_receipt_text.as_str()
                };
                ui.monospace(text);
            }
        });
    }

    fn view_hours(&mut self, ui: &mut Ui, f: &mut Frame) {
        ui.label(f.t(
            "Montag bis Freitag 10:00-18:00, Samstag 10:00-16:00, Sonntag geschlossen ist die Standardwoche.",
            "Monday to Friday 10:00-18:00, Saturday 10:00-16:00, Sunday closed, is the standard week.",
        ));
        ui.add_space(6.0);
        if ui
            .button(f.t("Standardwoche setzen", "Install standard week"))
            .clicked()
        {
            match self.suite.retail.store.install_standard_week() {
                Ok(()) => {
                    let _ = self.reload();
                    f.toast("Standard week saved", Tone::Teal);
                }
                Err(e) => f.toast(e.to_string(), Tone::Danger),
            }
        }
        if self.hours.is_empty() {
            ui.label(f.t("Noch keine Öffnungszeiten.", "No hours saved yet."));
            return;
        }
        ui.add_space(8.0);
        let hours = self.hours.clone();
        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("opening_hours").striped(true).show(ui, |ui| {
                ui.strong(f.t("Tag", "Day"));
                ui.strong(f.t("Zeiten", "Hours"));
                ui.strong(f.t("Anpassen", "Adjust"));
                ui.end_row();
                for row in hours {
                    ui.label(weekday_name(row.weekday, f.lang_de));
                    if row.closed {
                        ui.label(f.t("Geschlossen", "Closed"));
                    } else {
                        ui.label(format!(
                            "{} to {}",
                            hhmm(row.open_minute),
                            hhmm(row.close_minute)
                        ));
                    }
                    ui.push_id(row.weekday, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button(f.t("Auf -30", "Open -30")).clicked() {
                                self.save_hours(
                                    f,
                                    row.weekday,
                                    row.open_minute - 30,
                                    row.close_minute,
                                    row.closed,
                                );
                            }
                            if ui.button(f.t("Auf +30", "Open +30")).clicked() {
                                self.save_hours(
                                    f,
                                    row.weekday,
                                    row.open_minute + 30,
                                    row.close_minute,
                                    row.closed,
                                );
                            }
                            if ui.button(f.t("Zu -30", "Close -30")).clicked() {
                                self.save_hours(
                                    f,
                                    row.weekday,
                                    row.open_minute,
                                    row.close_minute - 30,
                                    row.closed,
                                );
                            }
                            if ui.button(f.t("Zu +30", "Close +30")).clicked() {
                                self.save_hours(
                                    f,
                                    row.weekday,
                                    row.open_minute,
                                    row.close_minute + 30,
                                    row.closed,
                                );
                            }
                            if row.closed {
                                if ui.button(f.t("Geöffnet", "Mark open")).clicked() {
                                    let (open_minute, close_minute) =
                                        if row.close_minute > row.open_minute {
                                            (row.open_minute, row.close_minute)
                                        } else {
                                            (10 * 60, 16 * 60)
                                        };
                                    self.save_hours(
                                        f,
                                        row.weekday,
                                        open_minute,
                                        close_minute,
                                        false,
                                    );
                                }
                            } else if ui.button(f.t("Geschlossen", "Mark closed")).clicked() {
                                self.save_hours(
                                    f,
                                    row.weekday,
                                    row.open_minute,
                                    row.close_minute,
                                    true,
                                );
                            }
                        });
                    });
                    ui.end_row();
                }
            });
        });
    }

    fn save_hours(
        &mut self,
        f: &mut Frame,
        weekday: i64,
        open_minute: i64,
        close_minute: i64,
        closed: bool,
    ) {
        match self
            .suite
            .retail
            .store
            .set_opening_hours(weekday, open_minute, close_minute, closed)
        {
            Ok(()) => {
                let _ = self.reload();
            }
            Err(e) => f.toast(e.to_string(), Tone::Danger),
        }
    }

    fn view_calendar(&mut self, ui: &mut Ui, f: &mut Frame) {
        ui.label(f.t(
            "Tage, die von der normalen Woche abweichen. Ein kurzer Tag hat eigene Zeiten.",
            "Days that are not the normal week. A short day keeps its own open and close.",
        ));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(f.t("Datum", "Date"));
            ui.text_edit_singleline(&mut self.cal_date);
            ui.label(f.t("Notiz", "Note"));
            ui.text_edit_singleline(&mut self.cal_note);
        });
        ui.horizontal(|ui| {
            if ui
                .button(f.t("Geschlossenen Tag", "Add closed day"))
                .clicked()
            {
                self.save_calendar(f, "closed", None, None);
            }
            if ui
                .button(f.t("Kurzer Tag 10:00-14:00", "Add short day 10:00-14:00"))
                .clicked()
            {
                self.save_calendar(f, "short", Some(10 * 60), Some(14 * 60));
            }
        });
        if self.calendar.is_empty() {
            ui.label(f.t("Noch keine Ausnahmen.", "No exceptions yet."));
            return;
        }
        ui.add_space(8.0);
        let days = self.calendar.clone();
        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("calendar_days").striped(true).show(ui, |ui| {
                ui.strong(f.t("Datum", "Date"));
                ui.strong(f.t("Art", "Kind"));
                ui.strong(f.t("Zeiten", "Hours"));
                ui.strong(f.t("Notiz", "Note"));
                ui.strong(f.t("Aktion", "Action"));
                ui.end_row();
                for day in days {
                    ui.label(&day.day);
                    ui.label(&day.kind);
                    match (day.open_minute, day.close_minute) {
                        (Some(open_minute), Some(close_minute)) if day.kind != "closed" => {
                            ui.label(format!("{} to {}", hhmm(open_minute), hhmm(close_minute)));
                        }
                        _ => {
                            ui.label(f.t("Geschlossen", "Closed"));
                        }
                    }
                    ui.label(&day.note);
                    ui.push_id(&day.day, |ui| {
                        if ui.button(f.t("Entfernen", "Remove")).clicked() {
                            match self.suite.retail.store.delete_calendar_day(&day.day) {
                                Ok(()) => {
                                    let _ = self.reload();
                                    f.toast("Day removed", Tone::Teal);
                                }
                                Err(e) => f.toast(e.to_string(), Tone::Danger),
                            }
                        }
                    });
                    ui.end_row();
                }
            });
        });
    }

    fn save_calendar(
        &mut self,
        f: &mut Frame,
        kind: &str,
        open_minute: Option<i64>,
        close_minute: Option<i64>,
    ) {
        let day = self.cal_date.trim().to_string();
        let note = self.cal_note.trim().to_string();
        match self.suite.retail.store.upsert_calendar_day(
            &day,
            kind,
            open_minute,
            close_minute,
            &note,
        ) {
            Ok(()) => {
                let _ = self.reload();
                f.toast("Calendar day saved", Tone::Teal);
            }
            Err(e) => f.toast(e.to_string(), Tone::Danger),
        }
    }

    fn view_roster(&mut self, ui: &mut Ui, f: &mut Frame) {
        ui.label(f.t(
            "Personen in diesem Laden. Wer geht, wird deaktiviert. Die Zeile bleibt.",
            "People who work this shop. Deactivate someone who has left. The row stays.",
        ));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(f.t("Name", "Name"));
            ui.text_edit_singleline(&mut self.emp_name);
            ui.label(f.t("Rolle", "Role"));
            ui.text_edit_singleline(&mut self.emp_role);
            if ui.button(f.t("Aufnehmen", "Add to roster")).clicked() {
                match self
                    .suite
                    .retail
                    .store
                    .add_employee(&self.emp_name, &self.emp_role)
                {
                    Ok(employee) => {
                        self.emp_name.clear();
                        let _ = self.reload();
                        f.toast(format!("{} added", employee.employee_id), Tone::Teal);
                    }
                    Err(e) => f.toast(e.to_string(), Tone::Danger),
                }
            }
        });
        if self.employees.is_empty() {
            ui.label(f.t(
                "Noch niemand. Demo-Artikel laden, oder Name und Rolle eintragen.",
                "No employees yet. Seed demo items, or add a name and a role.",
            ));
            return;
        }
        ui.add_space(8.0);
        let staff = self.employees.clone();
        ScrollArea::vertical().show(ui, |ui| {
            Grid::new("employees").striped(true).show(ui, |ui| {
                ui.strong(f.t("Nr", "Id"));
                ui.strong(f.t("Name", "Name"));
                ui.strong(f.t("Rolle", "Role"));
                ui.strong(f.t("Status", "Status"));
                ui.strong(f.t("Aktion", "Action"));
                ui.end_row();
                for employee in staff {
                    ui.label(&employee.employee_id);
                    ui.label(&employee.name);
                    ui.label(&employee.role);
                    ui.label(if employee.active {
                        f.t("Aktiv", "Active")
                    } else {
                        f.t("Inaktiv", "Inactive")
                    });
                    let label = if employee.active {
                        f.t("Deaktivieren", "Deactivate")
                    } else {
                        f.t("Aktivieren", "Activate")
                    };
                    ui.push_id(&employee.employee_id, |ui| {
                        if ui.button(label).clicked() {
                            match self
                                .suite
                                .retail
                                .store
                                .set_employee_active(&employee.employee_id, !employee.active)
                            {
                                Ok(()) => {
                                    let _ = self.reload();
                                }
                                Err(e) => f.toast(e.to_string(), Tone::Danger),
                            }
                        }
                    });
                    ui.end_row();
                }
            });
        });
    }

    fn view_sales(&self, ui: &mut Ui, f: &Frame) {
        let caption = match self.mode {
            0 => f.t(
                "Verkauf minus Storno, ein Balken pro Tag.",
                "Net of sales and storno, one bar per day.",
            ),
            1 => f.t(
                "Bestellungen, die offen, bezahlt, in Kommissionierung oder versendet sind.",
                "Orders that are still open, paid, picking, or shipped.",
            ),
            2 => f.t(
                "Brutto der Amazon-Bestellungen, die nicht storniert sind.",
                "Gross of Amazon orders that are not cancelled.",
            ),
            3 => f.t(
                "Abgerechnete Zyklen. Ein Aussetzen ist kein Umsatz.",
                "Billed renewal cycles. A skip is not a sale.",
            ),
            4 => f.t(
                "Digitale Käufe, inklusive USt.",
                "Digital checkouts, VAT included.",
            ),
            5 => f.t(
                "Handelsaufträge, die nicht storniert sind.",
                "Trade orders that are not cancelled.",
            ),
            6 => f.t(
                "Tickets, die nicht erstattet sind.",
                "Tickets that are not refunded.",
            ),
            _ => f.t(
                "Miete, Verspätung und Schaden. Die Kaution zählt nicht.",
                "Rental, late, and damage fees. The deposit is not sales.",
            ),
        };
        match self.load_sales() {
            Ok(points) => chart::show(ui, f, &points, caption),
            Err(error) => {
                ui.heading(f.t("Umsatz", "Sales"));
                ui.label(error);
            }
        }
    }

    fn load_sales(&self) -> Result<Vec<SalesPoint>, String> {
        match self.mode {
            0 => self
                .suite
                .retail
                .store
                .sales_by_day()
                .map_err(|e| e.to_string()),
            1 => self
                .suite
                .web
                .store
                .sales_by_day()
                .map_err(|e| e.to_string()),
            2 => self
                .suite
                .amazon
                .store
                .sales_by_day()
                .map_err(|e| e.to_string()),
            3 => self
                .suite
                .subscription
                .store
                .sales_by_day()
                .map_err(|e| e.to_string()),
            4 => self
                .suite
                .digital
                .store
                .sales_by_day()
                .map_err(|e| e.to_string()),
            5 => self
                .suite
                .wholesale
                .store
                .sales_by_day()
                .map_err(|e| e.to_string()),
            6 => self
                .suite
                .ticket
                .store
                .sales_by_day()
                .map_err(|e| e.to_string()),
            _ => self
                .suite
                .rental
                .store
                .sales_by_day()
                .map_err(|e| e.to_string()),
        }
    }

    fn open_webshop(&mut self, f: &mut Frame) {
        match webfront::open(self) {
            Ok(msg) => {
                self.notice = msg;
                f.toast("Webshop opened in the browser", Tone::Teal);
            }
            Err(err) => {
                self.notice = err.clone();
                f.toast(err, Tone::Danger);
            }
        }
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
        self.hours = self
            .suite
            .retail
            .store
            .list_opening_hours()
            .map_err(|e| e.to_string())?;
        self.calendar = self
            .suite
            .retail
            .store
            .list_calendar_days()
            .map_err(|e| e.to_string())?;
        self.employees = self
            .suite
            .retail
            .store
            .list_employees()
            .map_err(|e| e.to_string())?;
        self.desk = desk::Desk::load(&self.suite)?;
        Ok(())
    }
    fn nav(&self, f: &Frame) -> Vec<NavEntry> {
        let spec: &[(&str, &str, &str)] = match self.mode {
            0 => &[
                (f.t("Verkauf", "Floor"), "S", f.t("Kennzahlen", "Stats")),
                ("", "R", f.t("Kasse", "Register")),
                ("", "C", f.t("Katalog", "Catalog")),
                (f.t("Betrieb", "Shop"), "H", f.t("Öffnungszeiten", "Hours")),
                ("", "K", f.t("Kalender", "Calendar")),
                ("", "E", f.t("Personal", "Roster")),
                (f.t("Kasse", "Till"), "Z", f.t("Schicht", "Shift")),
                (f.t("Berichte", "Reports"), "G", f.t("Umsatz", "Sales")),
            ],
            1 => &[
                (f.t("Laden", "Shop"), "S", f.t("Kennzahlen", "Stats")),
                ("", "C", f.t("Katalog", "Catalog")),
                ("", "O", f.t("Bestellungen", "Orders")),
                (f.t("Versand", "Delivery"), "D", f.t("Zonen", "Shipping")),
                (f.t("Berichte", "Reports"), "G", f.t("Umsatz", "Sales")),
            ],
            2 => &[
                (f.t("Laden", "Shop"), "S", f.t("Kennzahlen", "Stats")),
                ("", "L", f.t("Angebote", "Listings")),
                ("", "O", f.t("Bestellungen", "Orders")),
                (
                    f.t("Nachschub", "Supply"),
                    "P",
                    f.t("Auffüllen", "Replenish"),
                ),
                (f.t("Berichte", "Reports"), "G", f.t("Umsatz", "Sales")),
            ],
            3 => &[
                (f.t("Abo", "Club"), "S", f.t("Kennzahlen", "Stats")),
                ("", "P", f.t("Pläne", "Plans")),
                ("", "U", f.t("Abonnenten", "Subscribers")),
                (f.t("Berichte", "Reports"), "G", f.t("Umsatz", "Sales")),
            ],
            4 => &[
                (f.t("Tresor", "Vault"), "S", f.t("Kennzahlen", "Stats")),
                ("", "C", f.t("Katalog", "Catalog")),
                ("", "O", f.t("Bestellungen", "Orders")),
                ("", "K", f.t("Lizenzen", "Licenses")),
                (f.t("Berichte", "Reports"), "G", f.t("Umsatz", "Sales")),
            ],
            5 => &[
                (f.t("Handel", "Trade"), "S", f.t("Kennzahlen", "Stats")),
                ("", "A", f.t("Konten", "Accounts")),
                ("", "C", f.t("Katalog", "Catalog")),
                ("", "O", f.t("Bestellungen", "Orders")),
                (f.t("Berichte", "Reports"), "G", f.t("Umsatz", "Sales")),
            ],
            6 => &[
                (
                    f.t("Abendkasse", "Box office"),
                    "S",
                    f.t("Kennzahlen", "Stats"),
                ),
                ("", "E", f.t("Veranstaltungen", "Events")),
                ("", "T", f.t("Tickets", "Tickets")),
                (f.t("Berichte", "Reports"), "G", f.t("Umsatz", "Sales")),
            ],
            _ => &[
                (f.t("Fuhrpark", "Fleet"), "S", f.t("Kennzahlen", "Stats")),
                ("", "G", f.t("Geräte", "Gear")),
                ("", "B", f.t("Buchungen", "Bookings")),
                (f.t("Berichte", "Reports"), "G", f.t("Umsatz", "Sales")),
            ],
        };
        let mut entries = Vec::new();
        for (group, icon, label) in spec {
            let mut entry = NavEntry::new(icon, *label);
            if !group.is_empty() {
                entry = entry.group(*group);
            }
            entries.push(entry);
        }
        entries
    }
    fn view(&mut self, ui: &mut Ui, view: usize, f: &mut Frame) {
        if view == sales_section(self.mode) {
            self.view_sales(ui, f);
            return;
        }
        match self.mode {
            0 => self.view_retail(ui, f, view),
            1 => desk::web(self, ui, f, view),
            2 => desk::amazon(self, ui, f, view),
            3 => desk::subscription(self, ui, f, view),
            4 => desk::digital(self, ui, f, view),
            5 => desk::wholesale(self, ui, f, view),
            6 => desk::tickets(self, ui, f, view),
            7 => desk::rental(self, ui, f, view),
            _ => {
                ui.label("Unknown shop");
            }
        }
    }
    fn menus(&self, f: &Frame) -> AppMenus {
        let modes = [
            f.t("Einzelhandel", "Retail"),
            "Web",
            "Amazon",
            f.t("Abo", "Subscription"),
            f.t("Digital", "Digital"),
            f.t("Großhandel", "Wholesale"),
            f.t("Tickets", "Ticket"),
            f.t("Verleih", "Rental"),
        ];
        let mode_items = modes
            .iter()
            .enumerate()
            .map(|(i, name)| Item::action(*name, &format!("shop.mode.{i}")).checked(self.mode == i))
            .collect();
        AppMenus {
            middle: vec![Menu::new(
                f.t("Laden", "Shop"),
                vec![
                    Item::sub(f.t("Modus", "Mode"), mode_items),
                    Item::sep(),
                    Item::action(f.t("Webshop öffnen", "Open webshop"), "shop.open_web"),
                ],
            )],
            ..AppMenus::default()
        }
    }
    fn menu_action(&mut self, id: &str, f: &mut Frame) {
        if let Some(rest) = id.strip_prefix("shop.mode.") {
            if let Ok(mode) = rest.parse::<usize>() {
                if mode < 8 {
                    self.mode = mode;
                    f.goto(0);
                }
            }
            return;
        }
        if id == "shop.open_web" {
            self.open_webshop(f);
        }
    }
    fn status_info(&self) -> String {
        let name = [
            "Retail",
            "Web",
            "Amazon",
            "Subscription",
            "Digital",
            "Wholesale",
            "Ticket",
            "Rental",
        ]
        .get(self.mode)
        .copied()
        .unwrap_or("Shop");
        format!("{name} shop")
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

fn sales_section(mode: usize) -> usize {
    match mode {
        0 => 7,
        3 | 6 | 7 => 3,
        _ => 4,
    }
}

fn weekday_name(weekday: i64, de: bool) -> &'static str {
    if de {
        match weekday {
            0 => "Montag",
            1 => "Dienstag",
            2 => "Mittwoch",
            3 => "Donnerstag",
            4 => "Freitag",
            5 => "Samstag",
            6 => "Sonntag",
            _ => "Unbekannt",
        }
    } else {
        match weekday {
            0 => "Monday",
            1 => "Tuesday",
            2 => "Wednesday",
            3 => "Thursday",
            4 => "Friday",
            5 => "Saturday",
            6 => "Sunday",
            _ => "Unknown",
        }
    }
}

fn hhmm(minutes: i64) -> String {
    let hour = minutes.div_euclid(60);
    let minute = minutes.rem_euclid(60);
    format!("{hour:02}:{minute:02}")
}
