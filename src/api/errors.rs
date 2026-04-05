//! API errors

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Request failed: {0}")]
    Request(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("JSON error: {0}")]
    Json(String),
}
