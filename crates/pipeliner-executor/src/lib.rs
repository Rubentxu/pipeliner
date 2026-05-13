//! # Pipeliner Executor
//!
//! Pipeline execution engine for Pipeliner. This crate provides the core
//! execution logic for running pipeline stages and steps.
//!
//! ## Architecture
//!
//! The executor is organized around:
//!
//! - `context`: Execution context for tracking state during execution
//! - `runtime`: Runtime for executing steps
//! - `strategy`: Execution strategies (sequential, parallel, matrix)
//! - `listener`: Event listeners for execution events
//!
//! ## Example
//!
//! ```rust,ignore
//! use pipeliner_executor::{Executor, ExecutionConfig};
//! use pipeliner_core::Pipeline;
//!
//! let pipeline = Pipeline::new().with_name("Example");
//! let config = ExecutionConfig::default();
//! let mut executor = Executor::new(pipeline, config);
//! let result = executor.run().await;
//! ```

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

pub mod context;
pub mod formatters;
pub mod listener;
pub mod local;
pub mod observers;
pub mod report;
pub mod runtime;
pub mod shell;
pub mod strategy;
pub mod temp_files;

pub use context::{ExecutionConfig, ExecutionContext};
pub use formatters::{create_formatter, OutputFormat, OutputFormatter};
pub use listener::ExecutionListener;
pub use local::{LocalExecutor, LocalResult};
pub use observers::{JsonCollector, LoggingObserver, NoopObserver, ObserverBox, PipelineContext, PipelineEvent, PipelineObserver};
pub use report::{ExecutionReport, StageReport, StepReport};
pub use runtime::StepExecutor;
pub use shell::{expand_variables, jenkins_shell_config, ShellCommand, ShellConfig, ShellResult};
pub use strategy::{ExecutionStrategy, ParallelStrategy, SequentialStrategy};
pub use temp_files::{JenkinsPathResolver, TempFileManager};

/// Re-exports
pub use pipeliner_core::{Pipeline, Stage, Step, StepType, Validate, ValidationError};

/// Executor result type
pub type ExecutorResult<T = ()> = Result<T, ExecutorError>;

/// Executor error types
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ExecutorError(#[from] ExecutorErrorKind);

/// Specific error kinds
#[derive(Debug, thiserror::Error)]
pub enum ExecutorErrorKind {
    #[error("step execution failed: {reason}")]
    StepFailed { reason: String },

    #[error("stage '{stage}' failed")]
    StageFailed { stage: String },

    #[error("pipeline timeout exceeded")]
    TimeoutExceeded,

    #[error("step retry exhausted after {attempts} attempts")]
    RetryExhausted { attempts: usize },

    #[error("agent allocation failed: {reason}")]
    AgentAllocationFailed { reason: String },

    #[error("I/O error: {reason}")]
    IoError { reason: std::io::Error },

    #[error("unexpected termination: {reason}")]
    UnexpectedTermination { reason: String },
}

impl From<std::io::Error> for ExecutorError {
    fn from(e: std::io::Error) -> Self {
        Self(ExecutorErrorKind::IoError { reason: e }.into())
    }
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Not started
    Pending,
    /// Currently running
    Running,
    /// Completed successfully
    Success,
    /// Completed with failures
    Failure,
    /// Stopped due to timeout
    Timeout,
    /// Aborted externally
    Aborted,
    /// Unstable (some failures but not critical)
    Unstable,
    /// Skipped (condition not met)
    Skipped,
}

impl Default for ExecutionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl ExecutionStatus {
    /// Returns true if the status indicates success
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionStatus::Success)
    }

    /// Returns true if the status indicates failure
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            ExecutionStatus::Failure | ExecutionStatus::Timeout | ExecutionStatus::Aborted
        )
    }
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Final status
    pub status: ExecutionStatus,
    /// Duration of execution
    pub duration: chrono::Duration,
    /// Number of stages executed
    pub stages_executed: usize,
    /// Number of steps executed
    pub steps_executed: usize,
    /// Error message if failed
    pub error: Option<String>,
}

/// Capabilities of an executor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            supports_caching: true,
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
    Degraded { reason: String },
    /// Executor is unhealthy
    Unhealthy { reason: String },
}

impl HealthStatus {
    /// Returns true if executor is healthy or degraded
    #[must_use]
    pub fn is_operational(&self) -> bool {
        !matches!(self, Self::Unhealthy { .. })
    }
}

/// Unified executor trait combining async execution with validation and capabilities
#[async_trait::async_trait(?Send)]
pub trait UnifiedExecutor {
    /// Execute a pipeline and return the result
    async fn execute_pipeline(&self, pipeline: &Pipeline) -> ExecutorResult<ExecutionResult>;

    /// Validate a pipeline without executing
    fn validate_pipeline(&self, pipeline: &Pipeline) -> Result<(), ValidationError>;

