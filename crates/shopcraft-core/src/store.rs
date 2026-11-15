use std::path::{Path, PathBuf};

use chrono::Utc;
use craft_biz::money::Money;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{PosError, Result};
use crate::models::{CheckoutInput, PosItem, PosStatus, Receipt, ReceiptLine, Shift, ZBon};

pub struct PosStore {
    conn: Connection,
    db_path: PathBuf,
}

impl PosStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(&db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self { conn, db_path };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self {
            conn,
            db_path: PathBuf::from(":memory:"),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS items (
                sku TEXT PRIMARY KEY,
                barcode TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                category TEXT NOT NULL,
                price_cents INTEGER NOT NULL CHECK(price_cents >= 0),
                vat_rate_bps INTEGER NOT NULL DEFAULT 2000,
                stock_qty INTEGER NOT NULL DEFAULT 0 CHECK(stock_qty >= 0),
                active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS shifts (
                shift_id TEXT PRIMARY KEY,
                clerk_name TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('OPEN', 'CLOSED')),
                opening_float_cents INTEGER NOT NULL CHECK(opening_float_cents >= 0),
                expected_cash_cents INTEGER NOT NULL,
                counted_cash_cents INTEGER,
                cash_diff_cents INTEGER,
                opened_at TEXT NOT NULL,
                closed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS receipts (
                receipt_no TEXT PRIMARY KEY,
                shift_id TEXT NOT NULL REFERENCES shifts(shift_id),
                kind TEXT NOT NULL CHECK(kind IN ('SALE', 'STORNO')),
                storno_of TEXT,
                stornoed INTEGER NOT NULL DEFAULT 0,
                subtotal_cents INTEGER NOT NULL,
                discount_cents INTEGER NOT NULL,
                vat_20_cents INTEGER NOT NULL,
                vat_10_cents INTEGER NOT NULL,
                total_cents INTEGER NOT NULL,
                cash_paid_cents INTEGER NOT NULL,
                card_paid_cents INTEGER NOT NULL,
                voucher_paid_cents INTEGER NOT NULL,
                change_cents INTEGER NOT NULL,
                rksv_sig TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS receipt_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                receipt_no TEXT NOT NULL REFERENCES receipts(receipt_no),
                sku TEXT NOT NULL REFERENCES items(sku),
                title TEXT NOT NULL,
                qty INTEGER NOT NULL,
                unit_price_cents INTEGER NOT NULL,
                line_total_cents INTEGER NOT NULL,
                vat_rate_bps INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS z_bons (
                zbon_no TEXT PRIMARY KEY,
                shift_id TEXT NOT NULL UNIQUE REFERENCES shifts(shift_id),
                clerk_name TEXT NOT NULL,
                receipt_count INTEGER NOT NULL,
                storno_count INTEGER NOT NULL,
                gross_total_cents INTEGER NOT NULL,
                cash_net_cents INTEGER NOT NULL,
                card_total_cents INTEGER NOT NULL,
                voucher_total_cents INTEGER NOT NULL,
                vat_20_cents INTEGER NOT NULL,
                vat_10_cents INTEGER NOT NULL,
                opening_float_cents INTEGER NOT NULL,
                expected_cash_cents INTEGER NOT NULL,
                counted_cash_cents INTEGER NOT NULL,
                cash_diff_cents INTEGER NOT NULL,
                closed_at TEXT NOT NULL
            );

            CREATE TRIGGER IF NOT EXISTS receipt_items_immutable_upd
            BEFORE UPDATE ON receipt_items
            BEGIN
                SELECT RAISE(ABORT, 'receipt_items is immutable');
            END;

            CREATE TRIGGER IF NOT EXISTS receipt_items_immutable_del
            BEFORE DELETE ON receipt_items
            BEGIN
                SELECT RAISE(ABORT, 'receipt_items is immutable');
            END;

            CREATE TRIGGER IF NOT EXISTS z_bons_immutable_upd
            BEFORE UPDATE ON z_bons
            BEGIN
                SELECT RAISE(ABORT, 'z_bons is immutable');
            END;

            CREATE TRIGGER IF NOT EXISTS z_bons_immutable_del
            BEFORE DELETE ON z_bons
            BEGIN
                SELECT RAISE(ABORT, 'z_bons is immutable');
            END;
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_item(&self, item: &PosItem) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO items (sku, barcode, title, category, price_cents, vat_rate_bps, stock_qty, active)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(sku) DO UPDATE SET
                barcode = excluded.barcode,
                title = excluded.title,
                category = excluded.category,
                price_cents = excluded.price_cents,
                vat_rate_bps = excluded.vat_rate_bps,
                stock_qty = excluded.stock_qty,
                active = excluded.active
            "#,
            params![
                item.sku,
                item.barcode,
                item.title,
                item.category,
                item.price_cents,
                item.vat_rate_bps,
                item.stock_qty,
                i64::from(item.active)
            ],
        )?;
        Ok(())
    }

    pub fn list_items(&self, category: Option<&str>) -> Result<Vec<PosItem>> {
        let mut sql = String::from(
            "SELECT sku, barcode, title, category, price_cents, vat_rate_bps, stock_qty, active FROM items WHERE active = 1",
        );
        if category.is_some() {
            sql.push_str(" AND lower(category) = lower(?1)");
        }
        sql.push_str(" ORDER BY title ASC");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = if let Some(cat) = category {
            stmt.query_map(params![cat], Self::map_item)?
        } else {
            stmt.query_map([], Self::map_item)?
        };
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<PosItem> {
        Ok(PosItem {
            sku: row.get(0)?,
            barcode: row.get(1)?,
            title: row.get(2)?,
            category: row.get(3)?,
            price_cents: row.get(4)?,
            vat_rate_bps: row.get(5)?,
            stock_qty: row.get(6)?,
            active: row.get::<_, i64>(7)? != 0,
        })
    }

    pub fn open_shift(&self, clerk_name: &str, opening_float_cents: i64) -> Result<Shift> {
        if clerk_name.trim().is_empty() {
            return Err(PosError::Invalid("clerk_name cannot be empty".into()));
        }
        if opening_float_cents < 0 {
            return Err(PosError::Invalid(
                "opening_float_cents cannot be negative".into(),
            ));
        }
        let open_existing: Option<String> = self
            .conn
            .query_row(
                "SELECT shift_id FROM shifts WHERE status = 'OPEN' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = open_existing {
            return Err(PosError::Invalid(format!("shift {id} is already open")));
        }

        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM shifts", [], |r| r.get(0))?;
        let now = Utc::now();
        let shift_id = format!("SHF-{}-{:03}", now.format("%Y%m%d"), count + 1);
        let opened_at = now.to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO shifts (shift_id, clerk_name, status, opening_float_cents, expected_cash_cents, opened_at)
            VALUES (?1, ?2, 'OPEN', ?3, ?3, ?4)
            "#,
            params![shift_id, clerk_name.trim(), opening_float_cents, opened_at],
        )?;

        self.get_shift(&shift_id)
    }

    pub fn get_shift(&self, shift_id: &str) -> Result<Shift> {
        self.conn
            .query_row(
                r#"
                SELECT shift_id, clerk_name, status, opening_float_cents, expected_cash_cents,
                       counted_cash_cents, cash_diff_cents, opened_at, closed_at
                FROM shifts WHERE shift_id = ?1
                "#,
                params![shift_id],
                |r| {
                    Ok(Shift {
                        shift_id: r.get(0)?,
                        clerk_name: r.get(1)?,
                        status: r.get(2)?,
                        opening_float_cents: r.get(3)?,
                        expected_cash_cents: r.get(4)?,
                        counted_cash_cents: r.get(5)?,
                        cash_diff_cents: r.get(6)?,
                        opened_at: r.get(7)?,
                        closed_at: r.get(8)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| PosError::NotFound(format!("shift {shift_id}")))
    }

    pub fn checkout(&mut self, input: &CheckoutInput) -> Result<Receipt> {
        if input.items.is_empty() {
            return Err(PosError::Invalid(
                "checkout requires at least one item".into(),
            ));
        }
        if input.cash_cents < 0 || input.card_cents < 0 || input.voucher_cents < 0 {
            return Err(PosError::Invalid(
                "tender amounts cannot be negative".into(),
            ));
        }

        let tx = self.conn.transaction()?;
        let shift_status: Option<String> = tx
            .query_row(
                "SELECT status FROM shifts WHERE shift_id = ?1",
                params![input.shift_id],
                |r| r.get(0),
            )
            .optional()?;
        match shift_status.as_deref() {
            Some("OPEN") => {}
            Some(_) => {
                return Err(PosError::Invalid(format!(
                    "shift {} is closed",
                    input.shift_id
                )))
            }
            None => return Err(PosError::NoOpenShift),
        }

        let mut subtotal = Money::ZERO;
        let mut vat_20 = Money::ZERO;
        let mut vat_10 = Money::ZERO;
        let mut lines = Vec::with_capacity(input.items.len());
        let discount_bps = input.discount_bps.unwrap_or(0).clamp(0, 10_000);

        for line in &input.items {
            if line.qty <= 0 {
                return Err(PosError::Invalid(format!("invalid qty for {}", line.code)));
            }
            let item: (String, String, i64, i64, i64, i64) = tx
                .query_row(
                    "SELECT sku, title, price_cents, vat_rate_bps, stock_qty, active FROM items WHERE sku = ?1 OR barcode = ?1",
                    params![line.code],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
                )
                .optional()?
                .ok_or_else(|| PosError::NotFound(format!("item {}", line.code)))?;

            let (sku, title, price_cents, vat_rate_bps, stock_qty, active) = item;
            if active == 0 {
                return Err(PosError::Invalid(format!("item {sku} is inactive")));
            }
            if stock_qty < line.qty {
                return Err(PosError::InsufficientStock {
                    sku,
                    requested: line.qty,
                    available: stock_qty,
                });
            }

            tx.execute(
                "UPDATE items SET stock_qty = stock_qty - ?1 WHERE sku = ?2",
                params![line.qty, sku],
            )?;

            let line_total = price_cents * line.qty;
            subtotal = subtotal.checked_add(Money::from_cents(line_total))?;

            let discounted_line = (line_total * (10_000 - discount_bps) + 5_000) / 10_000;
            let line_vat = (discounted_line * vat_rate_bps + (10_000 + vat_rate_bps) / 2)
                / (10_000 + vat_rate_bps);
            if vat_rate_bps == 1000 {
                vat_10 = vat_10.checked_add(Money::from_cents(line_vat))?;
            } else {
                vat_20 = vat_20.checked_add(Money::from_cents(line_vat))?;
            }

            lines.push(ReceiptLine {
                sku,
                title,
                qty: line.qty,
                unit_price_cents: price_cents,
                line_total_cents: line_total,
                vat_rate_bps,
            });
        }

        let discount_cents = (subtotal.cents() * discount_bps + 5_000) / 10_000;
        let total_cents = (subtotal.cents() - discount_cents).max(0);
        let tendered = input.cash_cents + input.card_cents + input.voucher_cents;
        if tendered < total_cents {
            return Err(PosError::InsufficientTender {
                total_cents,
                tendered_cents: tendered,
            });
        }
        let change_cents = tendered - total_cents;
        let net_cash_in_drawer = input.cash_cents - change_cents;

        let count: i64 = tx.query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))?;
        let now = Utc::now();
        let receipt_no = format!("POS-{}-{:04}", now.format("%Y%m%d"), count + 1);
        let created_at = now.to_rfc3339();
        let rksv_sig = format!(
            "RKSV-{:08X}-{}",
            (total_cents as u64).wrapping_mul(2654435761) & 0xFFFF_FFFF,
            count + 1
        );

        tx.execute(
            r#"
            INSERT INTO receipts (
                receipt_no, shift_id, kind, storno_of, stornoed, subtotal_cents, discount_cents,
                vat_20_cents, vat_10_cents, total_cents, cash_paid_cents, card_paid_cents,
                voucher_paid_cents, change_cents, rksv_sig, created_at
            ) VALUES (?1, ?2, 'SALE', NULL, 0, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                receipt_no,
                input.shift_id,
                subtotal.cents(),
                discount_cents,
                vat_20.cents(),
                vat_10.cents(),
                total_cents,
                input.cash_cents,
                input.card_cents,
                input.voucher_cents,
                change_cents,
                rksv_sig,
                created_at
            ],
        )?;

        for l in &lines {
            tx.execute(
                r#"
                INSERT INTO receipt_items (receipt_no, sku, title, qty, unit_price_cents, line_total_cents, vat_rate_bps)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    receipt_no,
                    l.sku,
                    l.title,
                    l.qty,
                    l.unit_price_cents,
                    l.line_total_cents,
                    l.vat_rate_bps
                ],
            )?;
        }

        tx.execute(
            "UPDATE shifts SET expected_cash_cents = expected_cash_cents + ?1 WHERE shift_id = ?2",
            params![net_cash_in_drawer, input.shift_id],
        )?;

        tx.commit()?;
        self.get_receipt(&receipt_no)
    }

    pub fn storno_receipt(&mut self, receipt_no: &str) -> Result<Receipt> {
        let orig = self.get_receipt(receipt_no)?;
        if orig.kind == "STORNO" {
            return Err(PosError::Invalid("cannot storno a storno receipt".into()));
        }
        if orig.stornoed {
            return Err(PosError::AlreadyStornoed(receipt_no.into()));
        }

        let tx = self.conn.transaction()?;
        let open_shift: Option<String> = tx
            .query_row(
                "SELECT shift_id FROM shifts WHERE status = 'OPEN' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let shift_id = open_shift.ok_or(PosError::NoOpenShift)?;

        tx.execute(
            "UPDATE receipts SET stornoed = 1 WHERE receipt_no = ?1",
            params![receipt_no],
        )?;

        for line in &orig.lines {
            tx.execute(
                "UPDATE items SET stock_qty = stock_qty + ?1 WHERE sku = ?2",
                params![line.qty, line.sku],
            )?;
        }

        let count: i64 = tx.query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))?;
        let now = Utc::now();
        let storno_no = format!("POS-{}-{:04}", now.format("%Y%m%d"), count + 1);
        let created_at = now.to_rfc3339();
        let rksv_sig = format!("RKSV-STORNO-{:04}", count + 1);

        let net_cash_orig = orig.cash_paid_cents - orig.change_cents;
        tx.execute(
            r#"
            INSERT INTO receipts (
                receipt_no, shift_id, kind, storno_of, stornoed, subtotal_cents, discount_cents,
                vat_20_cents, vat_10_cents, total_cents, cash_paid_cents, card_paid_cents,
                voucher_paid_cents, change_cents, rksv_sig, created_at
            ) VALUES (?1, ?2, 'STORNO', ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12, ?13)
            "#,
            params![
                storno_no,
                shift_id,
                receipt_no,
                -orig.subtotal_cents,
                -orig.discount_cents,
                -orig.vat_20_cents,
                -orig.vat_10_cents,
                -orig.total_cents,
                -net_cash_orig,
                -orig.card_paid_cents,
                -orig.voucher_paid_cents,
                rksv_sig,
                created_at
            ],
        )?;

        for line in &orig.lines {
            tx.execute(
                r#"
                INSERT INTO receipt_items (receipt_no, sku, title, qty, unit_price_cents, line_total_cents, vat_rate_bps)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    storno_no,
                    line.sku,
                    line.title,
                    -line.qty,
                    line.unit_price_cents,
                    -line.line_total_cents,
                    line.vat_rate_bps
                ],
            )?;
        }

        tx.execute(
            "UPDATE shifts SET expected_cash_cents = expected_cash_cents - ?1 WHERE shift_id = ?2",
            params![net_cash_orig, shift_id],
        )?;

        tx.commit()?;
        self.get_receipt(&storno_no)
    }

    pub fn close_shift(&mut self, shift_id: &str, counted_cash_cents: i64) -> Result<ZBon> {
        let shift = self.get_shift(shift_id)?;
        if shift.status == "CLOSED" {
            return Err(PosError::Invalid(format!(
                "shift {shift_id} is already closed"
            )));
        }

        let tx = self.conn.transaction()?;
        let (
            receipt_count,
            storno_count,
            gross_total,
            cash_net,
            card_total,
            voucher_total,
            vat_20,
            vat_10,
        ): (i64, i64, i64, i64, i64, i64, i64, i64) = tx.query_row(
            r#"
            SELECT
                COUNT(CASE WHEN kind = 'SALE' THEN 1 END),
                COUNT(CASE WHEN kind = 'STORNO' THEN 1 END),
                COALESCE(SUM(total_cents), 0),
                COALESCE(SUM(cash_paid_cents - change_cents), 0),
                COALESCE(SUM(card_paid_cents), 0),
                COALESCE(SUM(voucher_paid_cents), 0),
                COALESCE(SUM(vat_20_cents), 0),
                COALESCE(SUM(vat_10_cents), 0)
            FROM receipts WHERE shift_id = ?1
            "#,
            params![shift_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            },
        )?;

        let cash_diff = counted_cash_cents - shift.expected_cash_cents;
        let now = Utc::now();
        let closed_at = now.to_rfc3339();
        let z_count: i64 = tx.query_row("SELECT COUNT(*) FROM z_bons", [], |r| r.get(0))?;
        let zbon_no = format!("ZBON-{}-{:04}", now.format("%Y%m%d"), z_count + 1);

        tx.execute(
            r#"
            UPDATE shifts
            SET status = 'CLOSED', counted_cash_cents = ?1, cash_diff_cents = ?2, closed_at = ?3
            WHERE shift_id = ?4
            "#,
            params![counted_cash_cents, cash_diff, closed_at, shift_id],
        )?;

        tx.execute(
            r#"
            INSERT INTO z_bons (
                zbon_no, shift_id, clerk_name, receipt_count, storno_count, gross_total_cents,
                cash_net_cents, card_total_cents, voucher_total_cents, vat_20_cents, vat_10_cents,
                opening_float_cents, expected_cash_cents, counted_cash_cents, cash_diff_cents, closed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            "#,
            params![
                zbon_no,
                shift_id,
                shift.clerk_name,
                receipt_count,
                storno_count,
                gross_total,
                cash_net,
                card_total,
                voucher_total,
                vat_20,
                vat_10,
                shift.opening_float_cents,
                shift.expected_cash_cents,
                counted_cash_cents,
                cash_diff,
                closed_at
            ],
        )?;

        tx.commit()?;

        Ok(ZBon {
            zbon_no,
            shift_id: shift_id.into(),
            clerk_name: shift.clerk_name,
            receipt_count,
            storno_count,
            gross_total_cents: gross_total,
            cash_net_cents: cash_net,
            card_total_cents: card_total,
            voucher_total_cents: voucher_total,
            vat_20_cents: vat_20,
            vat_10_cents: vat_10,
            opening_float_cents: shift.opening_float_cents,
            expected_cash_cents: shift.expected_cash_cents,
            counted_cash_cents,
            cash_diff_cents: cash_diff,
            closed_at,
        })
    }

    pub fn get_receipt(&self, receipt_no: &str) -> Result<Receipt> {
        let mut r: Receipt = self
            .conn
            .query_row(
                r#"
                SELECT receipt_no, shift_id, kind, storno_of, stornoed, subtotal_cents, discount_cents,
                       vat_20_cents, vat_10_cents, total_cents, cash_paid_cents, card_paid_cents,
                       voucher_paid_cents, change_cents, rksv_sig, created_at
                FROM receipts WHERE receipt_no = ?1
                "#,
                params![receipt_no],
                |row| {
                    Ok(Receipt {
                        receipt_no: row.get(0)?,
                        shift_id: row.get(1)?,
                        kind: row.get(2)?,
                        storno_of: row.get(3)?,
                        stornoed: row.get::<_, i64>(4)? != 0,
                        subtotal_cents: row.get(5)?,
                        discount_cents: row.get(6)?,
                        vat_20_cents: row.get(7)?,
                        vat_10_cents: row.get(8)?,
                        total_cents: row.get(9)?,
                        cash_paid_cents: row.get(10)?,
                        card_paid_cents: row.get(11)?,
                        voucher_paid_cents: row.get(12)?,
                        change_cents: row.get(13)?,
                        rksv_sig: row.get(14)?,
                        created_at: row.get(15)?,
                        lines: Vec::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| PosError::NotFound(format!("receipt {receipt_no}")))?;

        let mut stmt = self.conn.prepare(
            "SELECT sku, title, qty, unit_price_cents, line_total_cents, vat_rate_bps FROM receipt_items WHERE receipt_no = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![receipt_no], |row| {
            Ok(ReceiptLine {
                sku: row.get(0)?,
                title: row.get(1)?,
                qty: row.get(2)?,
                unit_price_cents: row.get(3)?,
                line_total_cents: row.get(4)?,
                vat_rate_bps: row.get(5)?,
            })
        })?;
        for row in rows {
            r.lines.push(row?);
        }
        Ok(r)
    }

    pub fn format_receipt_text(&self, receipt_no: &str) -> Result<String> {
        let r = self.get_receipt(receipt_no)?;
        let fmt_eur = |c: i64| -> String {
            let sign = if c < 0 { "-" } else { "" };
            let abs = c.abs();
            format!("{sign}EUR {}.{:02}", abs / 100, abs % 100)
        };
        let mut out = String::new();
        out.push_str("==========================================\n");
        out.push_str("            ATELIER NORD VIENNA           \n");
        out.push_str("       Neubaugasse 12, 1070 Wien          \n");
        out.push_str("==========================================\n");
        out.push_str(&format!("Receipt: {:<20} [{}]\n", r.receipt_no, r.kind));
        out.push_str(&format!("Shift:   {}\n", r.shift_id));
        if let Some(orig) = &r.storno_of {
            out.push_str(&format!("Storno of: {orig}\n"));
        }
        out.push_str("------------------------------------------\n");
        for line in &r.lines {
            out.push_str(&format!(
                "{:<24} {:>3}x {:>12}\n",
                line.title.chars().take(24).collect::<String>(),
                line.qty,
                fmt_eur(line.line_total_cents)
            ));
        }
        out.push_str("------------------------------------------\n");
        if r.discount_cents != 0 {
            out.push_str(&format!(
                "{:<24} {:>17}\n",
                "Discount",
                fmt_eur(-r.discount_cents)
            ));
        }
        out.push_str(&format!("{:<24} {:>17}\n", "TOTAL", fmt_eur(r.total_cents)));
        out.push_str(&format!(
            "{:<24} {:>17}\n",
            "  incl. VAT 20%",
            fmt_eur(r.vat_20_cents)
        ));
        if r.vat_10_cents != 0 {
            out.push_str(&format!(
                "{:<24} {:>17}\n",
                "  incl. VAT 10%",
                fmt_eur(r.vat_10_cents)
            ));
        }
        out.push_str("------------------------------------------\n");
        out.push_str(&format!(
            "{:<24} {:>17}\n",
            "Cash",
            fmt_eur(r.cash_paid_cents)
        ));
        out.push_str(&format!(
            "{:<24} {:>17}\n",
            "Card",
            fmt_eur(r.card_paid_cents)
        ));
        if r.voucher_paid_cents != 0 {
            out.push_str(&format!(
                "{:<24} {:>17}\n",
                "Voucher",
                fmt_eur(r.voucher_paid_cents)
            ));
        }
        if r.change_cents != 0 {
            out.push_str(&format!(
                "{:<24} {:>17}\n",
                "Change",
                fmt_eur(r.change_cents)
            ));
        }
        out.push_str("------------------------------------------\n");
        out.push_str(&format!("Sig: {}\n", r.rksv_sig));
        out.push_str("==========================================\n");
        Ok(out)
    }

    pub fn status(&self) -> Result<PosStatus> {
        let active_items: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM items WHERE active = 1", [], |r| {
                    r.get(0)
                })?;
        let open_shift: Option<String> = self
            .conn
            .query_row(
                "SELECT shift_id FROM shifts WHERE status = 'OPEN' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let total_receipts: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM receipts", [], |r| r.get(0))?;
        let gross_sales_cents: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_cents), 0) FROM receipts",
            [],
            |r| r.get(0),
        )?;
        Ok(PosStatus {
            db_path: self.db_path.display().to_string(),
            active_items,
            open_shift,
            total_receipts,
            gross_sales_cents,
        })
    }

    pub fn seed_demo(&self) -> Result<usize> {
        let items = vec![
            PosItem {
                sku: "CER-VASE-01".into(),
                barcode: "9001234000018".into(),
                title: "Fluted Stoneware Vase".into(),
                category: "Ceramics".into(),
                price_cents: 6800,
                vat_rate_bps: 2000,
                stock_qty: 18,
                active: true,
            },
            PosItem {
                sku: "LGT-LAMP-02".into(),
                barcode: "9001234000025".into(),
                title: "Atelier Desk Lamp".into(),
                category: "Lighting".into(),
                price_cents: 18900,
                vat_rate_bps: 2000,
                stock_qty: 7,
                active: true,
            },
            PosItem {
                sku: "TXT-THRW-03".into(),
                barcode: "9001234000032".into(),
                title: "Alpine Merino Throw".into(),
                category: "Textiles".into(),
                price_cents: 12400,
                vat_rate_bps: 2000,
                stock_qty: 14,
                active: true,
            },
            PosItem {
                sku: "WRK-TRAY-04".into(),
                barcode: "9001234000049".into(),
                title: "Smoked Oak Valet Tray".into(),
                category: "Workspace".into(),
                price_cents: 4900,
                vat_rate_bps: 2000,
                stock_qty: 32,
                active: true,
            },
            PosItem {
                sku: "CER-MUG-05".into(),
                barcode: "9001234000056".into(),
                title: "Espresso Tumbler Pair".into(),
                category: "Ceramics".into(),
                price_cents: 3600,
                vat_rate_bps: 2000,
                stock_qty: 25,
                active: true,
            },
            PosItem {
                sku: "WRK-NOTE-06".into(),
                barcode: "9001234000063".into(),
                title: "Linen Studio Notebook".into(),
                category: "Workspace".into(),
                price_cents: 2400,
                vat_rate_bps: 1000,
                stock_qty: 44,
                active: true,
            },
        ];
        for item in &items {
            self.upsert_item(item)?;
        }
        Ok(items.len())
    }
}
