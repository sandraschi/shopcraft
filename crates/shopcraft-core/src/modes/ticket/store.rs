use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::error::{Result, TicketError};
use super::models::{EventTier, IssueTicketInput, StorefrontExport, TicketOrder, TicketStatus};

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
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS event_tiers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                event_date TEXT NOT NULL,
                venue TEXT NOT NULL,
                tier_name TEXT NOT NULL,
                price_cents INTEGER NOT NULL,
                capacity INTEGER NOT NULL,
                active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS ticket_orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ticket_code TEXT NOT NULL UNIQUE,
                attendee_name TEXT NOT NULL,
                attendee_email TEXT NOT NULL,
                code TEXT NOT NULL,
                event_title TEXT NOT NULL,
                tier_name TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                total_cents INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'issued',
                checked_in_at TEXT,
                created_at TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    pub fn seed_demo(&self) -> Result<usize> {
        let demo = [
            (
                "EVT-RUST-WS",
                "Pure-Rust Systems Workshop",
                "2026-12-05 10:00",
                "Neubau Makerspace, Vienna",
                "Workshop Pass",
                8900_i64,
                24_i64,
            ),
            (
                "EVT-JAZZ-GA",
                "Winter Courtyard Jazz Quartet",
                "2026-12-11 20:00",
                "Porgy & Bess Annex",
                "General Admission",
                2800_i64,
                120_i64,
            ),
            (
                "EVT-JAZZ-VIP",
                "Winter Courtyard Jazz Quartet",
                "2026-12-11 20:00",
                "Porgy & Bess Annex",
                "Front Table + Vinyl",
                6500_i64,
                16_i64,
            ),
            (
                "EVT-CERAMIC",
                "Wheel-Throwing Pottery Evening",
                "2026-12-14 18:30",
                "Studio Lehm, 1070",
                "Studio Seat",
                5400_i64,
                12_i64,
            ),
            (
                "EVT-SYNTH",
                "Modular Synth Patching Lab",
                "2026-12-18 19:00",
                "Klangwerk 4",
                "Lab Bench",
                4200_i64,
                18_i64,
            ),
            (
                "EVT-ESPRESSO",
                "Single-Origin Espresso Cupping",
                "2026-12-20 11:00",
                "Roesterei Nord",
                "Tasting Flight",
                2400_i64,
                20_i64,
            ),
        ];

        let mut inserted = 0_usize;
        for (code, title, event_date, venue, tier_name, price_cents, capacity) in demo {
            let changed = self.conn.execute(
                "INSERT OR IGNORE INTO event_tiers (code, title, event_date, venue, tier_name, price_cents, capacity, active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
                params![code, title, event_date, venue, tier_name, price_cents, capacity],
            )?;
            inserted += changed;
        }
        Ok(inserted)
    }

    pub fn list_events(&self) -> Result<Vec<EventTier>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.code, e.title, e.event_date, e.venue, e.tier_name,
                    e.price_cents, e.capacity,
                    COALESCE((SELECT SUM(o.quantity) FROM ticket_orders o WHERE o.code = e.code AND o.status != 'refunded'), 0) AS sold_count,
                    e.active
             FROM event_tiers e
             ORDER BY e.event_date ASC, e.id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EventTier {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                event_date: row.get(3)?,
                venue: row.get(4)?,
                tier_name: row.get(5)?,
                price_cents: row.get(6)?,
                capacity: row.get(7)?,
                sold_count: row.get(8)?,
                active: row.get::<_, i64>(9)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_event(&self, code: &str) -> Result<Option<EventTier>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.code, e.title, e.event_date, e.venue, e.tier_name,
                    e.price_cents, e.capacity,
                    COALESCE((SELECT SUM(o.quantity) FROM ticket_orders o WHERE o.code = e.code AND o.status != 'refunded'), 0) AS sold_count,
                    e.active
             FROM event_tiers e
             WHERE e.code = ?1",
        )?;
        stmt.query_row(params![code], |row| {
            Ok(EventTier {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                event_date: row.get(3)?,
                venue: row.get(4)?,
                tier_name: row.get(5)?,
                price_cents: row.get(6)?,
                capacity: row.get(7)?,
                sold_count: row.get(8)?,
                active: row.get::<_, i64>(9)? != 0,
            })
        })
        .optional()
        .map_err(Into::into)
    }

    pub fn issue_ticket(&self, input: &IssueTicketInput) -> Result<TicketOrder> {
        if input.quantity <= 0 || input.quantity > 20 {
            return Err(TicketError::InvalidQuantity(input.quantity));
        }
        let tier = self
            .get_event(&input.code)?
            .filter(|e| e.active)
            .ok_or_else(|| TicketError::TierNotFound(input.code.clone()))?;

        let remaining = (tier.capacity - tier.sold_count).max(0);
        if input.quantity > remaining {
            return Err(TicketError::SoldOut {
                code: tier.code,
                requested: input.quantity,
                remaining,
            });
        }

        let total_cents = tier.price_cents * input.quantity;
        let now = Utc::now().to_rfc3339();
        let seq: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM ticket_orders",
            [],
            |r| r.get(0),
        )?;
        let ticket_code = format!("TCK-{seq:05}");

        self.conn.execute(
            "INSERT INTO ticket_orders (
                ticket_code, attendee_name, attendee_email, code, event_title,
                tier_name, quantity, total_cents, status, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'issued', ?9)",
            params![
                ticket_code,
                input.attendee_name,
                input.attendee_email,
                tier.code,
                tier.title,
                tier.tier_name,
                input.quantity,
                total_cents,
                now
            ],
        )?;
        self.get_order(&ticket_code)?
            .ok_or(TicketError::TicketNotFound(ticket_code))
    }

    pub fn check_in(&self, ticket_code: &str) -> Result<TicketOrder> {
        let order = self
            .get_order(ticket_code)?
            .ok_or_else(|| TicketError::TicketNotFound(ticket_code.to_string()))?;
        if order.status != "issued" {
            return Err(TicketError::InvalidState(format!(
                "ticket {ticket_code} is '{}', expected 'issued'",
                order.status
            )));
        }
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE ticket_orders SET status = 'checked_in', checked_in_at = ?1 WHERE ticket_code = ?2",
            params![now, ticket_code],
        )?;
        self.get_order(ticket_code)?
            .ok_or_else(|| TicketError::TicketNotFound(ticket_code.to_string()))
    }

    pub fn refund_ticket(&self, ticket_code: &str) -> Result<TicketOrder> {
        let order = self
            .get_order(ticket_code)?
            .ok_or_else(|| TicketError::TicketNotFound(ticket_code.to_string()))?;
        if order.status != "issued" {
            return Err(TicketError::InvalidState(format!(
                "ticket {ticket_code} is '{}', only 'issued' tickets can be refunded",
                order.status
            )));
        }
        self.conn.execute(
            "UPDATE ticket_orders SET status = 'refunded' WHERE ticket_code = ?1",
            params![ticket_code],
        )?;
        self.get_order(ticket_code)?
            .ok_or_else(|| TicketError::TicketNotFound(ticket_code.to_string()))
    }

    pub fn get_order(&self, ticket_code: &str) -> Result<Option<TicketOrder>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ticket_code, attendee_name, attendee_email, code, event_title,
                    tier_name, quantity, total_cents, status, checked_in_at, created_at
             FROM ticket_orders
             WHERE ticket_code = ?1",
        )?;
        stmt.query_row(params![ticket_code], Self::map_order)
            .optional()
            .map_err(Into::into)
    }

    pub fn list_orders(&self) -> Result<Vec<TicketOrder>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ticket_code, attendee_name, attendee_email, code, event_title,
                    tier_name, quantity, total_cents, status, checked_in_at, created_at
             FROM ticket_orders
             ORDER BY id DESC",
        )?;
        let rows = stmt.query_map([], Self::map_order)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn map_order(row: &rusqlite::Row<'_>) -> rusqlite::Result<TicketOrder> {
        Ok(TicketOrder {
            id: row.get(0)?,
            ticket_code: row.get(1)?,
            attendee_name: row.get(2)?,
            attendee_email: row.get(3)?,
            code: row.get(4)?,
            event_title: row.get(5)?,
            tier_name: row.get(6)?,
            quantity: row.get(7)?,
            total_cents: row.get(8)?,
            status: row.get(9)?,
            checked_in_at: row.get(10)?,
            created_at: row.get(11)?,
        })
    }

    pub fn status(&self) -> Result<TicketStatus> {
        let events_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM event_tiers WHERE active = 1",
            [],
            |r| r.get(0),
        )?;
        let total_capacity: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(capacity), 0) FROM event_tiers WHERE active = 1",
            [],
            |r| r.get(0),
        )?;
        let seats_sold: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(quantity), 0) FROM ticket_orders WHERE status != 'refunded'",
            [],
            |r| r.get(0),
        )?;
        let seats_checked_in: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(quantity), 0) FROM ticket_orders WHERE status = 'checked_in'",
            [],
            |r| r.get(0),
        )?;
        let gross_revenue_cents: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_cents), 0) FROM ticket_orders WHERE status != 'refunded'",
            [],
            |r| r.get(0),
        )?;
        Ok(TicketStatus {
            events_count,
            total_capacity,
            seats_sold,
            seats_checked_in,
            gross_revenue_cents,
        })
    }

    pub fn export_storefront(&self, store_name: &str, tagline: &str) -> Result<StorefrontExport> {
        Ok(StorefrontExport {
            store_name: store_name.to_string(),
            tagline: tagline.to_string(),
            currency: "EUR".to_string(),
            generated_at: Utc::now().to_rfc3339(),
            events: self.list_events()?,
            orders: self.list_orders()?,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn capacity_checkin_and_refund_flow() {
        let store = Store::open_in_memory().unwrap();
        store.seed_demo().unwrap();

        // EVT-CERAMIC has capacity 12. Issue 10 seats first.
        let o1 = store
            .issue_ticket(&IssueTicketInput {
                attendee_name: "Alice".to_string(),
                attendee_email: "alice@example.com".to_string(),
                code: "EVT-CERAMIC".to_string(),
                quantity: 10,
            })
            .unwrap();
        assert_eq!(o1.total_cents, 54_000);
        assert_eq!(o1.status, "issued");

        // Requesting 3 more should fail with SoldOut (only 2 remaining).
        let err = store
            .issue_ticket(&IssueTicketInput {
                attendee_name: "Bob".to_string(),
                attendee_email: "bob@example.com".to_string(),
                code: "EVT-CERAMIC".to_string(),
                quantity: 3,
            })
            .unwrap_err();
        assert!(matches!(err, TicketError::SoldOut { remaining: 2, .. }));

        // Refund Alice's 10 seats, then Bob can buy 3.
        let refunded = store.refund_ticket(&o1.ticket_code).unwrap();
        assert_eq!(refunded.status, "refunded");

        let o2 = store
            .issue_ticket(&IssueTicketInput {
                attendee_name: "Bob".to_string(),
                attendee_email: "bob@example.com".to_string(),
                code: "EVT-CERAMIC".to_string(),
                quantity: 3,
            })
            .unwrap();
        let checked = store.check_in(&o2.ticket_code).unwrap();
        assert_eq!(checked.status, "checked_in");
        assert!(checked.checked_in_at.is_some());

        let st = store.status().unwrap();
        assert_eq!(st.seats_sold, 3);
        assert_eq!(st.seats_checked_in, 3);
    }
}
