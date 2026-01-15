//! # Pipeliner Infrastructure
//!
//! Container runtime infrastructure for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

pub mod podman;
pub mod runtime;

pub use podman::PodmanRuntime;

/// Re-exports
pub use pipeliner_core::agent::AgentType;
pub use pipeliner_executor::ExecutionContext;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct InfrastructureError(#[from] InfrastructureErrorKind);

#[derive(Debug, thiserror::Error)]
pub enum InfrastructureErrorKind {
    #[error("connection failed: {reason}")]
    ConnectionFailed { reason: String },
    #[error("container not found: {id}")]
    ContainerNotFound { id: String },
    #[error("container timeout")]
    ContainerTimeout,
}

impl From<std::io::Error> for InfrastructureError {
    fn from(e: std::io::Error) -> Self {
        Self(
            InfrastructureErrorKind::ConnectionFailed {
                reason: e.to_string(),
            }
            .into(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerStatus {
    #[default]
    Created,
    Running,
    Paused,
    Exited,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
}

#[derive(Debug, Clone)]
pub struct ContainerLogs {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerConfig {
    pub image: String,
    pub tag: String,
    pub registry: Option<String>,
    pub name: Option<String>,
    pub command: Vec<String>,
    pub environment: std::collections::HashMap<String, String>,
    pub working_dir: Option<std::path::PathBuf>,
    pub auto_remove: bool,
}

impl ContainerConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_image(mut self, image: impl Into<String>) -> Self {
        let image = image.into();
        if let Some((repo, tag)) = image.rsplit_once(':') {
            self.image = repo.to_string();
            self.tag = tag.to_string();
        } else {
            self.image = image;
            self.tag = "latest".to_string();
        }
        self
    }

    #[must_use]
    pub fn full_image(&self) -> String {
        match &self.registry {
            Some(registry) => format!("{}/{}", registry, self.image),
            None => self.image.clone(),
        }
    }
}

pub type ContainerResult<T = ()> = Result<T, InfrastructureError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_status_default() {
        assert_eq!(ContainerStatus::default(), ContainerStatus::Created);
    }

    #[test]
    fn test_container_config_new() {
        let config = ContainerConfig::new();
        assert!(config.image.is_empty());
    }

    #[test]
    fn test_container_config_with_image() {
        let config = ContainerConfig::new().with_image("rust:1.75");
        assert_eq!(config.image, "rust");
        assert_eq!(config.tag, "1.75");
    }
}
