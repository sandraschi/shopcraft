use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAccount {
    pub id: i64,
    pub account_code: String,
    pub company_name: String,
    pub country: String,
    pub vat_uid: String,
    pub vat_verified: bool,
    pub tier: String,
    pub payment_terms: String,
    pub credit_limit_cents: i64,
    pub balance_cents: i64,
    pub contact_email: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WholesaleProduct {
    pub id: i64,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub moq: i64,
    pub pack_size: i64,
    pub list_price_cents: i64,
    pub tier_silver_pct: i64,
    pub tier_gold_pct: i64,
    pub stock: i64,
    pub image_url: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLineInput {
    pub sku: String,
    pub qty: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderInput {
    pub account_code: String,
    pub po_reference: String,
    pub items: Vec<OrderLineInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLine {
    pub id: i64,
    pub order_id: i64,
    pub sku: String,
    pub product_name: String,
    pub qty: i64,
    pub unit_price_cents: i64,
    pub discount_pct: i64,
    pub line_total_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub id: i64,
    pub order_number: String,
    pub account_code: String,
    pub company_name: String,
    pub po_reference: String,
    pub status: String,
    pub payment_terms: String,
    pub reverse_charge: bool,
    pub subtotal_cents: i64,
    pub vat_cents: i64,
    pub total_cents: i64,
    pub due_date: String,
    pub created_at: String,
    pub lines: Vec<OrderLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteLine {
    pub sku: String,
    pub name: String,
    pub qty: i64,
    pub list_price_cents: i64,
    pub unit_price_cents: i64,
    pub discount_pct: i64,
    pub volume_bonus: bool,
    pub line_total_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResult {
    pub account_code: String,
    pub tier: String,
    pub payment_terms: String,
    pub reverse_charge: bool,
    pub vat_rate_pct: i64,
    pub subtotal_cents: i64,
    pub vat_cents: i64,
    pub total_cents: i64,
    pub credit_remaining_cents: i64,
    pub credit_ok: bool,
    pub lines: Vec<QuoteLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WholesaleStatus {
    pub active_products: i64,
    pub total_stock_units: i64,
    pub trade_accounts: i64,
    pub open_orders: i64,
    pub total_credit_exposure_cents: i64,
    pub total_revenue_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalExport {
    pub portal_name: String,
    pub tagline: String,
    pub currency: String,
    pub generated_at: String,
    pub accounts: Vec<TradeAccount>,
    pub products: Vec<WholesaleProduct>,
}
