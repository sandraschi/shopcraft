use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalItem {
    pub id: i64,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub daily_rate_cents: i64,
    pub deposit_cents: i64,
    pub units_total: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBookingInput {
    pub customer_name: String,
    pub customer_email: String,
    pub sku: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalBooking {
    pub id: i64,
    pub booking_number: String,
    pub customer_name: String,
    pub customer_email: String,
    pub sku: String,
    pub item_name: String,
    pub start_date: String,
    pub end_date: String,
    pub days: i64,
    pub rental_fee_cents: i64,
    pub deposit_cents: i64,
    pub late_fee_cents: i64,
    pub damage_fee_cents: i64,
    pub deposit_refund_cents: i64,
    pub status: String,
    pub returned_date: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalStatus {
    pub fleet_skus: i64,
    pub total_units: i64,
    pub active_bookings: i64,
    pub checked_out_units: i64,
    pub deposits_held_cents: i64,
    pub rental_revenue_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorefrontExport {
    pub store_name: String,
    pub tagline: String,
    pub currency: String,
    pub generated_at: String,
    pub items: Vec<RentalItem>,
    pub bookings: Vec<RentalBooking>,
}
