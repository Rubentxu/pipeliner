//! REST API server implementation for the Pipeliner API.

use pipeliner_events::LocalEventBus;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::types::RestConfig;

/// REST server
pub struct RestServer {
    config: RestConfig,
    event_bus: Arc<LocalEventBus>,
}

impl RestServer {
    pub fn new(config: RestConfig, event_bus: Arc<LocalEventBus>) -> Self {
        Self { config, event_bus }
    }

    pub async fn start(&self) -> Result<(), std::io::Error> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        info!("Starting REST API server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("REST API server listening on {}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            info!("Accepted connection from {:?}", stream.peer_addr()?);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutePipelineResponse {
    pub execution_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub id: String,
    pub pipeline_name: String,
    pub status: String,
    pub stages: Vec<StageResponse>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResponse {
    pub name: String,
    pub status: String,
    pub steps: Vec<StepResponse>,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResponse {
    pub name: String,
    pub status: String,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfoResponse {
    pub id: String,
    pub status: String,
    pub jobs_completed: usize,
    pub jobs_failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rest_server_new() {
        let event_bus = Arc::new(LocalEventBus::new());
        let config = RestConfig::default();
        let server = RestServer::new(config, event_bus);
        assert_eq!(server.config.port, 8080);
    }

    #[test]
    fn test_execute_pipeline_response() {
        let response = ExecutePipelineResponse {
            execution_id: "test-id".to_string(),
            status: "started".to_string(),
        };
        assert_eq!(response.execution_id, "test-id");
        assert_eq!(response.status, "started");
    }
}
