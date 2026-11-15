use thiserror::Error;

#[derive(Debug, Error)]
pub enum PosError {
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

    #[error("no open shift for register")]
    NoOpenShift,

    #[error("insufficient tender: total {total_cents} cents, tendered {tendered_cents} cents")]
    InsufficientTender {
        total_cents: i64,
        tendered_cents: i64,
    },

    #[error("insufficient stock for {sku}: requested {requested}, available {available}")]
    InsufficientStock {
        sku: String,
        requested: i64,
        available: i64,
    },

    #[error("receipt {0} already stornoed")]
    AlreadyStornoed(String),
}

pub type Result<T> = std::result::Result<T, PosError>;
