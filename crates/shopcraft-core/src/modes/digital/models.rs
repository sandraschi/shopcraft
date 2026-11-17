use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalProduct {
    pub id: i64,
    pub sku: String,
    pub title: String,
    pub kind: String,
    pub version: String,
    pub file_size_mb: f64,
    pub price_net_cents: i64,
    pub issues_license: bool,
    pub max_downloads: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatRate {
    pub country_code: String,
    pub country_name: String,
    pub vat_pct: i64,
    pub oss_eligible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalOrderInput {
    pub buyer_name: String,
    pub buyer_email: String,
    pub country_code: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedLicense {
    pub id: i64,
    pub order_id: i64,
    pub sku: String,
    pub license_key: String,
    pub download_token: String,
    pub downloads_used: i64,
    pub max_downloads: i64,
    pub revoked: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalOrder {
    pub id: i64,
    pub order_number: String,
    pub buyer_name: String,
    pub buyer_email: String,
    pub country_code: String,
    pub vat_pct: i64,
    pub subtotal_net_cents: i64,
    pub vat_cents: i64,
    pub total_cents: i64,
    pub status: String,
    pub created_at: String,
    pub licenses: Vec<IssuedLicense>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalStatus {
    pub active_products: i64,
    pub total_orders: i64,
    pub active_licenses: i64,
    pub total_downloads: i64,
    pub gross_revenue_cents: i64,
    pub oss_vat_collected_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorefrontExport {
    pub store_name: String,
    pub tagline: String,
    pub currency: String,
    pub generated_at: String,
    pub vat_rates: Vec<VatRate>,
    pub products: Vec<DigitalProduct>,
}
