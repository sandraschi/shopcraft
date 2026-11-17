use std::path::Path;

use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::error::{Result, WholesaleError};
use super::models::{
    CreateOrderInput, OrderLine, OrderLineInput, PortalExport, PurchaseOrder, QuoteLine,
    QuoteResult, TradeAccount, WholesaleProduct, WholesaleStatus,
};

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS trade_accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_code TEXT NOT NULL UNIQUE,
                company_name TEXT NOT NULL,
                country TEXT NOT NULL,
                vat_uid TEXT NOT NULL,
                vat_verified INTEGER NOT NULL DEFAULT 0,
                tier TEXT NOT NULL DEFAULT 'Standard',
                payment_terms TEXT NOT NULL DEFAULT 'Net30',
                credit_limit_cents INTEGER NOT NULL DEFAULT 0,
                balance_cents INTEGER NOT NULL DEFAULT 0,
                contact_email TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sku TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                moq INTEGER NOT NULL DEFAULT 1,
                pack_size INTEGER NOT NULL DEFAULT 1,
                list_price_cents INTEGER NOT NULL,
                tier_silver_pct INTEGER NOT NULL DEFAULT 10,
                tier_gold_pct INTEGER NOT NULL DEFAULT 18,
                stock INTEGER NOT NULL DEFAULT 0,
                image_url TEXT NOT NULL DEFAULT '',
                active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS purchase_orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_number TEXT NOT NULL UNIQUE,
                account_id INTEGER NOT NULL REFERENCES trade_accounts(id),
                po_reference TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Submitted',
                payment_terms TEXT NOT NULL,
                reverse_charge INTEGER NOT NULL DEFAULT 0,
                subtotal_cents INTEGER NOT NULL,
                vat_cents INTEGER NOT NULL,
                total_cents INTEGER NOT NULL,
                due_date TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS order_lines (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
                sku TEXT NOT NULL,
                product_name TEXT NOT NULL,
                qty INTEGER NOT NULL,
                unit_price_cents INTEGER NOT NULL,
                discount_pct INTEGER NOT NULL,
                line_total_cents INTEGER NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    pub fn seed_demo(&self) -> Result<(usize, usize)> {
        let now = Utc::now().to_rfc3339();
        let accounts = [
            (
                "B2B-1001",
                "Alpen Interior GmbH",
                "AT",
                "ATU68123456",
                true,
                "Gold",
                "Net30",
                2_500_000_i64,
                340_000_i64,
                "einkauf@alpen-interior.at",
            ),
            (
                "B2B-1002",
                "Nordlicht Handel AG",
                "DE",
                "DE298765432",
                true,
                "Silver",
                "Net60",
                1_800_000,
                125_000,
                "orders@nordlicht-handel.de",
            ),
            (
                "B2B-1003",
                "Studio Helvetia GmbH",
                "CH",
                "CHE-412.889.102",
                true,
                "Gold",
                "Net30",
                3_000_000,
                0,
                "procurement@helvetia-studio.ch",
            ),
            (
                "B2B-1004",
                "Kanaal Design B.V.",
                "NL",
                "NL854321987B01",
                false,
                "Standard",
                "Prepaid",
                500_000,
                0,
                "inkoop@kanaaldesign.nl",
            ),
        ];

        let mut acc_inserted = 0;
        for (code, name, country, uid, verified, tier, terms, limit_c, bal_c, email) in accounts {
            let changed = self.conn.execute(
                "INSERT OR IGNORE INTO trade_accounts
                 (account_code, company_name, country, vat_uid, vat_verified, tier, payment_terms, credit_limit_cents, balance_cents, contact_email, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![code, name, country, uid, verified as i64, tier, terms, limit_c, bal_c, email, now],
            )?;
            acc_inserted += changed;
        }

        let products = [
            (
                "WS-CER-01",
                "Stoneware Vessel 24cm (Case)",
                "Ceramics",
                12_i64,
                6_i64,
                4_200_i64,
                10_i64,
                18_i64,
                480_i64,
                "",
            ),
            (
                "WS-LGT-02",
                "Brushed Brass Desk Lamp",
                "Lighting",
                6,
                2,
                11_800,
                12,
                20,
                180,
                "",
            ),
            (
                "WS-TXT-03",
                "Merino Boucle Throw 140x200",
                "Textiles",
                10,
                5,
                6_900,
                10,
                18,
                320,
                "",
            ),
            (
                "WS-OAK-04",
                "Solid Oak Serving Tray",
                "Woodwork",
                24,
                6,
                2_850,
                10,
                16,
                600,
                "",
            ),
            (
                "WS-GLS-05",
                "Smoked Carafe 1.0L Set",
                "Glassware",
                12,
                4,
                3_600,
                8,
                15,
                400,
                "",
            ),
            (
                "WS-STN-06",
                "Travertine Plinth Low",
                "Objects",
                4,
                2,
                15_500,
                12,
                22,
                96,
                "",
            ),
        ];

        let mut prod_inserted = 0;
        for (sku, name, cat, moq, pack, price, silver, gold, stock, img) in products {
            let changed = self.conn.execute(
                "INSERT OR IGNORE INTO products
                 (sku, name, category, moq, pack_size, list_price_cents, tier_silver_pct, tier_gold_pct, stock, image_url, active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1)",
                params![sku, name, cat, moq, pack, price, silver, gold, stock, img],
            )?;
            prod_inserted += changed;
        }

        Ok((acc_inserted, prod_inserted))
    }

    pub fn list_accounts(&self) -> Result<Vec<TradeAccount>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_code, company_name, country, vat_uid, vat_verified, tier, payment_terms, credit_limit_cents, balance_cents, contact_email, created_at
             FROM trade_accounts ORDER BY account_code ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(TradeAccount {
                id: r.get(0)?,
                account_code: r.get(1)?,
                company_name: r.get(2)?,
                country: r.get(3)?,
                vat_uid: r.get(4)?,
                vat_verified: r.get::<_, i64>(5)? != 0,
                tier: r.get(6)?,
                payment_terms: r.get(7)?,
                credit_limit_cents: r.get(8)?,
                balance_cents: r.get(9)?,
                contact_email: r.get(10)?,
                created_at: r.get(11)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn get_account(&self, account_code: &str) -> Result<TradeAccount> {
        self.conn
            .query_row(
                "SELECT id, account_code, company_name, country, vat_uid, vat_verified, tier, payment_terms, credit_limit_cents, balance_cents, contact_email, created_at
                 FROM trade_accounts WHERE account_code = ?1",
                params![account_code],
                |r| {
                    Ok(TradeAccount {
                        id: r.get(0)?,
                        account_code: r.get(1)?,
                        company_name: r.get(2)?,
                        country: r.get(3)?,
                        vat_uid: r.get(4)?,
                        vat_verified: r.get::<_, i64>(5)? != 0,
                        tier: r.get(6)?,
                        payment_terms: r.get(7)?,
                        credit_limit_cents: r.get(8)?,
                        balance_cents: r.get(9)?,
                        contact_email: r.get(10)?,
                        created_at: r.get(11)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| WholesaleError::NotFound(format!("trade account '{account_code}'")))
    }

    pub fn list_products(&self, category: Option<&str>) -> Result<Vec<WholesaleProduct>> {
        let mut out = Vec::new();
        let sql = if category.is_some() {
            "SELECT id, sku, name, category, moq, pack_size, list_price_cents, tier_silver_pct, tier_gold_pct, stock, image_url, active
             FROM products WHERE active = 1 AND lower(category) = lower(?1) ORDER BY sku ASC"
        } else {
            "SELECT id, sku, name, category, moq, pack_size, list_price_cents, tier_silver_pct, tier_gold_pct, stock, image_url, active
             FROM products WHERE active = 1 ORDER BY sku ASC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let map_row = |r: &rusqlite::Row<'_>| -> rusqlite::Result<WholesaleProduct> {
            Ok(WholesaleProduct {
                id: r.get(0)?,
                sku: r.get(1)?,
                name: r.get(2)?,
                category: r.get(3)?,
                moq: r.get(4)?,
                pack_size: r.get(5)?,
                list_price_cents: r.get(6)?,
                tier_silver_pct: r.get(7)?,
                tier_gold_pct: r.get(8)?,
                stock: r.get(9)?,
                image_url: r.get(10)?,
                active: r.get::<_, i64>(11)? != 0,
            })
        };
        if let Some(cat) = category {
            for item in stmt.query_map(params![cat], map_row)? {
                out.push(item?);
            }
        } else {
            for item in stmt.query_map([], map_row)? {
                out.push(item?);
            }
        }
        Ok(out)
    }

    fn get_product(&self, sku: &str) -> Result<WholesaleProduct> {
        self.conn
            .query_row(
                "SELECT id, sku, name, category, moq, pack_size, list_price_cents, tier_silver_pct, tier_gold_pct, stock, image_url, active
                 FROM products WHERE sku = ?1",
                params![sku],
                |r| {
                    Ok(WholesaleProduct {
                        id: r.get(0)?,
                        sku: r.get(1)?,
                        name: r.get(2)?,
                        category: r.get(3)?,
                        moq: r.get(4)?,
                        pack_size: r.get(5)?,
                        list_price_cents: r.get(6)?,
                        tier_silver_pct: r.get(7)?,
                        tier_gold_pct: r.get(8)?,
                        stock: r.get(9)?,
                        image_url: r.get(10)?,
                        active: r.get::<_, i64>(11)? != 0,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| WholesaleError::NotFound(format!("SKU '{sku}'")))
    }

    pub fn quote(&self, account_code: &str, items: &[OrderLineInput]) -> Result<QuoteResult> {
        if items.is_empty() {
            return Err(WholesaleError::Invalid(
                "order must contain at least one item".into(),
            ));
        }
        let account = self.get_account(account_code)?;
        let mut lines = Vec::with_capacity(items.len());
        let mut subtotal_cents = 0_i64;

        for item in items {
            if item.qty <= 0 {
                return Err(WholesaleError::Invalid(format!(
                    "quantity for {} must be positive",
                    item.sku
                )));
            }
            let prod = self.get_product(&item.sku)?;
            if item.qty < prod.moq {
                return Err(WholesaleError::MoqViolation {
                    sku: prod.sku,
                    requested: item.qty,
                    moq: prod.moq,
                });
            }
            if prod.pack_size > 1 && item.qty % prod.pack_size != 0 {
                return Err(WholesaleError::PackSizeViolation {
                    sku: prod.sku,
                    requested: item.qty,
                    pack_size: prod.pack_size,
                });
            }
            if item.qty > prod.stock {
                return Err(WholesaleError::InsufficientStock {
                    sku: prod.sku,
                    requested: item.qty,
                    available: prod.stock,
                });
            }

            let base_discount = match account.tier.as_str() {
                "Gold" => prod.tier_gold_pct,
                "Silver" => prod.tier_silver_pct,
                _ => 0,
            };
            let volume_bonus = item.qty >= prod.moq * 3;
            let total_discount = (base_discount + if volume_bonus { 5 } else { 0 }).min(45);
            let unit_price_cents = (prod.list_price_cents * (100 - total_discount) + 50) / 100;
            let line_total_cents = unit_price_cents * item.qty;
            subtotal_cents += line_total_cents;

            lines.push(QuoteLine {
                sku: prod.sku,
                name: prod.name,
                qty: item.qty,
                list_price_cents: prod.list_price_cents,
                unit_price_cents,
                discount_pct: total_discount,
                volume_bonus,
                line_total_cents,
            });
        }

        // Domestic AT = 20% VAT. Cross-border verified UID or CH export = 0% reverse charge.
        // Unverified non-AT EU buyer = 20% VAT fallback.
        let reverse_charge = account.country != "AT" && account.vat_verified;
        let vat_rate_pct = if reverse_charge { 0 } else { 20 };
        let vat_cents = (subtotal_cents * vat_rate_pct + 50) / 100;
        let total_cents = subtotal_cents + vat_cents;

        let credit_remaining_cents = (account.credit_limit_cents - account.balance_cents).max(0);
        let credit_ok = account.payment_terms == "Prepaid"
            || (account.balance_cents + total_cents <= account.credit_limit_cents);

        Ok(QuoteResult {
            account_code: account.account_code,
            tier: account.tier,
            payment_terms: account.payment_terms,
            reverse_charge,
            vat_rate_pct,
            subtotal_cents,
            vat_cents,
            total_cents,
            credit_remaining_cents,
            credit_ok,
            lines,
        })
    }

    pub fn create_order(&mut self, input: &CreateOrderInput) -> Result<PurchaseOrder> {
        let quote = self.quote(&input.account_code, &input.items)?;
        let account = self.get_account(&input.account_code)?;
        if !quote.credit_ok {
            return Err(WholesaleError::CreditExceeded {
                account_code: account.account_code,
                balance_cents: account.balance_cents,
                order_cents: quote.total_cents,
                limit_cents: account.credit_limit_cents,
            });
        }

        let now = Utc::now();
        let terms_days = match account.payment_terms.as_str() {
            "Net60" => 60,
            "Net30" => 30,
            _ => 0,
        };
        let due_date = (now + Duration::days(terms_days))
            .format("%Y-%m-%d")
            .to_string();
        let created_at = now.to_rfc3339();
        let po_ref = if input.po_reference.trim().is_empty() {
            format!("PO-{}", now.format("%Y%m%d-%H%M%S"))
        } else {
            input.po_reference.trim().to_string()
        };

        let tx = self.conn.transaction()?;
        let next_seq: i64 = tx.query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM purchase_orders",
            [],
            |r| r.get(0),
        )?;
        let order_number = format!("WHL-{:05}", next_seq);

        tx.execute(
            "INSERT INTO purchase_orders
             (order_number, account_id, po_reference, status, payment_terms, reverse_charge, subtotal_cents, vat_cents, total_cents, due_date, created_at)
             VALUES (?1, ?2, ?3, 'Submitted', ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                order_number,
                account.id,
                po_ref,
                account.payment_terms,
                quote.reverse_charge as i64,
                quote.subtotal_cents,
                quote.vat_cents,
                quote.total_cents,
                due_date,
                created_at
            ],
        )?;
        let order_id = tx.last_insert_rowid();

        for line in &quote.lines {
            tx.execute(
                "INSERT INTO order_lines (order_id, sku, product_name, qty, unit_price_cents, discount_pct, line_total_cents)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    order_id,
                    line.sku,
                    line.name,
                    line.qty,
                    line.unit_price_cents,
                    line.discount_pct,
                    line.line_total_cents
                ],
            )?;
            tx.execute(
                "UPDATE products SET stock = stock - ?1 WHERE sku = ?2",
                params![line.qty, line.sku],
            )?;
        }

        if account.payment_terms != "Prepaid" {
            tx.execute(
                "UPDATE trade_accounts SET balance_cents = balance_cents + ?1 WHERE id = ?2",
                params![quote.total_cents, account.id],
            )?;
        }
        tx.commit()?;

        self.get_order(&order_number)
    }

    pub fn transition_order(
        &mut self,
        order_number: &str,
        new_status: &str,
    ) -> Result<PurchaseOrder> {
        let current = self.get_order(order_number)?;
        let valid = matches!(
            (current.status.as_str(), new_status),
            ("Submitted", "Approved")
                | ("Submitted", "Cancelled")
                | ("Approved", "Dispatched")
                | ("Approved", "Cancelled")
                | ("Dispatched", "Invoiced")
        );
        if !valid {
            return Err(WholesaleError::InvalidTransition {
                from: current.status,
                to: new_status.to_string(),
            });
        }

        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE purchase_orders SET status = ?1 WHERE order_number = ?2",
            params![new_status, order_number],
        )?;
        if new_status == "Cancelled" {
            for line in &current.lines {
                tx.execute(
                    "UPDATE products SET stock = stock + ?1 WHERE sku = ?2",
                    params![line.qty, line.sku],
                )?;
            }
            if current.payment_terms != "Prepaid" {
                tx.execute(
                    "UPDATE trade_accounts SET balance_cents = MAX(0, balance_cents - ?1) WHERE account_code = ?2",
                    params![current.total_cents, current.account_code],
                )?;
            }
        }
        tx.commit()?;
        self.get_order(order_number)
    }

    pub fn get_order(&self, order_number: &str) -> Result<PurchaseOrder> {
        let mut order = self
            .conn
            .query_row(
                "SELECT po.id, po.order_number, ta.account_code, ta.company_name, po.po_reference, po.status, po.payment_terms, po.reverse_charge, po.subtotal_cents, po.vat_cents, po.total_cents, po.due_date, po.created_at
                 FROM purchase_orders po
                 JOIN trade_accounts ta ON ta.id = po.account_id
                 WHERE po.order_number = ?1",
                params![order_number],
                |r| {
                    Ok(PurchaseOrder {
                        id: r.get(0)?,
                        order_number: r.get(1)?,
                        account_code: r.get(2)?,
                        company_name: r.get(3)?,
                        po_reference: r.get(4)?,
                        status: r.get(5)?,
                        payment_terms: r.get(6)?,
                        reverse_charge: r.get::<_, i64>(7)? != 0,
                        subtotal_cents: r.get(8)?,
                        vat_cents: r.get(9)?,
                        total_cents: r.get(10)?,
                        due_date: r.get(11)?,
                        created_at: r.get(12)?,
                        lines: Vec::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| WholesaleError::NotFound(format!("order '{order_number}'")))?;

        let mut stmt = self.conn.prepare(
            "SELECT id, order_id, sku, product_name, qty, unit_price_cents, discount_pct, line_total_cents
             FROM order_lines WHERE order_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![order.id], |r| {
            Ok(OrderLine {
                id: r.get(0)?,
                order_id: r.get(1)?,
                sku: r.get(2)?,
                product_name: r.get(3)?,
                qty: r.get(4)?,
                unit_price_cents: r.get(5)?,
                discount_pct: r.get(6)?,
                line_total_cents: r.get(7)?,
            })
        })?;
        order.lines = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(order)
    }

    pub fn list_orders(&self) -> Result<Vec<PurchaseOrder>> {
        let mut stmt = self
            .conn
            .prepare("SELECT order_number FROM purchase_orders ORDER BY id DESC")?;
        let nums: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut orders = Vec::with_capacity(nums.len());
        for num in nums {
            orders.push(self.get_order(&num)?);
        }
        Ok(orders)
    }

    pub fn status(&self) -> Result<WholesaleStatus> {
        let active_products: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM products WHERE active = 1", [], |r| {
                    r.get(0)
                })?;
        let total_stock_units: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(stock), 0) FROM products WHERE active = 1",
            [],
            |r| r.get(0),
        )?;
        let trade_accounts: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM trade_accounts", [], |r| r.get(0))?;
        let open_orders: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM purchase_orders WHERE status NOT IN ('Invoiced', 'Cancelled')",
            [],
            |r| r.get(0),
        )?;
        let total_credit_exposure_cents: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(balance_cents), 0) FROM trade_accounts",
            [],
            |r| r.get(0),
        )?;
        let total_revenue_cents: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_cents), 0) FROM purchase_orders WHERE status != 'Cancelled'",
            [],
            |r| r.get(0),
        )?;
        Ok(WholesaleStatus {
            active_products,
            total_stock_units,
            trade_accounts,
            open_orders,
            total_credit_exposure_cents,
            total_revenue_cents,
        })
    }

    pub fn export_portal(&self, portal_name: &str, tagline: &str) -> Result<PortalExport> {
        Ok(PortalExport {
            portal_name: portal_name.to_string(),
            tagline: tagline.to_string(),
            currency: "EUR".to_string(),
            generated_at: Utc::now().to_rfc3339(),
            accounts: self.list_accounts()?,
            products: self.list_products(None)?,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn seed_quote_and_reverse_charge_order() {
        let mut store = Store::open_in_memory().unwrap();
        let (accs, prods) = store.seed_demo().unwrap();
        assert_eq!(accs, 4);
        assert_eq!(prods, 6);

        // B2B-1002 is DE Silver with verified UID -> 0% reverse-charge VAT
        let q = store
            .quote(
                "B2B-1002",
                &[OrderLineInput {
                    sku: "WS-CER-01".into(),
                    qty: 36, // >= 12 * 3 -> 10% Silver + 5% volume = 15%
                }],
            )
            .unwrap();
        assert!(q.reverse_charge);
        assert_eq!(q.vat_rate_pct, 0);
        assert_eq!(q.lines[0].discount_pct, 15);
        assert_eq!(q.vat_cents, 0);

        let po = store
            .create_order(&CreateOrderInput {
                account_code: "B2B-1002".into(),
                po_reference: "DE-2026-88".into(),
                items: vec![OrderLineInput {
                    sku: "WS-CER-01".into(),
                    qty: 36,
                }],
            })
            .unwrap();
        assert_eq!(po.status, "Submitted");
        assert!(po.reverse_charge);

        let cancelled = store
            .transition_order(&po.order_number, "Cancelled")
            .unwrap();
        assert_eq!(cancelled.status, "Cancelled");
    }

    #[test]
    fn enforces_moq_and_pack_size() {
        let store = Store::open_in_memory().unwrap();
        store.seed_demo().unwrap();

        let err_moq = store.quote(
            "B2B-1001",
            &[OrderLineInput {
                sku: "WS-CER-01".into(),
                qty: 6,
            }],
        );
        assert!(matches!(err_moq, Err(WholesaleError::MoqViolation { .. })));

        let err_pack = store.quote(
            "B2B-1001",
            &[OrderLineInput {
                sku: "WS-CER-01".into(),
                qty: 13,
            }],
        );
        assert!(matches!(
            err_pack,
            Err(WholesaleError::PackSizeViolation { .. })
        ));
    }
}
