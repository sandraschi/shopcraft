use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::error::{AmazonError, Result};
use super::models::{
    AmazonOrder, AmazonStatus, Listing, RecordOrderInput, ReplenishmentAlert, RepriceResult,
    UnitEconomics,
};

pub struct AmazonStore {
    conn: Connection,
    db_path: PathBuf,
}

impl AmazonStore {
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
            CREATE TABLE IF NOT EXISTS listings (
                asin TEXT PRIMARY KEY,
                sku TEXT NOT NULL UNIQUE,
                marketplace TEXT NOT NULL,
                title TEXT NOT NULL,
                fulfillment TEXT NOT NULL CHECK(fulfillment IN ('FBA', 'FBM')),
                price_cents INTEGER NOT NULL CHECK(price_cents >= 0),
                cogs_cents INTEGER NOT NULL CHECK(cogs_cents >= 0),
                referral_fee_bps INTEGER NOT NULL CHECK(referral_fee_bps >= 0),
                fba_fee_cents INTEGER NOT NULL CHECK(fba_fee_cents >= 0),
                ppc_per_unit_cents INTEGER NOT NULL CHECK(ppc_per_unit_cents >= 0),
                vat_rate_bps INTEGER NOT NULL DEFAULT 2000,
                stock_fba INTEGER NOT NULL DEFAULT 0 CHECK(stock_fba >= 0),
                stock_fbm INTEGER NOT NULL DEFAULT 0 CHECK(stock_fbm >= 0),
                daily_velocity_milli INTEGER NOT NULL DEFAULT 0,
                min_price_cents INTEGER NOT NULL,
                max_price_cents INTEGER NOT NULL,
                buy_box_won INTEGER NOT NULL DEFAULT 1,
                active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS amazon_orders (
                amazon_order_id TEXT PRIMARY KEY,
                marketplace TEXT NOT NULL,
                fulfillment TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('PENDING', 'UNSHIPPED', 'SHIPPED', 'CANCELLED')),
                asin TEXT NOT NULL REFERENCES listings(asin),
                sku TEXT NOT NULL,
                qty INTEGER NOT NULL CHECK(qty > 0),
                gross_cents INTEGER NOT NULL,
                referral_fee_cents INTEGER NOT NULL,
                fba_fee_cents INTEGER NOT NULL,
                vat_cents INTEGER NOT NULL,
                net_profit_cents INTEGER NOT NULL,
                invoice_uploaded INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settlements (
                settlement_id TEXT PRIMARY KEY,
                marketplace TEXT NOT NULL,
                period_start TEXT NOT NULL,
                period_end TEXT NOT NULL,
                gross_sales_cents INTEGER NOT NULL,
                fees_cents INTEGER NOT NULL,
                refunds_cents INTEGER NOT NULL,
                net_payout_cents INTEGER NOT NULL,
                posted_at TEXT NOT NULL
            );

            CREATE TRIGGER IF NOT EXISTS settlements_immutable_upd
            BEFORE UPDATE ON settlements
            BEGIN
                SELECT RAISE(ABORT, 'settlements is immutable');
            END;

            CREATE TRIGGER IF NOT EXISTS settlements_immutable_del
            BEFORE DELETE ON settlements
            BEGIN
                SELECT RAISE(ABORT, 'settlements is immutable');
            END;
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_listing(&self, l: &Listing) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO listings (
                asin, sku, marketplace, title, fulfillment, price_cents, cogs_cents,
                referral_fee_bps, fba_fee_cents, ppc_per_unit_cents, vat_rate_bps,
                stock_fba, stock_fbm, daily_velocity_milli, min_price_cents, max_price_cents,
                buy_box_won, active
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(asin) DO UPDATE SET
                sku = excluded.sku,
                marketplace = excluded.marketplace,
                title = excluded.title,
                fulfillment = excluded.fulfillment,
                price_cents = excluded.price_cents,
                cogs_cents = excluded.cogs_cents,
                referral_fee_bps = excluded.referral_fee_bps,
                fba_fee_cents = excluded.fba_fee_cents,
                ppc_per_unit_cents = excluded.ppc_per_unit_cents,
                vat_rate_bps = excluded.vat_rate_bps,
                stock_fba = excluded.stock_fba,
                stock_fbm = excluded.stock_fbm,
                daily_velocity_milli = excluded.daily_velocity_milli,
                min_price_cents = excluded.min_price_cents,
                max_price_cents = excluded.max_price_cents,
                buy_box_won = excluded.buy_box_won,
                active = excluded.active
            "#,
            params![
                l.asin,
                l.sku,
                l.marketplace,
                l.title,
                l.fulfillment,
                l.price_cents,
                l.cogs_cents,
                l.referral_fee_bps,
                l.fba_fee_cents,
                l.ppc_per_unit_cents,
                l.vat_rate_bps,
                l.stock_fba,
                l.stock_fbm,
                l.daily_velocity_milli,
                l.min_price_cents,
                l.max_price_cents,
                i64::from(l.buy_box_won),
                i64::from(l.active)
            ],
        )?;
        Ok(())
    }