    /// Dry run - validate and report what would execute
    async fn dry_run(&self, pipeline: &Pipeline) -> ExecutorResult<ExecutionResult>;

    /// Return executor capabilities
    fn capabilities(&self) -> ExecutorCapabilities;
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self {
            status: ExecutionStatus::Pending,
            duration: chrono::Duration::zero(),
            stages_executed: 0,
            steps_executed: 0,
            error: None,
        }
    }
}

impl ExecutionResult {
    /// Creates a successful result
    #[must_use]
    pub fn success(stages: usize, steps: usize, duration: chrono::Duration) -> Self {
        Self {
            status: ExecutionStatus::Success,
            duration,
            stages_executed: stages,
            steps_executed: steps,
            error: None,
        }
    }

    /// Creates a failed result
    #[must_use]
    pub fn failure(
        stages: usize,
        steps: usize,
        duration: chrono::Duration,
        error: impl Into<String>,
    ) -> Self {
        Self {
            status: ExecutionStatus::Failure,
            duration,
            stages_executed: stages,
            steps_executed: steps,
            error: Some(error.into()),
        }
    }

    /// Returns true if the execution was successful
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }

    /// Returns true if the execution failed
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self.status, ExecutionStatus::Failure)
    }
}

/// Main executor entry point
#[derive(Debug)]
pub struct Executor {
    pipeline: Pipeline,
    config: ExecutionConfig,
    context: ExecutionContext,
}

impl Executor {
    /// Creates a new executor
    #[must_use]
    pub fn new(pipeline: Pipeline, config: ExecutionConfig) -> Self {
        Self {
            pipeline,
            config,
            context: ExecutionContext::new(),
        }
    }

    /// Runs the pipeline execution
    pub async fn run(&mut self) -> ExecutorResult<ExecutionResult> {
        use std::time::Instant;

        // Validate the pipeline first
        self.pipeline.validate().map_err(|e| {
            ExecutorError::from(ExecutorErrorKind::UnexpectedTermination {
                reason: format!("Pipeline validation failed: {}", e),
            })
        })?;

        let start = Instant::now();

        // Create a LocalExecutor
        let mut local = LocalExecutor::new();

        // Apply retry config if retry_on_failure is enabled
        if self.config.retry_on_failure {
            local = local.with_retry(self.config.max_retries);
        }

        // Execute with optional timeout
        let execute_future = local.execute(&self.pipeline);

        let results = if let Some(timeout) = self.config.global_timeout {
            match tokio::time::timeout(timeout, execute_future).await {
                Ok(results) => results,
                Err(_) => {
                    return Ok(ExecutionResult::failure(
                        self.pipeline.stages.len(),
                        0,
                        chrono::Duration::from_std(start.elapsed()).unwrap_or_default(),
                        "Pipeline execution timed out",
                    ));
                }
            }
        } else {
            execute_future.await
        };

        // Convert Vec<LocalResult> to ExecutionResult
        let duration = chrono::Duration::from_std(start.elapsed()).unwrap_or_default();
        let stages_executed = self.pipeline.stages.len();
        let steps_executed = results.len();

        // Check if any step failed
        let has_failure = results.iter().any(|r| !r.success);

        if has_failure {
            let error_msg = results
                .iter()
                .find(|r| !r.success)
                .map(|r| r.output.clone())
                .unwrap_or_else(|| "Unknown error".to_string());

            Ok(ExecutionResult::failure(
                stages_executed,
                steps_executed,
                duration,
                error_msg,
            ))
        } else {
            Ok(ExecutionResult::success(
                stages_executed,
                steps_executed,
                duration,
            ))
        }
    }

    /// Validates the pipeline before execution
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.pipeline.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::{Pipeline, Stage, Step, StepType, agent::AgentType};
    use tempfile::TempDir;

    fn create_test_pipeline() -> Pipeline {
        Pipeline::new()
            .with_name("Test Pipeline")
            .with_agent(AgentType::any())
            .with_stage(Stage {
                name: "Test Stage".to_string(),
                agent: None,
                environment: Default::default(),
                options: None,
                when: None,
                post: None,
                steps: vec![Step {
                    step_type: StepType::Echo {
                        message: "Hello".to_string(),
                    },
                    name: None,
                    timeout: None,
                    retry: None,
                }],
            })
    }

    #[test]
    fn test_executor_creation() {
        let pipeline = create_test_pipeline();
        let config = ExecutionConfig::default();
        let executor = Executor::new(pipeline, config);
        assert!(executor.validate().is_ok());
    }

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::success(1, 1, chrono::Duration::seconds(10));
        assert!(result.is_success());
        assert!(!result.is_failure());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_execution_result_failure() {
        let result = ExecutionResult::failure(0, 0, chrono::Duration::zero(), "test error");
        assert!(!result.is_success());
        assert!(result.is_failure());
        assert!(result.error.is_some());
    }

