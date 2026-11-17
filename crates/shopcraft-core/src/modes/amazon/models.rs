use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub asin: String,
    pub sku: String,
    pub marketplace: String,
    pub title: String,
    pub fulfillment: String, // "FBA" or "FBM"
    pub price_cents: i64,
    pub cogs_cents: i64,
    pub referral_fee_bps: i64,
    pub fba_fee_cents: i64,
    pub ppc_per_unit_cents: i64,
    pub vat_rate_bps: i64,
    pub stock_fba: i64,
    pub stock_fbm: i64,
    pub daily_velocity_milli: i64, // e.g. 1500 = 1.5 units/day
    pub min_price_cents: i64,
    pub max_price_cents: i64,
    pub buy_box_won: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitEconomics {
    pub asin: String,
    pub sku: String,
    pub gross_price_cents: i64,
    pub vat_cents: i64,
    pub net_revenue_cents: i64,
    pub referral_fee_cents: i64,
    pub fba_fee_cents: i64,
    pub cogs_cents: i64,
    pub ppc_per_unit_cents: i64,
    pub net_profit_cents: i64,
    pub net_margin_bps: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepriceResult {
    pub asin: String,
    pub old_price_cents: i64,
    pub competitor_cents: i64,
    pub new_price_cents: i64,
    pub clamped_to_floor: bool,
    pub clamped_to_ceiling: bool,
    pub buy_box_won: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplenishmentAlert {
    pub asin: String,
    pub sku: String,
    pub title: String,
    pub stock_fba: i64,
    pub stock_fbm: i64,
    pub days_of_cover: i64,
    pub recommended_ship_qty: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmazonOrder {
    pub amazon_order_id: String,
    pub marketplace: String,
    pub fulfillment: String,
    pub status: String,
    pub asin: String,
    pub sku: String,
    pub qty: i64,
    pub gross_cents: i64,
    pub referral_fee_cents: i64,
    pub fba_fee_cents: i64,
    pub vat_cents: i64,
    pub net_profit_cents: i64,
    pub invoice_uploaded: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordOrderInput {
    pub amazon_order_id: String,
    pub asin: String,
    pub qty: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmazonStatus {
    pub db_path: String,
    pub active_listings: i64,
    pub fba_units: i64,
    pub fbm_units: i64,
    pub pending_vat_invoices: i64,
    pub total_net_profit_cents: i64,
}
