use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PosItem {
    pub sku: String,
    pub barcode: String,
    pub title: String,
    pub category: String,
    pub price_cents: i64,
    pub vat_rate_bps: i64,
    pub stock_qty: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shift {
    pub shift_id: String,
    pub clerk_name: String,
    pub status: String,
    pub opening_float_cents: i64,
    pub expected_cash_cents: i64,
    pub counted_cash_cents: Option<i64>,
    pub cash_diff_cents: Option<i64>,
    pub opened_at: String,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutLineInput {
    pub code: String, // SKU or EAN barcode
    pub qty: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutInput {
    pub shift_id: String,
    pub items: Vec<CheckoutLineInput>,
    pub discount_bps: Option<i64>,
    pub cash_cents: i64,
    pub card_cents: i64,
    pub voucher_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptLine {
    pub sku: String,
    pub title: String,
    pub qty: i64,
    pub unit_price_cents: i64,
    pub line_total_cents: i64,
    pub vat_rate_bps: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_no: String,
    pub shift_id: String,
    pub kind: String, // SALE or STORNO
    pub storno_of: Option<String>,
    pub stornoed: bool,
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub vat_20_cents: i64,
    pub vat_10_cents: i64,
    pub total_cents: i64,
    pub cash_paid_cents: i64,
    pub card_paid_cents: i64,
    pub voucher_paid_cents: i64,
    pub change_cents: i64,
    pub rksv_sig: String,
    pub created_at: String,
    pub lines: Vec<ReceiptLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZBon {
    pub zbon_no: String,
    pub shift_id: String,
    pub clerk_name: String,
    pub receipt_count: i64,
    pub storno_count: i64,
    pub gross_total_cents: i64,
    pub cash_net_cents: i64,
    pub card_total_cents: i64,
    pub voucher_total_cents: i64,
    pub vat_20_cents: i64,
    pub vat_10_cents: i64,
    pub opening_float_cents: i64,
    pub expected_cash_cents: i64,
    pub counted_cash_cents: i64,
    pub cash_diff_cents: i64,
    pub closed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PosStatus {
    pub db_path: String,
    pub active_items: i64,
    pub open_shift: Option<String>,
    pub total_receipts: i64,
    pub gross_sales_cents: i64,
}

/// Weekday is 0 for Monday through 6 for Sunday. Minutes are from midnight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningHour {
    pub weekday: i64,
    pub open_minute: i64,
    pub close_minute: i64,
    pub closed: bool,
}

/// A day that is not the normal week: closed, a short day, or an extra open day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarDay {
    pub day: String,
    pub kind: String,
    pub open_minute: Option<i64>,
    pub close_minute: Option<i64>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub employee_id: String,
    pub name: String,
    pub role: String,
    pub active: bool,
}
