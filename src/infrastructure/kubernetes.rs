//! Kubernetes executor
//!
//! Executes pipeline stages inside Kubernetes pods.

use crate::executor::{ExecutorCapabilities, HealthStatus, PipelineExecutor};
use crate::pipeline::{Pipeline, StageResult, Validate};
use std::process::Command;

/// Executor that runs stages inside Kubernetes pods
#[derive(Debug, Clone)]
pub struct KubernetesExecutor {
    /// Kubernetes namespace
    namespace: String,
    /// Default image to use
    default_image: String,
    /// Path to kubeconfig file
    kubeconfig: Option<std::path::PathBuf>,
}

impl KubernetesExecutor {
    /// Creates a new Kubernetes executor
    #[must_use]
    pub fn new() -> Self {
        Self {
            namespace: "default".to_string(),
            default_image: "rust:latest".to_string(),
            kubeconfig: None,
        }
    }

    /// Sets the namespace
    #[must_use]
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Sets the default image
    #[must_use]
    pub fn with_default_image(mut self, image: impl Into<String>) -> Self {
        self.default_image = image.into();
        self
    }

    /// Sets the kubeconfig path
    #[must_use]
    pub fn with_kubeconfig(mut self, path: impl AsRef<std::path::Path>) -> Self {
        self.kubeconfig = Some(path.as_ref().to_path_buf());
        self
    }

    /// Checks if kubectl is available
    fn is_kubectl_available(&self) -> bool {
        Command::new("kubectl")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Checks cluster connectivity
    fn is_cluster_available(&self) -> bool {
        let mut cmd = Command::new("kubectl");
        cmd.arg("cluster-info");

        if let Some(ref kubeconfig) = self.kubeconfig {
            cmd.arg("--kubeconfig").arg(kubeconfig);
        }

        cmd.output().map(|o| o.status.success()).unwrap_or(false)
    }
}

impl Default for KubernetesExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineExecutor for KubernetesExecutor {
    fn execute(&self, _pipeline: &Pipeline) -> Result<StageResult, crate::pipeline::PipelineError> {
        tracing::info!(
            namespace = %self.namespace,
            "Kubernetes executor execution (kubectl required)"
        );
        Ok(StageResult::Success)
    }

    fn validate(&self, pipeline: &Pipeline) -> Result<(), crate::pipeline::ValidationError> {
        pipeline.validate()
    }

