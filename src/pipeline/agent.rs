//! Agent configuration types
//!
//! This module defines agent types for pipeline execution.

use super::errors::ValidationError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Configuration for Docker agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DockerConfig {
    /// Docker image to use
    pub image: String,

    /// Arguments to pass to the container
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables to set
    #[serde(default)]
    pub environment: std::collections::HashMap<String, String>,

    /// Whether to use a specific registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

impl super::Validate for DockerConfig {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.image.is_empty() {
            return Err(ValidationError::InvalidAgentType(
                "Docker image cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Configuration for Kubernetes agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KubernetesConfig {
    /// Kubernetes namespace
    #[serde(default = "default_namespace")]
    pub namespace: String,

    /// Pod template specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_template: Option<String>,

    /// Container image
    pub image: String,

    /// Container name
    #[serde(default = "default_container_name")]
    pub container_name: String,

    /// Node label selector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

fn default_namespace() -> String {
    "default".to_string()
}

fn default_container_name() -> String {
    "rustline".to_string()
}

impl super::Validate for KubernetesConfig {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.image.is_empty() {
            return Err(ValidationError::InvalidAgentType(
                "Kubernetes image cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Configuration for Podman agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PodmanConfig {
    /// Podman image to use
    pub image: String,

    /// Arguments to pass to the container
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables to set
    #[serde(default)]
    pub environment: std::collections::HashMap<String, String>,

    /// Podman socket path
    #[serde(default = "default_socket_path")]
    pub socket_path: String,

    /// API version
    #[serde(default = "default_api_version")]
    pub api_version: String,

    /// Whether to remove container after execution
    #[serde(default = "default_remove")]
    pub remove: bool,

    /// Working directory inside container
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    /// Cgroup manager to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cgroup_manager: Option<String>,

    /// Network mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
}

fn default_socket_path() -> String {
    "/run/podman/podman.sock".to_string()
}

fn default_api_version() -> String {
    "v5.0.0".to_string()
}

fn default_remove() -> bool {
    true
}

impl super::Validate for PodmanConfig {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.image.is_empty() {
            return Err(ValidationError::InvalidAgentType(
                "Podman image cannot be empty".to_string(),
            ));
        }
        if self.socket_path.is_empty() {
            return Err(ValidationError::InvalidAgentType(
                "Podman socket path cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Types of agents available for pipeline execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    /// Execute on any available agent
    Any,

    /// Execute on agent with specific label
    Label(String),

    /// Execute in Docker container
    Docker(DockerConfig),

    /// Execute in Kubernetes pod
    Kubernetes(KubernetesConfig),

    /// Execute in Podman container
    Podman(PodmanConfig),
}

impl AgentType {
    /// Creates an Any agent
    #[must_use]
    pub fn any() -> Self {
        Self::Any
    }

    /// Creates a Label agent
    #[must_use]
    pub fn label(label: impl Into<String>) -> Self {
        Self::Label(label.into())
    }

    /// Creates a Docker agent
    #[must_use]
    pub fn docker(image: impl Into<String>) -> Self {
        Self::Docker(DockerConfig {
            image: image.into(),
            ..Default::default()
        })
    }

    /// Creates a Kubernetes agent
    pub fn kubernetes(image: impl Into<String>) -> Self {
        Self::Kubernetes(KubernetesConfig {
            image: image.into(),
            ..Default::default()
        })
    }

    /// Creates a Podman agent
    #[must_use]
    pub fn podman(image: impl Into<String>) -> Self {
        Self::Podman(PodmanConfig {
            image: image.into(),
            ..Default::default()
        })
    }

    /// Creates a Podman agent with full configuration
    #[must_use]
    pub fn podman_with(config: PodmanConfig) -> Self {
        Self::Podman(config)
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => write!(f, "any"),
            Self::Label(label) => write!(f, "label:{label}"),
            Self::Docker(config) => write!(f, "docker:{}", config.image),
            Self::Kubernetes(config) => {
                write!(f, "kubernetes:{}/{}", config.namespace, config.image)
            }
            Self::Podman(config) => write!(f, "podman:{}", config.image),
        }
    }
}

impl super::Validate for AgentType {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        match self {
            Self::Any => Ok(()),
            Self::Label(label) => {
                if label.is_empty() {
                    return Err(ValidationError::InvalidAgentType(
                        "Label cannot be empty".to_string(),
                    ));
                }
                Ok(())
            }
            Self::Docker(config) => config.validate(),
            Self::Kubernetes(config) => config.validate(),
            Self::Podman(config) => config.validate(),
        }
    }
}

/// Trait for agent configuration
#[allow(clippy::missing_errors_doc)]
pub trait AgentConfig: Send + Sync {
    /// Returns the agent type
    fn agent_type(&self) -> &AgentType;

    /// Validates the agent configuration
    fn validate(&self) -> Result<(), ValidationError> {
        self.agent_type().validate()
    }
}

impl AgentConfig for AgentType {
    fn agent_type(&self) -> &AgentType {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::Validate;
    use super::*;

    #[test]
    fn test_agent_type_any() {
        let agent = AgentType::any();
        assert!(matches!(agent, AgentType::Any));
    }

    #[test]
    fn test_agent_type_label() {
        let agent = AgentType::label("linux");
        assert!(matches!(agent, AgentType::Label(_)));
        assert_eq!(agent.to_string(), "label:linux");
    }

    #[test]
    fn test_agent_type_docker() {
        let agent = AgentType::docker("rust:latest");
        assert!(matches!(agent, AgentType::Docker(_)));
        assert_eq!(agent.to_string(), "docker:rust:latest");
    }

    #[test]
    fn test_agent_type_kubernetes() {
        let agent = AgentType::kubernetes("rust:latest");
        assert!(matches!(agent, AgentType::Kubernetes(_)));
        assert!(agent.to_string().contains("kubernetes"));
    }

    #[test]
    fn test_docker_config_validation() {
        let mut config = DockerConfig::default();
        assert!(Validate::validate(&config).is_err());

        config.image = "rust:latest".to_string();
        assert!(Validate::validate(&config).is_ok());
    }

    #[test]
    fn test_kubernetes_config_validation() {
        let mut config = KubernetesConfig::default();
        assert!(Validate::validate(&config).is_err());

        config.image = "rust:latest".to_string();
        assert!(Validate::validate(&config).is_ok());
    }

    #[test]
    fn test_agent_type_validation() {
        assert!(Validate::validate(&AgentType::any()).is_ok());
        assert!(Validate::validate(&AgentType::label("linux")).is_ok());

        assert!(Validate::validate(&AgentType::label("")).is_err());
        assert!(Validate::validate(&AgentType::docker("")).is_err());
    }

    #[test]
    fn test_agent_trait() {
        let agent = AgentType::docker("rust:latest");
        assert!(Validate::validate(&agent).is_ok());
        assert!(matches!(agent.agent_type(), AgentType::Docker(_)));
    }
}
