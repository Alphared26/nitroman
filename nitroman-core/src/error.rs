#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("WMI error: {0}")]
    Wmi(String),
    #[error("unsupported platform: {0}")]
    Unsupported(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid data: {0}")]
    InvalidData(String),
}

pub type Result<T> = std::result::Result<T, Error>;
