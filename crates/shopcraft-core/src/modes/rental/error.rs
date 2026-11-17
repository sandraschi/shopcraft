use thiserror::Error;

#[derive(Debug, Error)]
pub enum RentalError {
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

    #[error("no units available for {sku} from {start_date} to {end_date}")]
    Unavailable {
        sku: String,
        start_date: String,
        end_date: String,
    },

    #[error("invalid booking transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
}

pub type Result<T> = std::result::Result<T, RentalError>;
