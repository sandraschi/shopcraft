use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub sku: String,
    pub slug: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub category: String,
    pub price_cents: i64,
    pub compare_at_cents: Option<i64>,
    pub vat_rate_bps: i64,
    pub stock_qty: i64,
    pub badge: Option<String>,
    pub accent_hex: String,
    pub material: String,
    pub dimensions: String,
    pub image_url: Option<String>,
    pub featured: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoCode {
    pub code: String,
    pub kind: String, // "PERCENT" or "FIXED_CENTS"
    pub value: i64,
    pub min_order_cents: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingZone {
    pub zone_code: String,
    pub label: String,
    pub base_rate_cents: i64,
    pub free_over_cents: i64,
    pub eta_days: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemInput {
    pub sku: String,
    pub qty: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderInput {
    pub customer_name: String,
    pub customer_email: String,
    pub shipping_address: String,
    pub zone_code: String,
    pub promo_code: Option<String>,
    pub items: Vec<OrderItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub sku: String,
    pub title: String,
    pub qty: i64,
    pub unit_price_cents: i64,
    pub line_total_cents: i64,
    pub vat_rate_bps: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_no: String,
    pub status: String,
    pub customer_name: String,
    pub customer_email: String,
    pub shipping_address: String,
    pub zone_code: String,
    pub promo_code: Option<String>,
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub shipping_cents: i64,
    pub vat_cents: i64,
    pub total_cents: i64,
    pub carrier: Option<String>,
    pub tracking_no: Option<String>,
    pub created_at: String,
    pub items: Vec<OrderItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorefrontExport {
    pub shop_name: String,
    pub tagline: String,
    pub currency: String,
    pub generated_at: String,
    pub categories: Vec<String>,
    pub products: Vec<Product>,
    pub shipping_zones: Vec<ShippingZone>,
    pub promo_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopStatus {
    pub db_path: String,
    pub active_products: i64,
    pub total_stock_units: i64,
    pub open_orders: i64,
    pub total_revenue_cents: i64,
}
