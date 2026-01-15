//! # Pipeliner API
//!
//! gRPC and REST API layer for Pipeliner pipeline execution.
//!
//! ## Architecture
//!
//! The API crate provides:
//!
//! - `grpc`: gRPC server implementations
//! - `rest`: REST API server
//! - `types`: API types and configuration
//!
//! ## Example
//!
//! ```rust,ignore
//! use pipeliner_api::{GrpcServer, RestServer, GrpcConfig, RestConfig};
//! use pipeliner_events::LocalEventBus;
//!
//! let event_bus = LocalEventBus::new();
//! let grpc_config = GrpcConfig::default();
//! let grpc_server = GrpcServer::new(grpc_config, event_bus.clone());
//! ```

#![warn(missing_docs)]
#![warn(unused)]

pub mod grpc;
pub mod rest;
pub mod types;

pub use grpc::GrpcServer;
pub use rest::RestServer;
pub use types::{ApiConfig, GrpcConfig, RestConfig};

/// API errors
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ApiError(#[from] ApiErrorKind);

#[derive(Debug, thiserror::Error)]
pub enum ApiErrorKind {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("not found: {0}")]
    NotFound(String),
}

/// Result type for API operations
pub type ApiResult<T = ()> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.grpc_port, 50051);
        assert_eq!(config.rest_port, 8080);
    }

    #[test]
    fn test_grpc_config_default() {
        let config = GrpcConfig::default();
        assert_eq!(config.port, 50051);
        assert!(!config.tls_enabled);
    }

    #[test]
    fn test_rest_config_default() {
        let config = RestConfig::default();
        assert_eq!(config.port, 8080);
        assert!(config.cors_enabled);
    }
}
