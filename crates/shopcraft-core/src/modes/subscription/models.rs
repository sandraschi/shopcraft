use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub id: i64,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub cadence_label: String,
    pub cadence_days: i64,
    pub base_price_cents: i64,
    pub subscriber_discount_pct: i64,
    pub net_price_cents: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeInput {
    pub customer_name: String,
    pub customer_email: String,
    pub iban: String,
    pub plan_sku: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: i64,
    pub sub_number: String,
    pub customer_name: String,
    pub customer_email: String,
    pub iban: String,
    pub mandate_ref: String,
    pub plan_sku: String,
    pub plan_name: String,
    pub cadence_days: i64,
    pub net_price_cents: i64,
    pub status: String,
    pub next_billing_date: String,
    pub cycles_billed: i64,
    pub skips_used: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewalCycle {
    pub id: i64,
    pub sub_number: String,
    pub plan_sku: String,
    pub kind: String,
    pub amount_cents: i64,
    pub billing_date: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubStatus {
    pub active_plans: i64,
    pub active_subscriptions: i64,
    pub paused_subscriptions: i64,
    pub mrr_cents: i64,
    pub total_cycles_billed: i64,
    pub total_skips: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorefrontExport {
    pub store_name: String,
    pub tagline: String,
    pub currency: String,
    pub generated_at: String,
    pub plans: Vec<SubscriptionPlan>,
    pub subscriptions: Vec<Subscription>,
}
