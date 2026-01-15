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
pub mod listener;
pub mod runtime;
pub mod strategy;

pub use context::{ExecutionConfig, ExecutionContext};
pub use listener::ExecutionListener;
pub use runtime::StepExecutor;
pub use strategy::{ExecutionStrategy, ParallelStrategy, SequentialStrategy};

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
        todo!()
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
}
