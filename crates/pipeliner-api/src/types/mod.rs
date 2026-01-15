//! API types for the Pipeliner gRPC and REST API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub grpc_port: u16,
    pub rest_port: u16,
    pub host: String,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            grpc_port: 50051,
            rest_port: 8080,
            host: "0.0.0.0".to_string(),
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

/// gRPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    pub port: u16,
    pub host: String,
    pub max_concurrent_rpcs: Option<usize>,
    pub max_receive_message_length: Option<usize>,
    pub max_send_message_length: Option<usize>,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            port: 50051,
            host: "0.0.0.0".to_string(),
            max_concurrent_rpcs: None,
            max_receive_message_length: None,
            max_send_message_length: None,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

/// REST configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestConfig {
    pub port: u16,
    pub host: String,
    pub cors_enabled: bool,
    pub cors_origins: Vec<String>,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

impl Default for RestConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "0.0.0.0".to_string(),
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

/// Pipeline status for API responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PipelineStatus {
    Pending,
    Running,
    Success,
    Failure,
    Cancelled,
    Unstable,
}

/// Pipeline info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInfo {
    pub id: Uuid,
    pub name: String,
    pub status: PipelineStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Execution info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInfo {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub pipeline_name: String,
    pub status: PipelineStatus,
    pub stages: Vec<StageInfo>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Stage info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageInfo {
    pub name: String,
    pub status: PipelineStatus,
    pub steps: Vec<StepInfo>,
    pub duration_seconds: Option<f64>,
}

/// Step info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInfo {
    pub name: String,
    pub status: PipelineStatus,
    pub duration_seconds: Option<f64>,
    pub output: Option<String>,
}

/// Worker info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub id: String,
    pub status: String,
    pub current_job_id: Option<Uuid>,
    pub jobs_completed: usize,
    pub jobs_failed: usize,
    pub last_heartbeat: Option<DateTime<Utc>>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub components: Vec<ComponentHealth>,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.grpc_port, 50051);
        assert_eq!(config.rest_port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert!(!config.tls_enabled);
    }

    #[test]
    fn test_grpc_config_default() {
        let config = GrpcConfig::default();
        assert_eq!(config.port, 50051);
        assert_eq!(config.host, "0.0.0.0");
        assert!(!config.tls_enabled);
    }

    #[test]
    fn test_rest_config_default() {
        let config = RestConfig::default();
        assert_eq!(config.port, 8080);
        assert!(config.cors_enabled);
        assert_eq!(config.cors_origins, vec!["*"]);
    }

    #[test]
    fn test_pipeline_info() {
        let info = PipelineInfo {
            id: Uuid::new_v4(),
            name: "test-pipeline".to_string(),
            status: PipelineStatus::Running,
            created_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
        };
        assert_eq!(info.status, PipelineStatus::Running);
    }
}
