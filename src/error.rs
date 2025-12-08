use std::io;

use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DistributorError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("configuration error: {0}")]
    Config(String),
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
    #[error("other distributor error: {0}")]
    Other(String),
}