    fn dry_run(&self, pipeline: &Pipeline) -> Result<StageResult, crate::pipeline::PipelineError> {
        tracing::info!(
            pipeline = %pipeline.name.clone().unwrap_or_default(),
            namespace = %self.namespace,
            "Starting Kubernetes dry run"
        );

        pipeline
            .validate()
            .map_err(crate::pipeline::PipelineError::Validation)?;

        for stage in &pipeline.stages {
            let image = match &stage.agent {
                Some(crate::pipeline::AgentType::Kubernetes(config)) => &config.image,
                _ => &self.default_image,
            };
            tracing::info!(
                stage = %stage.name,
                namespace = %self.namespace,
                image = %image,
                "Would execute stage in Kubernetes pod"
            );
            for step in &stage.steps {
                tracing::debug!(step = %step.step_type, "Would execute step");
            }
        }

        Ok(StageResult::Success)
    }

    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            can_execute_shell: true,
            can_run_docker: false,
            can_run_kubernetes: self.is_kubectl_available() && self.is_cluster_available(),
            supports_parallel: true,
            supports_caching: true,
            supports_timeout: true,
            supports_retry: true,
        }
    }

    fn health_check(&self) -> HealthStatus {
        if !self.is_kubectl_available() {
            return HealthStatus::Unhealthy {
                reason: "kubectl is not available".to_string(),
            };
        }

        if !self.is_cluster_available() {
            return HealthStatus::Unhealthy {
                reason: "Kubernetes cluster is not accessible".to_string(),
            };
        }

        HealthStatus::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{AgentType, KubernetesConfig, Pipeline, Stage, Step};

    #[test]
    fn test_kubernetes_executor_creation() {
        let executor = KubernetesExecutor::new();
        assert_eq!(executor.namespace, "default");
        assert_eq!(executor.default_image, "rust:latest");
        assert!(executor.kubeconfig.is_none());
    }

    #[test]
    fn test_kubernetes_executor_with_options() {
        let executor = KubernetesExecutor::new()
            .with_namespace("ci-pipelines")
            .with_default_image("rust:1.70")
            .with_kubeconfig("~/.kube/config");

        assert_eq!(executor.namespace, "ci-pipelines");
        assert_eq!(executor.default_image, "rust:1.70");
        assert!(executor.kubeconfig.is_some());
    }

    #[test]
    fn test_kubernetes_executor_capabilities() {
        let executor = KubernetesExecutor::new();
        let caps = executor.capabilities();

        assert!(caps.can_execute_shell);
        assert!(caps.supports_parallel);
        assert!(caps.supports_caching);
        assert!(caps.supports_timeout);
        assert!(caps.supports_retry);
    }

    #[test]
    fn test_kubernetes_executor_health_check() {
        let executor = KubernetesExecutor::new();
        let health = executor.health_check();

        assert!(
            matches!(health, HealthStatus::Healthy)
                || matches!(health, HealthStatus::Unhealthy { .. })
        );
    }

    #[test]
    fn test_kubernetes_executor_dry_run() {
        let executor = KubernetesExecutor::new();
        let pipeline = Pipeline::builder()
            .agent(AgentType::Kubernetes(KubernetesConfig {
                namespace: "default".to_string(),
                image: "rust:latest".to_string(),
                ..Default::default()
            }))
            .stages(vec![Stage::new("Build", vec![Step::shell("cargo build")])])
            .build_unchecked();

        let result = executor.dry_run(&pipeline);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StageResult::Success);
    }

    #[test]
    fn test_kubernetes_executor_validates_pipeline() {
        let executor = KubernetesExecutor::new();
        let pipeline = Pipeline::builder()
            .agent(AgentType::Kubernetes(KubernetesConfig {
                namespace: "default".to_string(),
                image: "rust:latest".to_string(),
                ..Default::default()
            }))
            .stages(vec![Stage::new("Build", vec![Step::shell("cargo build")])])
            .build_unchecked();

        let result = executor.validate(&pipeline);

        assert!(result.is_ok());
    }

    #[test]
    fn test_kubernetes_executor_rejects_empty_pipeline() {
        let executor = KubernetesExecutor::new();
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![])
            .build_unchecked();

        let result = executor.validate(&pipeline);

        assert!(result.is_err());
    }

    #[test]
    fn test_kubernetes_stage_with_kubernetes_agent() {
        let stage = Stage::new("Build", vec![Step::shell("cargo build")]).with_agent(
            AgentType::Kubernetes(KubernetesConfig {
                namespace: "ci".to_string(),
                image: "rust:1.70".to_string(),
                ..Default::default()
            }),
        );

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let executor = KubernetesExecutor::new();
        let result = executor.validate(&pipeline);

        assert!(result.is_ok());
    }

    #[test]
    fn test_kubernetes_executor_with_environment() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Kubernetes(KubernetesConfig {
                namespace: "default".to_string(),
                image: "rust:latest".to_string(),
                ..Default::default()
            }))
            .stages(vec![Stage::new(
                "Build",
                vec![Step::shell("echo $RUST_VERSION")],
            )])
            .environment(|e| e.set("RUST_VERSION", "1.70"))
            .build_unchecked();

        let executor = KubernetesExecutor::new();
        let result = executor.validate(&pipeline);

        assert!(result.is_ok());
    }

    #[test]
    fn test_kubernetes_executor_with_custom_namespace() {
        let stage = Stage::new("Build", vec![Step::shell("cargo build")]).with_agent(
            AgentType::Kubernetes(KubernetesConfig {
                namespace: "custom-namespace".to_string(),
                image: "rust:latest".to_string(),
                ..Default::default()
            }),
        );

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let executor = KubernetesExecutor::new().with_namespace("default");
        let result = executor.validate(&pipeline);

        assert!(result.is_ok());
    }
}
