use thiserror::Error;

#[derive(Debug, Error)]
pub enum DigitalError {
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

    #[error("download limit reached for token {0}")]
    DownloadLimitReached(String),

    #[error("license revoked: {0}")]
    LicenseRevoked(String),
}

pub type Result<T> = std::result::Result<T, DigitalError>;