    #[test]
    fn test_execution_status() {
        assert_eq!(ExecutionStatus::Pending, ExecutionStatus::Pending);
        assert_ne!(ExecutionStatus::Pending, ExecutionStatus::Running);
    }

    // =======================================================================
    // Task T3.10: ExecutorCapabilities and HealthStatus Tests
    // =======================================================================

    #[test]
    fn test_executor_capabilities_default() {
        let caps = ExecutorCapabilities::default();
        assert!(caps.can_execute_shell);
        assert!(!caps.can_run_docker);
        assert!(!caps.can_run_kubernetes);
        assert!(!caps.supports_parallel);
        assert!(caps.supports_caching);
        assert!(caps.supports_timeout);
        assert!(caps.supports_retry);
    }

    #[test]
    fn test_executor_capabilities_equality() {
        let caps1 = ExecutorCapabilities::default();
        let caps2 = ExecutorCapabilities::default();
        assert_eq!(caps1, caps2);

        let caps3 = ExecutorCapabilities {
            can_execute_shell: false,
            ..Default::default()
        };
        assert_ne!(caps1, caps3);
    }

    #[test]
    fn test_health_status_healthy() {
        let status = HealthStatus::Healthy;
        assert!(status.is_operational());
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_status_degraded() {
        let status = HealthStatus::Degraded {
            reason: "high load".to_string(),
        };
        assert!(status.is_operational());
        assert!(matches!(status, HealthStatus::Degraded { .. }));
    }

    #[test]
    fn test_health_status_unhealthy() {
        let status = HealthStatus::Unhealthy {
            reason: "connection lost".to_string(),
        };
        assert!(!status.is_operational());
        assert!(matches!(status, HealthStatus::Unhealthy { .. }));
    }

    #[test]
    fn test_health_status_is_operational() {
        assert!(HealthStatus::Healthy.is_operational());
        assert!(HealthStatus::Degraded {
            reason: "test".to_string()
        }
        .is_operational());
        assert!(!HealthStatus::Unhealthy {
            reason: "test".to_string()
        }
        .is_operational());
    }

    // =======================================================================
    // Task T3.11: Re-export Tests
    // =======================================================================

    #[test]
    fn test_unified_executor_trait_is_exportable() {
        // Verify the trait itself is accessible and can be used as a bound
        fn _check_executor<E: UnifiedExecutor>(_: &E) {}
        // If this compiles, the trait is properly exported
    }

    #[test]
    fn test_executor_capabilities_from_crate_root() {
        // Verify we can construct ExecutorCapabilities from the re-export
        let caps = ExecutorCapabilities {
            can_execute_shell: true,
            can_run_docker: true,
            can_run_kubernetes: false,
            supports_parallel: true,
            supports_caching: false,
            supports_timeout: true,
            supports_retry: false,
        };
        assert!(caps.can_execute_shell);
        assert!(caps.can_run_docker);
        assert!(!caps.can_run_kubernetes);
        assert!(caps.supports_parallel);
        assert!(!caps.supports_caching);
        assert!(caps.supports_timeout);
        assert!(!caps.supports_retry);
    }

    #[test]
    fn test_health_status_from_crate_root() {
        // Verify we can construct HealthStatus from the re-export
        let healthy = HealthStatus::Healthy;
        let degraded = HealthStatus::Degraded {
            reason: "test".to_string(),
        };
        let unhealthy = HealthStatus::Unhealthy {
            reason: "test".to_string(),
        };
        assert_eq!(healthy, HealthStatus::Healthy);
        assert!(degraded.is_operational());
        assert!(!unhealthy.is_operational());
    }

    // =======================================================================
    // Task T3.5: Executor::run() Tests
    // =======================================================================

    #[tokio::test]
    async fn test_executor_run_basic_success() {
        let pipeline = Pipeline::new()
            .with_name("test-pipeline")
            .with_stage(Stage {
                name: "build".to_string(),
                agent: None,
                environment: Default::default(),
                options: None,
                when: None,
                post: None,
                steps: vec![Step {
                    step_type: StepType::Echo {
                        message: "Hello".to_string(),
                    },
                    name: Some("echo-step".to_string()),
                    timeout: None,
                    retry: None,
                }],
            });

        let config = ExecutionConfig::default();
        let mut executor = Executor::new(pipeline, config);
        let result = executor.run().await.unwrap();

        assert!(result.is_success());
        assert!(result.error.is_none());
        assert_eq!(result.stages_executed, 1);
        assert_eq!(result.steps_executed, 1);
    }

    #[tokio::test]
    async fn test_executor_run_with_failure() {
        let pipeline = Pipeline::new()
            .with_name("test-pipeline")
            .with_stage(Stage {
                name: "build".to_string(),
                agent: None,
                environment: Default::default(),
                options: None,
                when: None,
                post: None,
                steps: vec![Step {
                    step_type: StepType::Shell {
                        command: "exit 1".to_string(),
                    },
                    name: Some("failing-step".to_string()),
                    timeout: None,
                    retry: None,
                }],
            });

        let config = ExecutionConfig::default();
        let mut executor = Executor::new(pipeline, config);
        let result = executor.run().await.unwrap();

        assert!(!result.is_success());
        assert!(result.error.is_some());
        assert_eq!(result.stages_executed, 1);
        assert_eq!(result.steps_executed, 1);
    }

    #[tokio::test]
    async fn test_executor_run_with_retry_on_failure() {
        let pipeline = Pipeline::new()
            .with_name("test-pipeline")
            .with_stage(Stage {
                name: "build".to_string(),
                agent: None,
                environment: Default::default(),
                options: None,
                when: None,
                post: None,
                steps: vec![Step {
                    step_type: StepType::Echo {
                        message: "Hello".to_string(),
                    },
                    name: Some("echo-step".to_string()),
                    timeout: None,
                    retry: None,
                }],
            });

        let config = ExecutionConfig {
            retry_on_failure: true,
            max_retries: 3,
            ..Default::default()
        };
        let mut executor = Executor::new(pipeline, config);
        let result = executor.run().await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.stages_executed, 1);
    }

