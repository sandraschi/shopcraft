use std::path::Path;

use chrono::{NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::error::{RentalError, Result};
use super::models::{
    CreateBookingInput, RentalBooking, RentalItem, RentalStatus, StorefrontExport,
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
            CREATE TABLE IF NOT EXISTS rental_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sku TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                daily_rate_cents INTEGER NOT NULL,
                deposit_cents INTEGER NOT NULL,
                units_total INTEGER NOT NULL DEFAULT 1,
                active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS bookings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                booking_number TEXT NOT NULL UNIQUE,
                customer_name TEXT NOT NULL,
                customer_email TEXT NOT NULL,
                sku TEXT NOT NULL REFERENCES rental_items(sku),
                start_date TEXT NOT NULL,
                end_date TEXT NOT NULL,
                days INTEGER NOT NULL,
                rental_fee_cents INTEGER NOT NULL,
                deposit_cents INTEGER NOT NULL,
                late_fee_cents INTEGER NOT NULL DEFAULT 0,
                damage_fee_cents INTEGER NOT NULL DEFAULT 0,
                deposit_refund_cents INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'Reserved',
                returned_date TEXT,
                created_at TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    pub fn seed_demo(&self) -> Result<usize> {
        let items = [
            (
                "RNT-EBIKE-01",
                "Alpine E-MTB Full-Suspension",
                "Bikes",
                5_900_i64,
                25_000_i64,
                4_i64,
            ),
            (
                "RNT-CAM-02",
                "Cinema 4K Mirrorless Kit",
                "Camera",
                8_500,
                40_000,
                2,
            ),
            (
                "RNT-CARGO-03",
                "Electric Family Cargo Bike",
                "Bikes",
                4_900,
                20_000,
                3,
            ),
            (
                "RNT-AUDIO-04",
                "32-bit Field Recorder & Shotgun Set",
                "Audio",
                3_500,
                15_000,
                3,
            ),
            (
                "RNT-DRONE-05",
                "Thermal Inspection Drone Kit",
                "Drones",
                11_000,
                50_000,
                1,
            ),
            (
                "RNT-CAMP-06",
                "4-Season Hardshell Roof Tent",
                "Outdoor",
                4_200,
                20_000,
                2,
            ),
        ];
        let mut inserted = 0;
        for (sku, name, cat, rate, dep, units) in items {
            inserted += self.conn.execute(
                "INSERT OR IGNORE INTO rental_items (sku, name, category, daily_rate_cents, deposit_cents, units_total, active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
                params![sku, name, cat, rate, dep, units],
            )?;
        }
        Ok(inserted)
    }

    pub fn list_items(&self) -> Result<Vec<RentalItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sku, name, category, daily_rate_cents, deposit_cents, units_total, active
             FROM rental_items WHERE active = 1 ORDER BY sku ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(RentalItem {
                id: r.get(0)?,
                sku: r.get(1)?,
                name: r.get(2)?,
                category: r.get(3)?,
                daily_rate_cents: r.get(4)?,
                deposit_cents: r.get(5)?,
                units_total: r.get(6)?,
                active: r.get::<_, i64>(7)? != 0,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn get_item(&self, sku: &str) -> Result<RentalItem> {
        self.conn
            .query_row(
                "SELECT id, sku, name, category, daily_rate_cents, deposit_cents, units_total, active
                 FROM rental_items WHERE sku = ?1",
                params![sku],
                |r| {
                    Ok(RentalItem {
                        id: r.get(0)?,
                        sku: r.get(1)?,
                        name: r.get(2)?,
                        category: r.get(3)?,
                        daily_rate_cents: r.get(4)?,
                        deposit_cents: r.get(5)?,
                        units_total: r.get(6)?,
                        active: r.get::<_, i64>(7)? != 0,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| RentalError::NotFound(format!("rental SKU '{sku}'")))
    }

    pub fn available_units(&self, sku: &str, start_date: &str, end_date: &str) -> Result<i64> {
        let item = self.get_item(sku)?;
        let overlapping: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bookings
             WHERE sku = ?1 AND status IN ('Reserved', 'CheckedOut')
               AND start_date <= ?3 AND end_date >= ?2",
            params![sku, start_date, end_date],
            |r| r.get(0),
        )?;
        Ok((item.units_total - overlapping).max(0))
    }

    fn parse_days(start_date: &str, end_date: &str) -> Result<i64> {
        let s = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
            .map_err(|_| RentalError::Invalid(format!("invalid start_date '{start_date}'")))?;
        let e = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
            .map_err(|_| RentalError::Invalid(format!("invalid end_date '{end_date}'")))?;
        if e < s {
            return Err(RentalError::Invalid(
                "end_date must be >= start_date".into(),
            ));
        }
        Ok((e - s).num_days() + 1)
    }

    pub fn book(&mut self, input: &CreateBookingInput) -> Result<RentalBooking> {
        if input.customer_email.trim().is_empty() {
            return Err(RentalError::Invalid("customer_email is required".into()));
        }
        let days = Self::parse_days(&input.start_date, &input.end_date)?;
        let item = self.get_item(input.sku.trim())?;
        let avail = self.available_units(&item.sku, &input.start_date, &input.end_date)?;
        if avail <= 0 {
            return Err(RentalError::Unavailable {
                sku: item.sku,
                start_date: input.start_date.clone(),
                end_date: input.end_date.clone(),
            });
        }

        let rental_fee_cents = days * item.daily_rate_cents;
        let now = Utc::now().to_rfc3339();
        let next_seq: i64 =
            self.conn
                .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM bookings", [], |r| {
                    r.get(0)
                })?;
        let booking_number = format!("RNT-{:05}", next_seq);

        self.conn.execute(
            "INSERT INTO bookings
             (booking_number, customer_name, customer_email, sku, start_date, end_date, days, rental_fee_cents, deposit_cents, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Reserved', ?10)",
            params![
                booking_number,
                input.customer_name.trim(),
                input.customer_email.trim(),
                item.sku,
                input.start_date,
                input.end_date,
                days,
                rental_fee_cents,
                item.deposit_cents,
                now
            ],
        )?;
        self.get_booking(&booking_number)
    }

    pub fn checkout(&mut self, booking_number: &str) -> Result<RentalBooking> {
        let b = self.get_booking(booking_number)?;
        if b.status != "Reserved" {
            return Err(RentalError::InvalidTransition {
                from: b.status,
                to: "CheckedOut".into(),
            });
        }
        self.conn.execute(
            "UPDATE bookings SET status = 'CheckedOut' WHERE booking_number = ?1",
            params![booking_number],
        )?;
        self.get_booking(booking_number)
    }

    pub fn checkin(
        &mut self,
        booking_number: &str,
        return_date: &str,
        damage_fee_cents: i64,
    ) -> Result<RentalBooking> {
        let b = self.get_booking(booking_number)?;
        if b.status != "CheckedOut" {
            return Err(RentalError::InvalidTransition {
                from: b.status,
                to: "Returned".into(),
            });
        }
        let end = NaiveDate::parse_from_str(&b.end_date, "%Y-%m-%d")
            .map_err(|_| RentalError::Invalid("corrupt end_date".into()))?;
        let ret = NaiveDate::parse_from_str(return_date, "%Y-%m-%d")
            .map_err(|_| RentalError::Invalid(format!("invalid return_date '{return_date}'")))?;
        let item = self.get_item(&b.sku)?;
        let late_days = (ret - end).num_days().max(0);
        let late_fee_cents = late_days * ((item.daily_rate_cents * 150 + 50) / 100);
        let total_deduct = late_fee_cents + damage_fee_cents.max(0);
        let deposit_refund_cents = (b.deposit_cents - total_deduct).max(0);

        self.conn.execute(
            "UPDATE bookings
             SET status = 'Returned', returned_date = ?1, late_fee_cents = ?2, damage_fee_cents = ?3, deposit_refund_cents = ?4
             WHERE booking_number = ?5",
            params![
                return_date,
                late_fee_cents,
                damage_fee_cents.max(0),
                deposit_refund_cents,
                booking_number
            ],
        )?;
        self.get_booking(booking_number)
    }

    pub fn get_booking(&self, booking_number: &str) -> Result<RentalBooking> {
        self.conn
            .query_row(
                "SELECT b.id, b.booking_number, b.customer_name, b.customer_email, b.sku, i.name,
                        b.start_date, b.end_date, b.days, b.rental_fee_cents, b.deposit_cents,
                        b.late_fee_cents, b.damage_fee_cents, b.deposit_refund_cents, b.status,
                        b.returned_date, b.created_at
                 FROM bookings b
                 JOIN rental_items i ON i.sku = b.sku
                 WHERE b.booking_number = ?1",
                params![booking_number],
                |r| {
                    Ok(RentalBooking {
                        id: r.get(0)?,
                        booking_number: r.get(1)?,
                        customer_name: r.get(2)?,
                        customer_email: r.get(3)?,
                        sku: r.get(4)?,
                        item_name: r.get(5)?,
                        start_date: r.get(6)?,
                        end_date: r.get(7)?,
                        days: r.get(8)?,
                        rental_fee_cents: r.get(9)?,
                        deposit_cents: r.get(10)?,
                        late_fee_cents: r.get(11)?,
                        damage_fee_cents: r.get(12)?,
                        deposit_refund_cents: r.get(13)?,
                        status: r.get(14)?,
                        returned_date: r.get(15)?,
                        created_at: r.get(16)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| RentalError::NotFound(format!("booking '{booking_number}'")))
    }

    pub fn list_bookings(&self) -> Result<Vec<RentalBooking>> {
        let mut stmt = self
            .conn
            .prepare("SELECT booking_number FROM bookings ORDER BY id DESC")?;
        let nums: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out = Vec::with_capacity(nums.len());
        for num in nums {
            out.push(self.get_booking(&num)?);
        }
        Ok(out)
    }

    pub fn sales_by_day(&self) -> Result<Vec<crate::sales::SalesPoint>> {
        crate::sales::by_day(
            &self.conn,
            "SELECT substr(created_at, 1, 10),
                    COALESCE(SUM(rental_fee_cents + late_fee_cents + damage_fee_cents), 0),
                    COUNT(*)
             FROM bookings WHERE status != 'Cancelled' GROUP BY 1 ORDER BY 1",
        )
        .map_err(Into::into)
    }

    pub fn status(&self) -> Result<RentalStatus> {
        let fleet_skus: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM rental_items WHERE active = 1",
            [],
            |r| r.get(0),
        )?;
        let total_units: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(units_total), 0) FROM rental_items WHERE active = 1",
            [],
            |r| r.get(0),
        )?;
        let active_bookings: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bookings WHERE status IN ('Reserved', 'CheckedOut')",
            [],
            |r| r.get(0),
        )?;
        let checked_out_units: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bookings WHERE status = 'CheckedOut'",
            [],
            |r| r.get(0),
        )?;
        let deposits_held_cents: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(deposit_cents), 0) FROM bookings WHERE status IN ('Reserved', 'CheckedOut')",
            [],
            |r| r.get(0),
        )?;
        let rental_revenue_cents: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(rental_fee_cents + late_fee_cents + damage_fee_cents), 0) FROM bookings WHERE status != 'Cancelled'",
            [],
            |r| r.get(0),
        )?;
        Ok(RentalStatus {
            fleet_skus,
            total_units,
            active_bookings,
            checked_out_units,
            deposits_held_cents,
            rental_revenue_cents,
        })
    }

    pub fn export_storefront(&self, store_name: &str, tagline: &str) -> Result<StorefrontExport> {
        Ok(StorefrontExport {
            store_name: store_name.to_string(),
            tagline: tagline.to_string(),
            currency: "EUR".to_string(),
            generated_at: Utc::now().to_rfc3339(),
            items: self.list_items()?,
            bookings: self.list_bookings()?,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn overlap_capacity_and_late_checkin() {
        let mut store = Store::open_in_memory().unwrap();
        assert_eq!(store.seed_demo().unwrap(), 6);

        let b1 = store
            .book(&CreateBookingInput {
                customer_name: "Max Hofer".into(),
                customer_email: "max@example.at".into(),
                sku: "RNT-DRONE-05".into(), // units_total = 1
                start_date: "2026-11-20".into(),
                end_date: "2026-11-22".into(),
            })
            .unwrap();
        assert_eq!(b1.days, 3);
        assert_eq!(b1.rental_fee_cents, 33_000);

        let overlap = store.book(&CreateBookingInput {
            customer_name: "Lea Berger".into(),
            customer_email: "lea@example.at".into(),
            sku: "RNT-DRONE-05".into(),
            start_date: "2026-11-21".into(),
            end_date: "2026-11-23".into(),
        });
        assert!(matches!(overlap, Err(RentalError::Unavailable { .. })));

        store.checkout(&b1.booking_number).unwrap();
        let ret = store.checkin(&b1.booking_number, "2026-11-23", 0).unwrap();
        assert_eq!(ret.late_fee_cents, 16_500); // 1 day * 1.5 * 11000
        assert_eq!(ret.deposit_refund_cents, 50_000 - 16_500);
    }
}
