//! Container runtime trait and implementations.

use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

use crate::{
    ContainerConfig, ContainerInfo, ContainerLogs, ContainerResult, ContainerStatus, ImageInfo,
};

/// Container runtime trait
#[async_trait]
pub trait Runtime: Send + Sync {
    fn name(&self) -> &str;
    async fn is_available(&self) -> bool;
    async fn run(&self, config: &ContainerConfig) -> ContainerResult<ContainerInfo>;
    async fn run_wait(
        &self,
        config: &ContainerConfig,
        timeout: Option<Duration>,
    ) -> ContainerResult<ContainerLogs>;
    async fn stop(&self, id: &str, timeout: Option<Duration>) -> ContainerResult<()>;
    async fn remove(&self, id: &str, force: bool) -> ContainerResult<()>;
    async fn status(&self, id: &str) -> ContainerResult<ContainerStatus>;
    async fn logs(&self, id: &str, follow: bool) -> ContainerResult<ContainerLogs>;
    async fn list(&self) -> ContainerResult<Vec<ContainerInfo>>;
    async fn pull(&self, image: &str, auth: Option<&str>) -> ContainerResult<()>;
    async fn images(&self) -> ContainerResult<Vec<ImageInfo>>;
    async fn remove_image(&self, id: &str, force: bool) -> ContainerResult<()>;
    async fn create_network(&self, name: &str, subnet: Option<&str>) -> ContainerResult<()>;
    async fn remove_network(&self, name: &str) -> ContainerResult<()>;
}

/// Network mode
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NetworkMode {
    #[default]
    Bridge,
    Host,
    None,
    Network(String),
}

/// Port mapping
#[derive(Debug, Clone)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
}

/// Volume mount
#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub source: PathBuf,
    pub target: PathBuf,
    pub read_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_mode_default() {
        assert_eq!(NetworkMode::default(), NetworkMode::Bridge);
    }
}
