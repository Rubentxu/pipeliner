//! Infrastructure layer
//!
//! This module contains external integrations and adapters.

mod config;
pub mod container;
pub mod docker;
mod github_actions;
mod gitlab_ci;
mod kubernetes;
mod logging;
mod metrics;
pub mod podman;

pub use config::Config;
pub use container::{ContainerExecutor, ContainerRuntime, DockerExecutor};
pub use github_actions::GitHubActionsBackend;
pub use gitlab_ci::GitLabCIBackend;
pub use kubernetes::KubernetesExecutor;
pub use logging::init_logging;
pub use metrics::{MetricsCollector, PipelineMetrics};
pub use podman::{PodmanError, PodmanExecutor};
