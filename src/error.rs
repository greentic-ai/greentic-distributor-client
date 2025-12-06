use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DistributorError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("wit error: {0}")]
    Wit(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("resource not found")]
    NotFound,
    #[error("permission denied")]
    PermissionDenied,
    #[error("unexpected status {status}: {body}")]
    Status { status: StatusCode, body: String },
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
