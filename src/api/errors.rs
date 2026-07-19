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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authentication_error() {
        let err = ApiError::Authentication("invalid key".to_string());
        assert!(err.to_string().contains("Authentication failed"));
        assert!(err.to_string().contains("invalid key"));
    }

    #[test]
    fn test_rate_limit_error() {
        let err = ApiError::RateLimit("too many requests".to_string());
        assert!(err.to_string().contains("Rate limit exceeded"));
    }

    #[test]
    fn test_request_error() {
        let err = ApiError::Request("bad request".to_string());
        assert!(err.to_string().contains("Request failed"));
    }

    #[test]
    fn test_network_error() {
        let err = ApiError::Network("connection refused".to_string());
        assert!(err.to_string().contains("Network error"));
    }

    #[test]
    fn test_json_error() {
        let err = ApiError::Json("invalid json".to_string());
        assert!(err.to_string().contains("JSON error"));
    }

    #[test]
    fn test_error_debug() {
        let err = ApiError::Authentication("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Authentication"));
    }
}