    pub fn get_listing(&self, asin: &str) -> Result<Listing> {
        self.conn
            .query_row(
                r#"
                SELECT asin, sku, marketplace, title, fulfillment, price_cents, cogs_cents,
                       referral_fee_bps, fba_fee_cents, ppc_per_unit_cents, vat_rate_bps,
                       stock_fba, stock_fbm, daily_velocity_milli, min_price_cents, max_price_cents,
                       buy_box_won, active
                FROM listings WHERE asin = ?1
                "#,
                params![asin],
                Self::map_listing,
            )
            .optional()?
            .ok_or_else(|| AmazonError::NotFound(format!("listing {asin}")))
    }

    pub fn list_listings(&self, marketplace: Option<&str>) -> Result<Vec<Listing>> {
        let mut sql = String::from(
            "SELECT asin, sku, marketplace, title, fulfillment, price_cents, cogs_cents,
                    referral_fee_bps, fba_fee_cents, ppc_per_unit_cents, vat_rate_bps,
                    stock_fba, stock_fbm, daily_velocity_milli, min_price_cents, max_price_cents,
                    buy_box_won, active
             FROM listings WHERE active = 1",
        );
        if marketplace.is_some() {
            sql.push_str(" AND upper(marketplace) = upper(?1)");
        }
        sql.push_str(" ORDER BY title ASC");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = if let Some(mp) = marketplace {
            stmt.query_map(params![mp], Self::map_listing)?
        } else {
            stmt.query_map([], Self::map_listing)?
        };
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn map_listing(r: &rusqlite::Row<'_>) -> rusqlite::Result<Listing> {
        Ok(Listing {
            asin: r.get(0)?,
            sku: r.get(1)?,
            marketplace: r.get(2)?,
            title: r.get(3)?,
            fulfillment: r.get(4)?,
            price_cents: r.get(5)?,
            cogs_cents: r.get(6)?,
            referral_fee_bps: r.get(7)?,
            fba_fee_cents: r.get(8)?,
            ppc_per_unit_cents: r.get(9)?,
            vat_rate_bps: r.get(10)?,
            stock_fba: r.get(11)?,
            stock_fbm: r.get(12)?,
            daily_velocity_milli: r.get(13)?,
            min_price_cents: r.get(14)?,
            max_price_cents: r.get(15)?,
            buy_box_won: r.get::<_, i64>(16)? != 0,
            active: r.get::<_, i64>(17)? != 0,
        })
    }

    pub fn unit_economics(&self, asin: &str) -> Result<UnitEconomics> {
        let l = self.get_listing(asin)?;
        let gross = l.price_cents;
        let vat =
            (gross * l.vat_rate_bps + (10_000 + l.vat_rate_bps) / 2) / (10_000 + l.vat_rate_bps);
        let net_rev = gross - vat;
        let referral = (gross * l.referral_fee_bps + 5_000) / 10_000;
        let net_profit = net_rev - referral - l.fba_fee_cents - l.cogs_cents - l.ppc_per_unit_cents;
        let net_margin_bps = if net_rev > 0 {
            (net_profit * 10_000) / net_rev
        } else {
            0
        };
        Ok(UnitEconomics {
            asin: l.asin,
            sku: l.sku,
            gross_price_cents: gross,
            vat_cents: vat,
            net_revenue_cents: net_rev,
            referral_fee_cents: referral,
            fba_fee_cents: l.fba_fee_cents,
            cogs_cents: l.cogs_cents,
            ppc_per_unit_cents: l.ppc_per_unit_cents,
            net_profit_cents: net_profit,
            net_margin_bps,
        })
    }

    pub fn reprice(&self, asin: &str, competitor_cents: i64) -> Result<RepriceResult> {
        if competitor_cents <= 0 {
            return Err(AmazonError::Invalid(
                "competitor_cents must be positive".into(),
            ));
        }
        let l = self.get_listing(asin)?;
        let target = competitor_cents - 10;
        let mut clamped_to_floor = false;
        let mut clamped_to_ceiling = false;
        let new_price = if target < l.min_price_cents {
            clamped_to_floor = true;
            l.min_price_cents
        } else if target > l.max_price_cents {
            clamped_to_ceiling = true;
            l.max_price_cents
        } else {
            target
        };
        let buy_box_won = new_price <= competitor_cents;
        self.conn.execute(
            "UPDATE listings SET price_cents = ?1, buy_box_won = ?2 WHERE asin = ?3",
            params![new_price, i64::from(buy_box_won), asin],
        )?;
        Ok(RepriceResult {
            asin: asin.into(),
            old_price_cents: l.price_cents,
            competitor_cents,
            new_price_cents: new_price,
            clamped_to_floor,
            clamped_to_ceiling,
            buy_box_won,
        })
    }

    pub fn replenishment_alerts(&self, target_days: i64) -> Result<Vec<ReplenishmentAlert>> {
        let listings = self.list_listings(None)?;
        let mut alerts = Vec::new();
        for l in listings {
            if l.fulfillment != "FBA" || l.daily_velocity_milli <= 0 {
                continue;
            }
            let days_of_cover = (l.stock_fba * 1000) / l.daily_velocity_milli;
            if days_of_cover < target_days {
                let desired = (target_days * 2 * l.daily_velocity_milli + 999) / 1000;
                let ship_qty = (desired - l.stock_fba).max(0).min(l.stock_fbm);
                alerts.push(ReplenishmentAlert {
                    asin: l.asin,
                    sku: l.sku,
                    title: l.title,
                    stock_fba: l.stock_fba,
                    stock_fbm: l.stock_fbm,
                    days_of_cover,
                    recommended_ship_qty: ship_qty,
                });
            }
        }
        Ok(alerts)
    }

    pub fn record_order(&self, input: &RecordOrderInput) -> Result<AmazonOrder> {
        if input.qty <= 0 {
            return Err(AmazonError::Invalid("qty must be positive".into()));
        }
        let l = self.get_listing(&input.asin)?;
        let econ = self.unit_economics(&input.asin)?;
        let gross = econ.gross_price_cents * input.qty;
        let referral = econ.referral_fee_cents * input.qty;
        let fba = econ.fba_fee_cents * input.qty;
        let vat = econ.vat_cents * input.qty;
        let net_profit = econ.net_profit_cents * input.qty;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO amazon_orders (
                amazon_order_id, marketplace, fulfillment, status, asin, sku, qty,
                gross_cents, referral_fee_cents, fba_fee_cents, vat_cents, net_profit_cents,
                invoice_uploaded, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13)
            "#,
            params![
                input.amazon_order_id,
                l.marketplace,
                l.fulfillment,
                input.status,
                l.asin,
                l.sku,
                input.qty,
                gross,
                referral,
                fba,
                vat,
                net_profit,
                now
            ],
        )?;

        Ok(AmazonOrder {
            amazon_order_id: input.amazon_order_id.clone(),
            marketplace: l.marketplace,
            fulfillment: l.fulfillment,
            status: input.status.clone(),
            asin: l.asin,
            sku: l.sku,
            qty: input.qty,
            gross_cents: gross,
            referral_fee_cents: referral,
            fba_fee_cents: fba,
            vat_cents: vat,
            net_profit_cents: net_profit,
            invoice_uploaded: false,
            created_at: now,
        })
    }

