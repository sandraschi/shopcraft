use thiserror::Error;

#[derive(Debug, Error)]
pub enum WholesaleError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("business rule error: {0}")]
    Biz(#[from] craft_biz::BizError),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("MOQ violation for {sku}: requested {requested}, minimum {moq}")]
    MoqViolation {
        sku: String,
        requested: i64,
        moq: i64,
    },

    #[error("pack size violation for {sku}: requested {requested} is not a multiple of pack size {pack_size}")]
    PackSizeViolation {
        sku: String,
        requested: i64,
        pack_size: i64,
    },

    #[error("insufficient stock for {sku}: requested {requested}, available {available}")]
    InsufficientStock {
        sku: String,
        requested: i64,
        available: i64,
    },

    #[error("credit limit exceeded for {account_code}: balance {balance_cents} + order {order_cents} > limit {limit_cents}")]
    CreditExceeded {
        account_code: String,
        balance_cents: i64,
        order_cents: i64,
        limit_cents: i64,
    },

    #[error("invalid order transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
}

pub type Result<T> = std::result::Result<T, WholesaleError>;
