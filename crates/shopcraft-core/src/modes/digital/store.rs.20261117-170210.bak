use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::error::{DigitalError, Result};
use super::models::{
    DigitalOrder, DigitalOrderInput, DigitalProduct, DigitalStatus, IssuedLicense,
    StorefrontExport, VatRate,
};

pub struct Store {
    conn: Connection,
}

fn fnv1a_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn make_license_key(seed: &str) -> String {
    let h1 = fnv1a_64(seed.as_bytes());
    let h2 = fnv1a_64(format!("{seed}:v2").as_bytes());
    let a = (h1 >> 32) as u16;
    let b = (h1 & 0xffff) as u16;
    let c = (h2 >> 32) as u16;
    format!("DSC-{a:04X}-{b:04X}-{c:04X}")
}

fn make_download_token(seed: &str) -> String {
    let h = fnv1a_64(format!("tok:{seed}").as_bytes());
    format!("dl_{h:016x}")
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
            CREATE TABLE IF NOT EXISTS vat_rates (
                country_code TEXT PRIMARY KEY,
                country_name TEXT NOT NULL,
                vat_pct INTEGER NOT NULL,
                oss_eligible INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS digital_products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sku TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                kind TEXT NOT NULL,
                version TEXT NOT NULL,
                file_size_mb REAL NOT NULL,
                price_net_cents INTEGER NOT NULL,
                issues_license INTEGER NOT NULL DEFAULT 1,
                max_downloads INTEGER NOT NULL DEFAULT 5,
                active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS digital_orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_number TEXT NOT NULL UNIQUE,
                buyer_name TEXT NOT NULL,
                buyer_email TEXT NOT NULL,
                country_code TEXT NOT NULL,
                vat_pct INTEGER NOT NULL,
                subtotal_net_cents INTEGER NOT NULL,
                vat_cents INTEGER NOT NULL,
                total_cents INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'Paid',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS issued_licenses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL REFERENCES digital_orders(id) ON DELETE CASCADE,
                sku TEXT NOT NULL,
                license_key TEXT NOT NULL UNIQUE,
                download_token TEXT NOT NULL UNIQUE,
                downloads_used INTEGER NOT NULL DEFAULT 0,
                max_downloads INTEGER NOT NULL DEFAULT 5,
                revoked INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    pub fn seed_demo(&self) -> Result<(usize, usize)> {
        let rates = [
            ("AT", "Austria", 20_i64, true),
            ("DE", "Germany", 19, true),
            ("FR", "France", 20, true),
            ("NL", "Netherlands", 21, true),
            ("IT", "Italy", 22, true),
            ("ES", "Spain", 21, true),
            ("SE", "Sweden", 25, true),
            ("CH", "Switzerland (Export)", 0, false),
        ];
        let mut r_count = 0;
        for (code, name, pct, oss) in rates {
            r_count += self.conn.execute(
                "INSERT OR IGNORE INTO vat_rates (country_code, country_name, vat_pct, oss_eligible)
                 VALUES (?1, ?2, ?3, ?4)",
                params![code, name, pct, oss as i64],
            )?;
        }

        let products = [
            (
                "DIG-RUST-PRO",
                "Local Craft Studio Pro License",
                "License",
                "2.4.0",
                48.5_f64,
                14_900_i64,
                true,
                10_i64,
            ),
            (
                "DIG-EBOOK-01",
                "Local-First Rust Architecture Handbook (PDF/EPUB)",
                "Ebook",
                "1.2.0",
                18.2,
                3_900,
                false,
                5,
            ),
            (
                "DIG-AUDIO-02",
                "Alpine Field & Foley Sound Pack (24-bit WAV)",
                "Asset Pack",
                "1.0.0",
                640.0,
                5_900,
                true,
                3,
            ),
            (
                "DIG-CAD-03",
                "Parametric Joinery & CNC Template Library",
                "Templates",
                "3.1.0",
                92.4,
                7_900,
                true,
                5,
            ),
            (
                "DIG-COURSE-04",
                "Zero-Panic SQLite & egui Workshop (4K MP4)",
                "Course",
                "2026.1",
                1420.0,
                11_900,
                true,
                5,
            ),
            (
                "DIG-FONT-05",
                "Nordic Mono & Grotesk Typeface Family",
                "Fonts",
                "1.5.0",
                6.8,
                8_900,
                true,
                8,
            ),
        ];

        let mut p_count = 0;
        for (sku, title, kind, ver, mb, price, lic, max_dl) in products {
            p_count += self.conn.execute(
                "INSERT OR IGNORE INTO digital_products
                 (sku, title, kind, version, file_size_mb, price_net_cents, issues_license, max_downloads, active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1)",
                params![sku, title, kind, ver, mb, price, lic as i64, max_dl],
            )?;
        }
        Ok((r_count, p_count))
    }

    pub fn list_vat_rates(&self) -> Result<Vec<VatRate>> {
        let mut stmt = self.conn.prepare(
            "SELECT country_code, country_name, vat_pct, oss_eligible FROM vat_rates ORDER BY country_code ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(VatRate {
                country_code: r.get(0)?,
                country_name: r.get(1)?,
                vat_pct: r.get(2)?,
                oss_eligible: r.get::<_, i64>(3)? != 0,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn list_products(&self) -> Result<Vec<DigitalProduct>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sku, title, kind, version, file_size_mb, price_net_cents, issues_license, max_downloads, active
             FROM digital_products WHERE active = 1 ORDER BY sku ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(DigitalProduct {
                id: r.get(0)?,
                sku: r.get(1)?,
                title: r.get(2)?,
                kind: r.get(3)?,
                version: r.get(4)?,
                file_size_mb: r.get(5)?,
                price_net_cents: r.get(6)?,
                issues_license: r.get::<_, i64>(7)? != 0,
                max_downloads: r.get(8)?,
                active: r.get::<_, i64>(9)? != 0,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn get_vat_rate(&self, country_code: &str) -> Result<VatRate> {
        self.conn
            .query_row(
                "SELECT country_code, country_name, vat_pct, oss_eligible FROM vat_rates WHERE upper(country_code) = upper(?1)",
                params![country_code],
                |r| {
                    Ok(VatRate {
                        country_code: r.get(0)?,
                        country_name: r.get(1)?,
                        vat_pct: r.get(2)?,
                        oss_eligible: r.get::<_, i64>(3)? != 0,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| DigitalError::NotFound(format!("VAT country '{country_code}'")))
    }

    fn get_product(&self, sku: &str) -> Result<DigitalProduct> {
        self.conn
            .query_row(
                "SELECT id, sku, title, kind, version, file_size_mb, price_net_cents, issues_license, max_downloads, active
                 FROM digital_products WHERE sku = ?1",
                params![sku],
                |r| {
                    Ok(DigitalProduct {
                        id: r.get(0)?,
                        sku: r.get(1)?,
                        title: r.get(2)?,
                        kind: r.get(3)?,
                        version: r.get(4)?,
                        file_size_mb: r.get(5)?,
                        price_net_cents: r.get(6)?,
                        issues_license: r.get::<_, i64>(7)? != 0,
                        max_downloads: r.get(8)?,
                        active: r.get::<_, i64>(9)? != 0,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| DigitalError::NotFound(format!("SKU '{sku}'")))
    }

    pub fn checkout(&mut self, input: &DigitalOrderInput) -> Result<DigitalOrder> {
        if input.items.is_empty() {
            return Err(DigitalError::Invalid(
                "cart must contain at least one SKU".into(),
            ));
        }
        if input.buyer_email.trim().is_empty() {
            return Err(DigitalError::Invalid("buyer_email is required".into()));
        }

        let vat = self.get_vat_rate(&input.country_code)?;
        let mut prods = Vec::with_capacity(input.items.len());
        let mut subtotal_net_cents = 0_i64;
        for sku in &input.items {
            let p = self.get_product(sku.trim())?;
            subtotal_net_cents += p.price_net_cents;
            prods.push(p);
        }
        let vat_cents = (subtotal_net_cents * vat.vat_pct + 50) / 100;
        let total_cents = subtotal_net_cents + vat_cents;
        let now = Utc::now().to_rfc3339();

        let tx = self.conn.transaction()?;
        let next_seq: i64 = tx.query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM digital_orders",
            [],
            |r| r.get(0),
        )?;
        let order_number = format!("DIG-{:05}", next_seq);

        tx.execute(
            "INSERT INTO digital_orders
             (order_number, buyer_name, buyer_email, country_code, vat_pct, subtotal_net_cents, vat_cents, total_cents, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'Paid', ?9)",
            params![
                order_number,
                input.buyer_name.trim(),
                input.buyer_email.trim(),
                vat.country_code,
                vat.vat_pct,
                subtotal_net_cents,
                vat_cents,
                total_cents,
                now
            ],
        )?;
        let order_id = tx.last_insert_rowid();

        for (idx, p) in prods.iter().enumerate() {
            let seed = format!("{order_number}:{idx}:{}:{}", p.sku, input.buyer_email);
            let license_key = if p.issues_license {
                make_license_key(&seed)
            } else {
                format!("NO-KEY-{next_seq}-{idx}")
            };
            let download_token = make_download_token(&seed);
            tx.execute(
                "INSERT INTO issued_licenses (order_id, sku, license_key, download_token, downloads_used, max_downloads, revoked, created_at)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, 0, ?6)",
                params![order_id, p.sku, license_key, download_token, p.max_downloads, now],
            )?;
        }
        tx.commit()?;

        self.get_order(&order_number)
    }

    pub fn redeem_download(&mut self, token: &str) -> Result<IssuedLicense> {
        let lic = self
            .conn
            .query_row(
                "SELECT id, order_id, sku, license_key, download_token, downloads_used, max_downloads, revoked, created_at
                 FROM issued_licenses WHERE download_token = ?1",
                params![token],
                |r| {
                    Ok(IssuedLicense {
                        id: r.get(0)?,
                        order_id: r.get(1)?,
                        sku: r.get(2)?,
                        license_key: r.get(3)?,
                        download_token: r.get(4)?,
                        downloads_used: r.get(5)?,
                        max_downloads: r.get(6)?,
                        revoked: r.get::<_, i64>(7)? != 0,
                        created_at: r.get(8)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| DigitalError::NotFound(format!("download token '{token}'")))?;

        if lic.revoked {
            return Err(DigitalError::LicenseRevoked(lic.license_key));
        }
        if lic.downloads_used >= lic.max_downloads {
            return Err(DigitalError::DownloadLimitReached(lic.download_token));
        }
        self.conn.execute(
            "UPDATE issued_licenses SET downloads_used = downloads_used + 1 WHERE id = ?1",
            params![lic.id],
        )?;
        let mut updated = lic;
        updated.downloads_used += 1;
        Ok(updated)
    }

    pub fn revoke_license(&mut self, license_key: &str) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE issued_licenses SET revoked = 1 WHERE license_key = ?1",
            params![license_key],
        )?;
        if changed == 0 {
            return Err(DigitalError::NotFound(format!("license '{license_key}'")));
        }
        Ok(())
    }

    pub fn get_order(&self, order_number: &str) -> Result<DigitalOrder> {
        let mut order = self
            .conn
            .query_row(
                "SELECT id, order_number, buyer_name, buyer_email, country_code, vat_pct, subtotal_net_cents, vat_cents, total_cents, status, created_at
                 FROM digital_orders WHERE order_number = ?1",
                params![order_number],
                |r| {
                    Ok(DigitalOrder {
                        id: r.get(0)?,
                        order_number: r.get(1)?,
                        buyer_name: r.get(2)?,
                        buyer_email: r.get(3)?,
                        country_code: r.get(4)?,
                        vat_pct: r.get(5)?,
                        subtotal_net_cents: r.get(6)?,
                        vat_cents: r.get(7)?,
                        total_cents: r.get(8)?,
                        status: r.get(9)?,
                        created_at: r.get(10)?,
                        licenses: Vec::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| DigitalError::NotFound(format!("order '{order_number}'")))?;

        let mut stmt = self.conn.prepare(
            "SELECT id, order_id, sku, license_key, download_token, downloads_used, max_downloads, revoked, created_at
             FROM issued_licenses WHERE order_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![order.id], |r| {
            Ok(IssuedLicense {
                id: r.get(0)?,
                order_id: r.get(1)?,
                sku: r.get(2)?,
                license_key: r.get(3)?,
                download_token: r.get(4)?,
                downloads_used: r.get(5)?,
                max_downloads: r.get(6)?,
                revoked: r.get::<_, i64>(7)? != 0,
                created_at: r.get(8)?,
            })
        })?;
        order.licenses = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(order)
    }

    pub fn list_orders(&self) -> Result<Vec<DigitalOrder>> {
        let mut stmt = self
            .conn
            .prepare("SELECT order_number FROM digital_orders ORDER BY id DESC")?;
        let nums: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out = Vec::with_capacity(nums.len());
        for num in nums {
            out.push(self.get_order(&num)?);
        }
        Ok(out)
    }

    pub fn list_licenses(&self) -> Result<Vec<IssuedLicense>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, order_id, sku, license_key, download_token, downloads_used, max_downloads, revoked, created_at
             FROM issued_licenses ORDER BY id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(IssuedLicense {
                id: r.get(0)?,
                order_id: r.get(1)?,
                sku: r.get(2)?,
                license_key: r.get(3)?,
                download_token: r.get(4)?,
                downloads_used: r.get(5)?,
                max_downloads: r.get(6)?,
                revoked: r.get::<_, i64>(7)? != 0,
                created_at: r.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn status(&self) -> Result<DigitalStatus> {
        let active_products: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM digital_products WHERE active = 1",
            [],
            |r| r.get(0),
        )?;
        let total_orders: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM digital_orders", [], |r| r.get(0))?;
        let active_licenses: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM issued_licenses WHERE revoked = 0",
            [],
            |r| r.get(0),
        )?;
        let total_downloads: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(downloads_used), 0) FROM issued_licenses",
            [],
            |r| r.get(0),
        )?;
        let gross_revenue_cents: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_cents), 0) FROM digital_orders",
            [],
            |r| r.get(0),
        )?;
        let oss_vat_collected_cents: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(vat_cents), 0) FROM digital_orders WHERE country_code != 'AT'",
            [],
            |r| r.get(0),
        )?;
        Ok(DigitalStatus {
            active_products,
            total_orders,
            active_licenses,
            total_downloads,
            gross_revenue_cents,
            oss_vat_collected_cents,
        })
    }

    pub fn export_storefront(&self, store_name: &str, tagline: &str) -> Result<StorefrontExport> {
        Ok(StorefrontExport {
            store_name: store_name.to_string(),
            tagline: tagline.to_string(),
            currency: "EUR".to_string(),
            generated_at: Utc::now().to_rfc3339(),
            vat_rates: self.list_vat_rates()?,
            products: self.list_products()?,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn checkout_oss_vat_and_download_limit() {
        let mut store = Store::open_in_memory().unwrap();
        let (rates, prods) = store.seed_demo().unwrap();
        assert_eq!(rates, 8);
        assert_eq!(prods, 6);

        let order = store
            .checkout(&DigitalOrderInput {
                buyer_name: "Lukas Meyer".into(),
                buyer_email: "lukas@example.de".into(),
                country_code: "DE".into(),
                items: vec!["DIG-AUDIO-02".into()],
            })
            .unwrap();
        assert_eq!(order.vat_pct, 19);
        assert_eq!(order.subtotal_net_cents, 5900);
        assert_eq!(order.vat_cents, 1121);
        assert_eq!(order.total_cents, 7021);
        assert_eq!(order.licenses.len(), 1);
        let tok = order.licenses[0].download_token.clone();

        for i in 1..=3 {
            let dl = store.redeem_download(&tok).unwrap();
            assert_eq!(dl.downloads_used, i);
        }
        let err = store.redeem_download(&tok);
        assert!(matches!(err, Err(DigitalError::DownloadLimitReached(_))));
    }

    #[test]
    fn export_zero_vat_and_revoke() {
        let mut store = Store::open_in_memory().unwrap();
        store.seed_demo().unwrap();
        let order = store
            .checkout(&DigitalOrderInput {
                buyer_name: "Elena Frei".into(),
                buyer_email: "elena@studio.ch".into(),
                country_code: "CH".into(),
                items: vec!["DIG-RUST-PRO".into()],
            })
            .unwrap();
        assert_eq!(order.vat_pct, 0);
        assert_eq!(order.vat_cents, 0);
        let key = order.licenses[0].license_key.clone();
        let tok = order.licenses[0].download_token.clone();
        store.revoke_license(&key).unwrap();
        assert!(matches!(
            store.redeem_download(&tok),
            Err(DigitalError::LicenseRevoked(_))
        ));
    }
}