    pub fn mark_invoice_uploaded(&self, amazon_order_id: &str) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE amazon_orders SET invoice_uploaded = 1 WHERE amazon_order_id = ?1",
            params![amazon_order_id],
        )?;
        if rows == 0 {
            return Err(AmazonError::NotFound(format!("order {amazon_order_id}")));
        }
        Ok(())
    }

    pub fn status(&self) -> Result<AmazonStatus> {
        let active_listings: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM listings WHERE active = 1", [], |r| {
                    r.get(0)
                })?;
        let (fba_units, fbm_units): (i64, i64) = self.conn.query_row(
            "SELECT COALESCE(SUM(stock_fba), 0), COALESCE(SUM(stock_fbm), 0) FROM listings WHERE active = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let pending_vat_invoices: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM amazon_orders WHERE invoice_uploaded = 0 AND status != 'CANCELLED'",
            [],
            |r| r.get(0),
        )?;
        let total_net_profit_cents: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(net_profit_cents), 0) FROM amazon_orders WHERE status != 'CANCELLED'",
            [],
            |r| r.get(0),
        )?;
        Ok(AmazonStatus {
            db_path: self.db_path.display().to_string(),
            active_listings,
            fba_units,
            fbm_units,
            pending_vat_invoices,
            total_net_profit_cents,
        })
    }

    pub fn seed_demo(&self) -> Result<usize> {
        let listings = vec![
            Listing {
                asin: "B0ATELIER1".into(),
                sku: "CER-VASE-01".into(),
                marketplace: "DE".into(),
                title: "Fluted Stoneware Vase".into(),
                fulfillment: "FBA".into(),
                price_cents: 6990,
                cogs_cents: 1850,
                referral_fee_bps: 1500,
                fba_fee_cents: 540,
                ppc_per_unit_cents: 320,
                vat_rate_bps: 2000,
                stock_fba: 12,
                stock_fbm: 18,
                daily_velocity_milli: 1500, // 8 days cover -> triggers 14d alert
                min_price_cents: 5900,
                max_price_cents: 7900,
                buy_box_won: true,
                active: true,
            },
            Listing {
                asin: "B0ATELIER2".into(),
                sku: "LGT-LAMP-02".into(),
                marketplace: "DE".into(),
                title: "Atelier Desk Lamp".into(),
                fulfillment: "FBA".into(),
                price_cents: 18900,
                cogs_cents: 6200,
                referral_fee_bps: 1500,
                fba_fee_cents: 780,
                ppc_per_unit_cents: 850,
                vat_rate_bps: 2000,
                stock_fba: 22,
                stock_fbm: 7,
                daily_velocity_milli: 600,
                min_price_cents: 16500,
                max_price_cents: 21000,
                buy_box_won: true,
                active: true,
            },
            Listing {
                asin: "B0ATELIER3".into(),
                sku: "TXT-THRW-03".into(),
                marketplace: "DE".into(),
                title: "Alpine Merino Throw".into(),
                fulfillment: "FBA".into(),
                price_cents: 12400,
                cogs_cents: 4100,
                referral_fee_bps: 1500,
                fba_fee_cents: 610,
                ppc_per_unit_cents: 450,
                vat_rate_bps: 2000,
                stock_fba: 5,
                stock_fbm: 14,
                daily_velocity_milli: 1000,
                min_price_cents: 10900,
                max_price_cents: 14500,
                buy_box_won: false,
                active: true,
            },
            Listing {
                asin: "B0ATELIER4".into(),
                sku: "WRK-TRAY-04".into(),
                marketplace: "DE".into(),
                title: "Smoked Oak Valet Tray".into(),
                fulfillment: "FBM".into(),
                price_cents: 4900,
                cogs_cents: 1400,
                referral_fee_bps: 1500,
                fba_fee_cents: 0,
                ppc_per_unit_cents: 200,
                vat_rate_bps: 2000,
                stock_fba: 0,
                stock_fbm: 32,
                daily_velocity_milli: 800,
                min_price_cents: 4200,
                max_price_cents: 5600,
                buy_box_won: true,
                active: true,
            },
        ];
        for l in &listings {
            self.upsert_listing(l)?;
        }
        Ok(listings.len())
    }
}
