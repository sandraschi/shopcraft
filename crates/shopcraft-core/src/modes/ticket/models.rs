use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTier {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub event_date: String,
    pub venue: String,
    pub tier_name: String,
    pub price_cents: i64,
    pub capacity: i64,
    pub sold_count: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueTicketInput {
    pub attendee_name: String,
    pub attendee_email: String,
    pub code: String,
    pub quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketOrder {
    pub id: i64,
    pub ticket_code: String,
    pub attendee_name: String,
    pub attendee_email: String,
    pub code: String,
    pub event_title: String,
    pub tier_name: String,
    pub quantity: i64,
    pub total_cents: i64,
    pub status: String,
    pub checked_in_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketStatus {
    pub events_count: i64,
    pub total_capacity: i64,
    pub seats_sold: i64,
    pub seats_checked_in: i64,
    pub gross_revenue_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorefrontExport {
    pub store_name: String,
    pub tagline: String,
    pub currency: String,
    pub generated_at: String,
    pub events: Vec<EventTier>,
    pub orders: Vec<TicketOrder>,
}
