//! Agent types and configuration for pipeline execution.
//!
//! This module defines the different agent types that can be used
//! to execute pipeline stages, including Docker, Kubernetes, Podman, and label-based agents.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Agent type for pipeline execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentType {
    /// Use any available agent
    Any,

    /// Use an agent with a specific label
    Label {
        /// Label to match
        label: String,
    },

    /// Docker container agent
    Docker {
        /// Docker image to use
        image: String,
        /// Docker registry
        #[serde(skip_serializing_if = "Option::is_none")]
        registry: Option<String>,
        /// Docker credentials
        #[serde(skip_serializing_if = "Option::is_none")]
        credentials: Option<DockerCredentials>,
        /// Working directory inside container
        #[serde(skip_serializing_if = "Option::is_none")]
        working_dir: Option<PathBuf>,
        /// Environment variables
        #[serde(default)]
        environment: HashMap<String, String>,
        /// Always pull image
        #[serde(default)]
        always_pull: bool,
    },

    /// Podman container agent
    Podman {
        /// Podman image to use
        image: String,
        /// Podman registry
        #[serde(skip_serializing_if = "Option::is_none")]
        registry: Option<String>,
        /// Podman credentials
        #[serde(skip_serializing_if = "Option::is_none")]
        credentials: Option<PodmanCredentials>,
        /// Working directory inside container
        #[serde(skip_serializing_if = "Option::is_none")]
        working_dir: Option<PathBuf>,
        /// Environment variables
        #[serde(default)]
        environment: HashMap<String, String>,
    },

    /// Kubernetes agent
    Kubernetes {
        /// Kubernetes namespace
        #[serde(default = "default_namespace")]
        namespace: String,
        /// Pod template
        #[serde(skip_serializing_if = "Option::is_none")]
        pod_template: Option<PodTemplate>,
        /// Container image
        #[serde(skip_serializing_if = "Option::is_none")]
        image: Option<String>,
        /// Service account
        #[serde(skip_serializing_if = "Option::is_none")]
        service_account: Option<String>,
        /// Node selector
        #[serde(default)]
        node_selector: HashMap<String, String>,
    },

    /// Custom agent via label expression
    Custom {
        /// Custom label expression
        label: String,
    },
}

fn default_namespace() -> String {
    "default".to_string()
}

/// Docker credentials for registry authentication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerCredentials {
    /// Registry URL
    pub registry: String,
    /// Username
    pub username: String,
    /// Password or token
    pub password: String,
}

/// Podman credentials for registry authentication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodmanCredentials {
    /// Registry URL
    pub registry: String,
    /// Username
    pub username: String,
    /// Password or token
    pub password: String,
}

/// Docker configuration for agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DockerConfig {
    /// Docker image
    pub image: String,
    /// Registry URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// Credentials ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_id: Option<String>,
}

/// Kubernetes configuration for agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesConfig {
    /// Namespace
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// Default pod template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_pod_template: Option<PodTemplate>,
}

/// Podman configuration for agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PodmanConfig {
    /// Podman image
    pub image: String,
    /// Registry URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

/// Generic agent configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    /// Agent type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_type: Option<AgentType>,
    /// Label for label-based agents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Custom cloud
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud: Option<String>,
}

/// Kubernetes pod template
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PodTemplate {
    /// Pod name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Container spec
    #[serde(default)]
    pub containers: Vec<ContainerSpec>,
    /// Volumes
    #[serde(default)]
    pub volumes: Vec<VolumeSpec>,
    /// Node selector
    #[serde(default)]
    pub node_selector: HashMap<String, String>,
    /// Service account name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_name: Option<String>,
    /// Image pull secrets
    #[serde(default)]
    pub image_pull_secrets: Vec<String>,
}

/// Container specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSpec {
    /// Container name
    pub name: String,
    /// Docker image
    pub image: String,
    /// Working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
    /// Command
    #[serde(default)]
    pub command: Vec<String>,
    /// Arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables
    #[serde(default)]
    pub env: Vec<EnvVar>,
    /// Resource limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceLimits>,
}

/// Environment variable
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EnvVar {
    /// Simple key-value
    Value { name: String, value: String },
    /// From secret
    Secret {
        name: String,
        secret_name: String,
        secret_key: String,
    },
    /// From config map
    ConfigMap {
        name: String,
        config_map_name: String,
        config_map_key: String,
    },
}

/// Resource limits
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLimits {
    /// CPU limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    /// Memory limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    /// CPU request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_request: Option<String>,
    /// Memory request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_request: Option<String>,
}

/// Volume specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VolumeSpec {
    /// Empty dir volume
    EmptyDir {
        name: String,
        #[serde(default)]
        medium: String,
    },
    /// Secret volume
    Secret {
        name: String,
        secret_name: String,
        #[serde(default)]
        items: Vec<KeyPath>,
    },
    /// Config map volume
    ConfigMap {
        name: String,
        config_map_name: String,
    },
    /// Persistent volume claim
    PersistentVolumeClaim { name: String, claim_name: String },
}

/// Key-path mapping
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPath {
    pub key: String,
    pub path: String,
}

impl AgentType {
    /// Creates an "any" agent type
    #[must_use]
    pub fn any() -> Self {
        Self::Any
    }

    /// Creates a label-based agent
    #[must_use]
    pub fn label(label: impl Into<String>) -> Self {
        Self::Label {
            label: label.into(),
        }
    }

    /// Creates a Docker agent
    #[must_use]
    pub fn docker(image: impl Into<String>) -> Self {
        Self::Docker {
            image: image.into(),
            registry: None,
            credentials: None,
            working_dir: None,
            environment: HashMap::new(),
            always_pull: false,
        }
    }

    /// Creates a Kubernetes agent
    #[must_use]
    pub fn kubernetes() -> Self {
        Self::Kubernetes {
            namespace: default_namespace(),
            pod_template: None,
            image: None,
            service_account: None,
            node_selector: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_any() {
        let agent = AgentType::any();
        assert!(matches!(agent, AgentType::Any));
    }

    #[test]
    fn test_agent_label() {
        let agent = AgentType::label("linux");
        assert!(matches!(agent, AgentType::Label { label } if label == "linux"));
    }

    #[test]
    fn test_agent_docker() {
        let agent = AgentType::docker("rust:1.75");
        if let AgentType::Docker { image, .. } = agent {
            assert_eq!(image, "rust:1.75");
        } else {
            panic!("Expected Docker agent");
        }
    }

    #[test]
    fn test_agent_kubernetes() {
        let agent = AgentType::kubernetes();
        if let AgentType::Kubernetes { namespace, .. } = agent {
            assert_eq!(namespace, "default");
        } else {
            panic!("Expected Kubernetes agent");
        }
    }
}
