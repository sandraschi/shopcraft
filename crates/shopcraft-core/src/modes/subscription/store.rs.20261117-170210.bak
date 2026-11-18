use std::path::Path;

use chrono::{Duration, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::error::{Result, SubError};
use super::models::{
    RenewalCycle, StorefrontExport, SubStatus, SubscribeInput, Subscription, SubscriptionPlan,
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
            CREATE TABLE IF NOT EXISTS plans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sku TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                cadence_label TEXT NOT NULL,
                cadence_days INTEGER NOT NULL,
                base_price_cents INTEGER NOT NULL,
                subscriber_discount_pct INTEGER NOT NULL DEFAULT 15,
                active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sub_number TEXT NOT NULL UNIQUE,
                customer_name TEXT NOT NULL,
                customer_email TEXT NOT NULL,
                iban TEXT NOT NULL,
                mandate_ref TEXT NOT NULL UNIQUE,
                plan_sku TEXT NOT NULL REFERENCES plans(sku),
                status TEXT NOT NULL DEFAULT 'Active',
                next_billing_date TEXT NOT NULL,
                cycles_billed INTEGER NOT NULL DEFAULT 1,
                skips_used INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS renewal_cycles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sub_number TEXT NOT NULL REFERENCES subscriptions(sub_number),
                plan_sku TEXT NOT NULL,
                kind TEXT NOT NULL,
                amount_cents INTEGER NOT NULL,
                billing_date TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    pub fn seed_demo(&self) -> Result<usize> {
        let plans = [
            (
                "SUB-COFFEE-01",
                "Alpine Single-Origin Roast (1kg)",
                "Coffee",
                "Every 14 days",
                14_i64,
                3_400_i64,
                15_i64,
            ),
            (
                "SUB-PET-02",
                "Organic Dog & Cat Nutrition Box",
                "Pets",
                "Monthly",
                30,
                5_800,
                15,
            ),
            (
                "SUB-TEA-03",
                "Herbal Apothecary Loose-Leaf Trio",
                "Tea",
                "Monthly",
                30,
                2_900,
                12,
            ),
            (
                "SUB-HOME-04",
                "Zero-Waste Home Refill Bundle",
                "Home",
                "Every 30 days",
                30,
                3_900,
                18,
            ),
            (
                "SUB-PANTRY-05",
                "Artisan Sourdough & Olive Oil Crate",
                "Pantry",
                "Monthly",
                30,
                6_400,
                10,
            ),
            (
                "SUB-CRAFT-06",
                "Seasonal Nordic Objects Discovery",
                "Lifestyle",
                "Quarterly",
                90,
                9_900,
                20,
            ),
        ];

        let mut inserted = 0;
        for (sku, name, cat, label, days, price, disc) in plans {
            inserted += self.conn.execute(
                "INSERT OR IGNORE INTO plans (sku, name, category, cadence_label, cadence_days, base_price_cents, subscriber_discount_pct, active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
                params![sku, name, cat, label, days, price, disc],
            )?;
        }
        Ok(inserted)
    }

    fn plan_net_cents(base: i64, disc_pct: i64) -> i64 {
        (base * (100 - disc_pct) + 50) / 100
    }

    pub fn list_plans(&self) -> Result<Vec<SubscriptionPlan>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sku, name, category, cadence_label, cadence_days, base_price_cents, subscriber_discount_pct, active
             FROM plans WHERE active = 1 ORDER BY sku ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            let base: i64 = r.get(6)?;
            let disc: i64 = r.get(7)?;
            Ok(SubscriptionPlan {
                id: r.get(0)?,
                sku: r.get(1)?,
                name: r.get(2)?,
                category: r.get(3)?,
                cadence_label: r.get(4)?,
                cadence_days: r.get(5)?,
                base_price_cents: base,
                subscriber_discount_pct: disc,
                net_price_cents: Self::plan_net_cents(base, disc),
                active: r.get::<_, i64>(8)? != 0,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn get_plan(&self, sku: &str) -> Result<SubscriptionPlan> {
        self.conn
            .query_row(
                "SELECT id, sku, name, category, cadence_label, cadence_days, base_price_cents, subscriber_discount_pct, active
                 FROM plans WHERE sku = ?1",
                params![sku],
                |r| {
                    let base: i64 = r.get(6)?;
                    let disc: i64 = r.get(7)?;
                    Ok(SubscriptionPlan {
                        id: r.get(0)?,
                        sku: r.get(1)?,
                        name: r.get(2)?,
                        category: r.get(3)?,
                        cadence_label: r.get(4)?,
                        cadence_days: r.get(5)?,
                        base_price_cents: base,
                        subscriber_discount_pct: disc,
                        net_price_cents: Self::plan_net_cents(base, disc),
                        active: r.get::<_, i64>(8)? != 0,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| SubError::NotFound(format!("plan '{sku}'")))
    }

    pub fn subscribe(&mut self, input: &SubscribeInput) -> Result<Subscription> {
        if input.customer_email.trim().is_empty() {
            return Err(SubError::Invalid("customer_email is required".into()));
        }
        let iban_clean = input.iban.trim().replace(' ', "").to_uppercase();
        if iban_clean.len() < 12 {
            return Err(SubError::Invalid("valid SEPA IBAN is required".into()));
        }
        let plan = self.get_plan(input.plan_sku.trim())?;
        let now = Utc::now();
        let next_date = (now + Duration::days(plan.cadence_days))
            .format("%Y-%m-%d")
            .to_string();
        let today = now.format("%Y-%m-%d").to_string();
        let created_at = now.to_rfc3339();

        let tx = self.conn.transaction()?;
        let next_seq: i64 = tx.query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM subscriptions",
            [],
            |r| r.get(0),
        )?;
        let sub_number = format!("SUB-{:05}", next_seq);
        let mandate_ref = format!("MND-{:05}", next_seq);

        tx.execute(
            "INSERT INTO subscriptions
             (sub_number, customer_name, customer_email, iban, mandate_ref, plan_sku, status, next_billing_date, cycles_billed, skips_used, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'Active', ?7, 1, 0, ?8)",
            params![
                sub_number,
                input.customer_name.trim(),
                input.customer_email.trim(),
                iban_clean,
                mandate_ref,
                plan.sku,
                next_date,
                created_at
            ],
        )?;
        tx.execute(
            "INSERT INTO renewal_cycles (sub_number, plan_sku, kind, amount_cents, billing_date, created_at)
             VALUES (?1, ?2, 'Billed', ?3, ?4, ?5)",
            params![sub_number, plan.sku, plan.net_price_cents, today, created_at],
        )?;
        tx.commit()?;

        self.get_subscription(&sub_number)
    }

    pub fn get_subscription(&self, sub_number: &str) -> Result<Subscription> {
        self.conn
            .query_row(
                "SELECT s.id, s.sub_number, s.customer_name, s.customer_email, s.iban, s.mandate_ref,
                        s.plan_sku, p.name, p.cadence_days, p.base_price_cents, p.subscriber_discount_pct,
                        s.status, s.next_billing_date, s.cycles_billed, s.skips_used, s.created_at
                 FROM subscriptions s
                 JOIN plans p ON p.sku = s.plan_sku
                 WHERE s.sub_number = ?1",
                params![sub_number],
                |r| {
                    let base: i64 = r.get(9)?;
                    let disc: i64 = r.get(10)?;
                    Ok(Subscription {
                        id: r.get(0)?,
                        sub_number: r.get(1)?,
                        customer_name: r.get(2)?,
                        customer_email: r.get(3)?,
                        iban: r.get(4)?,
                        mandate_ref: r.get(5)?,
                        plan_sku: r.get(6)?,
                        plan_name: r.get(7)?,
                        cadence_days: r.get(8)?,
                        net_price_cents: Self::plan_net_cents(base, disc),
                        status: r.get(11)?,
                        next_billing_date: r.get(12)?,
                        cycles_billed: r.get(13)?,
                        skips_used: r.get(14)?,
                        created_at: r.get(15)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| SubError::NotFound(format!("subscription '{sub_number}'")))
    }

    pub fn list_subscriptions(&self) -> Result<Vec<Subscription>> {
        let mut stmt = self
            .conn
            .prepare("SELECT sub_number FROM subscriptions ORDER BY id DESC")?;
        let nums: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out = Vec::with_capacity(nums.len());
        for num in nums {
            out.push(self.get_subscription(&num)?);
        }
        Ok(out)
    }

    fn advance_date(date_str: &str, days: i64) -> String {
        if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            (d + Duration::days(days)).format("%Y-%m-%d").to_string()
        } else {
            (Utc::now() + Duration::days(days))
                .format("%Y-%m-%d")
                .to_string()
        }
    }

    pub fn skip_next(&mut self, sub_number: &str) -> Result<Subscription> {
        let sub = self.get_subscription(sub_number)?;
        if sub.status != "Active" {
            return Err(SubError::InvalidState {
                status: sub.status,
                action: "skip".into(),
            });
        }
        let next = Self::advance_date(&sub.next_billing_date, sub.cadence_days);
        let now = Utc::now().to_rfc3339();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE subscriptions SET next_billing_date = ?1, skips_used = skips_used + 1 WHERE sub_number = ?2",
            params![next, sub_number],
        )?;
        tx.execute(
            "INSERT INTO renewal_cycles (sub_number, plan_sku, kind, amount_cents, billing_date, created_at)
             VALUES (?1, ?2, 'Skipped', 0, ?3, ?4)",
            params![sub_number, sub.plan_sku, sub.next_billing_date, now],
        )?;
        tx.commit()?;
        self.get_subscription(sub_number)
    }

    pub fn bill_renewal(&mut self, sub_number: &str) -> Result<Subscription> {
        let sub = self.get_subscription(sub_number)?;
        if sub.status != "Active" {
            return Err(SubError::InvalidState {
                status: sub.status,
                action: "bill".into(),
            });
        }
        let next = Self::advance_date(&sub.next_billing_date, sub.cadence_days);
        let now = Utc::now().to_rfc3339();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE subscriptions SET next_billing_date = ?1, cycles_billed = cycles_billed + 1 WHERE sub_number = ?2",
            params![next, sub_number],
        )?;
        tx.execute(
            "INSERT INTO renewal_cycles (sub_number, plan_sku, kind, amount_cents, billing_date, created_at)
             VALUES (?1, ?2, 'Billed', ?3, ?4, ?5)",
            params![sub_number, sub.plan_sku, sub.net_price_cents, sub.next_billing_date, now],
        )?;
        tx.commit()?;
        self.get_subscription(sub_number)
    }

    pub fn set_status(&mut self, sub_number: &str, new_status: &str) -> Result<Subscription> {
        let sub = self.get_subscription(sub_number)?;
        let valid = matches!(
            (sub.status.as_str(), new_status),
            ("Active", "Paused")
                | ("Paused", "Active")
                | ("Active", "Cancelled")
                | ("Paused", "Cancelled")
        );
        if !valid {
            return Err(SubError::InvalidState {
                status: sub.status,
                action: new_status.to_string(),
            });
        }
        self.conn.execute(
            "UPDATE subscriptions SET status = ?1 WHERE sub_number = ?2",
            params![new_status, sub_number],
        )?;
        self.get_subscription(sub_number)
    }

    pub fn swap_plan(&mut self, sub_number: &str, new_plan_sku: &str) -> Result<Subscription> {
        let sub = self.get_subscription(sub_number)?;
        if sub.status == "Cancelled" {
            return Err(SubError::InvalidState {
                status: sub.status,
                action: "swap".into(),
            });
        }
        let plan = self.get_plan(new_plan_sku)?;
        self.conn.execute(
            "UPDATE subscriptions SET plan_sku = ?1 WHERE sub_number = ?2",
            params![plan.sku, sub_number],
        )?;
        self.get_subscription(sub_number)
    }

    pub fn list_cycles(&self) -> Result<Vec<RenewalCycle>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sub_number, plan_sku, kind, amount_cents, billing_date, created_at
             FROM renewal_cycles ORDER BY id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(RenewalCycle {
                id: r.get(0)?,
                sub_number: r.get(1)?,
                plan_sku: r.get(2)?,
                kind: r.get(3)?,
                amount_cents: r.get(4)?,
                billing_date: r.get(5)?,
                created_at: r.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn status(&self) -> Result<SubStatus> {
        let active_plans: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM plans WHERE active = 1", [], |r| {
                    r.get(0)
                })?;
        let active_subscriptions: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM subscriptions WHERE status = 'Active'",
            [],
            |r| r.get(0),
        )?;
        let paused_subscriptions: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM subscriptions WHERE status = 'Paused'",
            [],
            |r| r.get(0),
        )?;
        let active_subs = self
            .list_subscriptions()?
            .into_iter()
            .filter(|s| s.status == "Active");
        let mrr_cents: i64 = active_subs
            .map(|s| (s.net_price_cents * 30) / s.cadence_days.max(1))
            .sum();
        let total_cycles_billed: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(cycles_billed), 0) FROM subscriptions",
            [],
            |r| r.get(0),
        )?;
        let total_skips: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(skips_used), 0) FROM subscriptions",
            [],
            |r| r.get(0),
        )?;
        Ok(SubStatus {
            active_plans,
            active_subscriptions,
            paused_subscriptions,
            mrr_cents,
            total_cycles_billed,
            total_skips,
        })
    }

    pub fn export_storefront(&self, store_name: &str, tagline: &str) -> Result<StorefrontExport> {
        Ok(StorefrontExport {
            store_name: store_name.to_string(),
            tagline: tagline.to_string(),
            currency: "EUR".to_string(),
            generated_at: Utc::now().to_rfc3339(),
            plans: self.list_plans()?,
            subscriptions: self.list_subscriptions()?,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_skip_swap_and_pause() {
        let mut store = Store::open_in_memory().unwrap();
        assert_eq!(store.seed_demo().unwrap(), 6);

        let sub = store
            .subscribe(&SubscribeInput {
                customer_name: "Anna Gruber".into(),
                customer_email: "anna@example.at".into(),
                iban: "AT61 1904 3002 3457 3201".into(),
                plan_sku: "SUB-COFFEE-01".into(),
            })
            .unwrap();
        assert_eq!(sub.status, "Active");
        assert_eq!(sub.cycles_billed, 1);

        let skipped = store.skip_next(&sub.sub_number).unwrap();
        assert_eq!(skipped.skips_used, 1);
        assert_ne!(skipped.next_billing_date, sub.next_billing_date);

        let swapped = store.swap_plan(&sub.sub_number, "SUB-PET-02").unwrap();
        assert_eq!(swapped.plan_sku, "SUB-PET-02");

        let paused = store.set_status(&sub.sub_number, "Paused").unwrap();
        assert_eq!(paused.status, "Paused");
        assert!(matches!(
            store.skip_next(&sub.sub_number),
            Err(SubError::InvalidState { .. })
        ));
    }
}
