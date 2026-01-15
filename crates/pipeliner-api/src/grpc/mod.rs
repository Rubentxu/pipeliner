//! gRPC server implementation for the Pipeliner API.

use pipeliner_events::LocalEventBus;
use std::sync::Arc;
use tracing::info;

use crate::types::GrpcConfig;

/// gRPC server
pub struct GrpcServer {
    config: GrpcConfig,
    event_bus: Arc<LocalEventBus>,
}

impl GrpcServer {
    pub fn new(config: GrpcConfig, event_bus: Arc<LocalEventBus>) -> Self {
        Self { config, event_bus }
    }

    pub async fn start(&self) {
        info!(
            "gRPC server configured on {}:{}",
            self.config.host, self.config.port
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grpc_server_new() {
        let event_bus = Arc::new(LocalEventBus::new());
        let config = GrpcConfig::default();
        let server = GrpcServer::new(config, event_bus);
        assert_eq!(server.config.port, 50051);
    }
}
