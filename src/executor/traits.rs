//! Pipeline execution traits
//!
//! This module defines traits and interfaces for pipeline execution.

use crate::pipeline::{Pipeline, StageResult};
use std::collections::HashMap;

/// Trait for executing pipelines
#[allow(clippy::missing_errors_doc)]
pub trait PipelineExecutor: Send + Sync {
    /// Executes a pipeline and returns the result
    fn execute(&self, pipeline: &Pipeline) -> Result<StageResult, crate::pipeline::PipelineError>;

    /// Validates a pipeline without executing it
    fn validate(&self, pipeline: &Pipeline) -> Result<(), crate::pipeline::ValidationError>;

    /// Performs a dry run of the pipeline (no side effects)
    fn dry_run(&self, pipeline: &Pipeline) -> Result<StageResult, crate::pipeline::PipelineError>;

    /// Returns the capabilities of this executor
    fn capabilities(&self) -> ExecutorCapabilities;

    /// Performs a health check
    fn health_check(&self) -> HealthStatus;
}

/// Capabilities of an executor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct ExecutorCapabilities {
    /// Can execute shell commands
    pub can_execute_shell: bool,

    /// Can run in Docker containers
    pub can_run_docker: bool,

    /// Can run in Kubernetes pods
    pub can_run_kubernetes: bool,

    /// Supports parallel execution
    pub supports_parallel: bool,

    /// Supports caching
    pub supports_caching: bool,

    /// Supports timeout
    pub supports_timeout: bool,

    /// Supports retry
    pub supports_retry: bool,
}

impl Default for ExecutorCapabilities {
    fn default() -> Self {
        Self {
            can_execute_shell: true,
            can_run_docker: false,
            can_run_kubernetes: false,
            supports_parallel: false,
            supports_caching: false,
            supports_timeout: true,
            supports_retry: true,
        }
    }
}

/// Health status of an executor
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Executor is healthy
    Healthy,

    /// Executor is degraded (some features unavailable)
    Degraded {
        /// Reason for degradation
        reason: String,
    },

    /// Executor is unhealthy
    Unhealthy {
        /// Reason for being unhealthy
        reason: String,
    },
}

impl HealthStatus {
    /// Returns true if executor is healthy or degraded
    #[must_use]
    pub fn is_operational(&self) -> bool {
        !matches!(self, Self::Unhealthy { .. })
    }
}

/// Context for pipeline execution
#[derive(Debug, Clone)]
pub struct PipelineContext {
    /// Environment variables
    pub env: HashMap<String, String>,

    /// Current working directory
    pub cwd: std::path::PathBuf,

    /// Pipeline ID
    pub pipeline_id: String,

    /// Stage results from previous stages
    pub stage_results: HashMap<String, StageResult>,
}

impl PipelineContext {
    /// Creates a new pipeline context
    #[must_use]
    pub fn new() -> Self {
        Self {
            env: std::env::vars().collect(),
            cwd: std::env::current_dir().unwrap_or_default(),
            pipeline_id: uuid::Uuid::new_v4().to_string(),
            stage_results: HashMap::new(),
        }
    }

    /// Sets an environment variable
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env.insert(key.into(), value.into());
    }

    /// Gets an environment variable
    #[must_use]
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    /// Sets the current working directory
    pub fn set_cwd(&mut self, path: impl Into<std::path::PathBuf>) {
        self.cwd = path.into();
    }

    /// Records the result of a stage
    pub fn record_stage_result(&mut self, stage_name: &str, result: StageResult) {
        self.stage_results.insert(stage_name.to_string(), result);
    }

    /// Gets the result of a previous stage
    #[must_use]
    pub fn get_stage_result(&self, stage_name: &str) -> Option<&StageResult> {
        self.stage_results.get(stage_name)
    }
}

impl Default for PipelineContext {
    fn default() -> Self {
        Self::new()
    }
}