    #[tokio::test]
    async fn test_executor_run_with_timeout() {
        // Note: Due to std::process::Command being blocking, the timeout may not
        // interrupt a long-running shell command. This test verifies the timeout
        // is set up correctly but may not fail as expected in all cases.
        let pipeline = Pipeline::new()
            .with_name("test-pipeline")
            .with_stage(Stage {
                name: "build".to_string(),
                agent: None,
                environment: Default::default(),
                options: None,
                when: None,
                post: None,
                steps: vec![Step {
                    step_type: StepType::Echo {
                        message: "Fast step".to_string(),
                    },
                    name: Some("fast-step".to_string()),
                    timeout: None,
                    retry: None,
                }],
            });

        let config = ExecutionConfig {
            global_timeout: Some(std::time::Duration::from_secs(30)),
            ..Default::default()
        };
        let mut executor = Executor::new(pipeline, config);
        let result = executor.run().await.unwrap();

        // With a reasonable timeout, the fast step should succeed
        assert!(result.is_success());
        assert!(result.error.is_none());
        assert_eq!(result.stages_executed, 1);
    }

    #[tokio::test]
    async fn test_executor_run_with_multiple_stages() {
        let stage1 = Stage {
            name: "build".to_string(),
            agent: None,
            environment: Default::default(),
            options: None,
            when: None,
            post: None,
            steps: vec![Step {
                step_type: StepType::Echo {
                    message: "Building".to_string(),
                },
                name: Some("build-step".to_string()),
                timeout: None,
                retry: None,
            }],
        };

        let stage2 = Stage {
            name: "test".to_string(),
            agent: None,
            environment: Default::default(),
            options: None,
            when: None,
            post: None,
            steps: vec![Step {
                step_type: StepType::Echo {
                    message: "Testing".to_string(),
                },
                name: Some("test-step".to_string()),
                timeout: None,
                retry: None,
            }],
        };

        let pipeline = Pipeline::new()
            .with_name("test-pipeline")
            .with_stage(stage1)
            .with_stage(stage2);

        let config = ExecutionConfig::default();
        let mut executor = Executor::new(pipeline, config);
        let result = executor.run().await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.stages_executed, 2);
        assert_eq!(result.steps_executed, 2);
    }

    #[tokio::test]
    async fn test_executor_run_validates_pipeline() {
        // An invalid pipeline (no stages) should fail validation
        let pipeline = Pipeline::new().with_name("empty-pipeline");

        let config = ExecutionConfig::default();
        let mut executor = Executor::new(pipeline, config);
        let result = executor.run().await;

        // Should return an error because validation should fail
        assert!(result.is_err() || result.unwrap().error.is_some());
    }

    #[tokio::test]
    async fn test_executor_run_with_shell_command() {
        let pipeline = Pipeline::new()
            .with_name("test-pipeline")
            .with_stage(Stage {
                name: "build".to_string(),
                agent: None,
                environment: Default::default(),
                options: None,
                when: None,
                post: None,
                steps: vec![Step {
                    step_type: StepType::Shell {
                        command: "echo 'Hello World'".to_string(),
                    },
                    name: Some("shell-step".to_string()),
                    timeout: None,
                    retry: None,
                }],
            });

        let config = ExecutionConfig::default();
        let mut executor = Executor::new(pipeline, config);
        let result = executor.run().await.unwrap();

        assert!(result.is_success());
        assert!(result.error.is_none());
    }
}
