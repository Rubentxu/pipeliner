//! Error types for pipeline domain

use thiserror::Error;

/// Errors that can occur during pipeline operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PipelineError {
    /// Validation failed with specified reason
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationError),

    /// Stage execution failed
    #[error("Stage '{stage}' failed: {error}")]
    StageFailed {
        /// Name of the stage that failed.
        stage: String,
        /// Error message describing the failure.
        error: String,
    },

    /// Command execution failed
    #[error("Command failed with exit code {code}: {stderr}")]
    CommandFailed {
        /// Exit code returned by the command.
        code: i32,
        /// Standard error output from the command.
        stderr: String,
    },

    /// Timeout exceeded
    #[error("Timeout after {duration:?}")]
    Timeout {
        /// Duration before timeout.
        duration: std::time::Duration,
    },

    /// IO error occurred
    #[error("IO error: {0}")]
    Io(String),

    /// Agent configuration error
    #[error("Agent configuration error: {0}")]
    AgentConfig(String),
}

impl From<std::io::Error> for PipelineError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Validation errors for pipeline components
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Name cannot be empty
    #[error("Name cannot be empty")]
    EmptyName,

    /// Name too long
    #[error("Name too long: max {max} characters, got {len}")]
    NameTooLong {
        /// Maximum allowed length.
        max: usize,
        /// Actual length of the name.
        len: usize,
    },

    /// Invalid characters in name
    #[error("Invalid characters in name: '{name}'")]
    InvalidNameChars {
        /// The invalid name.
        name: String,
    },

    /// Pipeline must have at least one stage
    #[error("Pipeline must have at least one stage")]
    EmptyPipeline,

    /// Stage must have at least one step
    #[error("Stage '{stage}' must have at least one step")]
    EmptyStage {
        /// Name of the empty stage.
        stage: String,
    },

    /// Invalid timeout value
    #[error("Invalid timeout: must be positive, got {value}")]
    InvalidTimeout {
        /// The invalid timeout value.
        value: u64,
    },

    /// Invalid retry count
    #[error("Invalid retry count: must be positive, got {value}")]
    InvalidRetryCount {
        /// The invalid retry count.
        value: usize,
    },

    /// Invalid agent type
    #[error("Invalid agent type: {0}")]
    InvalidAgentType(String),

    /// Invalid cron expression
    #[error("Invalid cron expression: '{0}'")]
    InvalidCronExpression(String),
}
