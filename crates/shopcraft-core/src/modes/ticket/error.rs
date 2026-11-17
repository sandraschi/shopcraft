use thiserror::Error;

#[derive(Debug, Error)]
pub enum TicketError {
    #[error("sqlite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("event tier not found: {0}")]
    TierNotFound(String),
    #[error("ticket order not found: {0}")]
    TicketNotFound(String),
    #[error(
        "sold out / insufficient seats for {code}: requested {requested}, remaining {remaining}"
    )]
    SoldOut {
        code: String,
        requested: i64,
        remaining: i64,
    },
    #[error("invalid quantity: {0}")]
    InvalidQuantity(i64),
    #[error("invalid state transition: {0}")]
    InvalidState(String),
}

pub type Result<T> = std::result::Result<T, TicketError>;
