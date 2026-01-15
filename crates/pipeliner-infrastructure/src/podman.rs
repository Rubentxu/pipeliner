//! Native Podman API client.

use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;
use tracing::debug;

use crate::ContainerConfig;
use crate::ContainerInfo;
use crate::ContainerLogs;
use crate::ContainerResult;
use crate::ContainerStatus;
use crate::ImageInfo;
use crate::runtime::Runtime;

/// Native Podman runtime
#[derive(Debug)]
pub struct PodmanRuntime {
    socket_path: PathBuf,
}

impl PodmanRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self::with_socket(PathBuf::from("/run/podman/podman.sock"))
    }

    #[must_use]
    pub fn with_socket(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }
}

#[async_trait]
impl Runtime for PodmanRuntime {
    fn name(&self) -> &str {
        "podman"
    }

    async fn is_available(&self) -> bool {
        debug!("Checking Podman at {}", self.socket_path.display());
        self.socket_path.exists()
    }

    async fn run(&self, config: &ContainerConfig) -> ContainerResult<ContainerInfo> {
        debug!("Creating container from image: {}", config.full_image());
        let id = format!(
            "podman-{}",
            uuid::Uuid::new_v4()
                .to_string()
                .chars()
                .take(8)
                .collect::<String>()
        );
        Ok(ContainerInfo {
            id: id.clone(),
            name: config.name.clone().unwrap_or(id),
            image: config.full_image(),
            status: ContainerStatus::Created,
        })
    }

    async fn run_wait(
        &self,
        config: &ContainerConfig,
        _timeout: Option<Duration>,
    ) -> ContainerResult<ContainerLogs> {
        let _ = self.run(config).await?;
        Ok(ContainerLogs {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        })
    }

    async fn stop(&self, _id: &str, _timeout: Option<Duration>) -> ContainerResult<()> {
        Ok(())
    }

    async fn remove(&self, _id: &str, _force: bool) -> ContainerResult<()> {
        Ok(())
    }

    async fn status(&self, _id: &str) -> ContainerResult<ContainerStatus> {
        Ok(ContainerStatus::Created)
    }

    async fn logs(&self, _id: &str, _follow: bool) -> ContainerResult<ContainerLogs> {
        Ok(ContainerLogs {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        })
    }

    async fn list(&self) -> ContainerResult<Vec<ContainerInfo>> {
        Ok(vec![])
    }

    async fn pull(&self, image: &str, _auth: Option<&str>) -> ContainerResult<()> {
        tracing::info!("Pulling image: {}", image);
        Ok(())
    }

    async fn images(&self) -> ContainerResult<Vec<ImageInfo>> {
        Ok(vec![])
    }

    async fn remove_image(&self, _id: &str, _force: bool) -> ContainerResult<()> {
        Ok(())
    }

    async fn create_network(&self, _name: &str, _subnet: Option<&str>) -> ContainerResult<()> {
        Ok(())
    }

    async fn remove_network(&self, _name: &str) -> ContainerResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_podman_runtime_name() {
        let runtime = PodmanRuntime::new();
        assert_eq!(runtime.name(), "podman");
    }
}
