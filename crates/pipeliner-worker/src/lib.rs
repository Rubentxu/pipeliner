//! # Pipeliner Worker
//!
//! Background worker for Pipeliner job processing. This crate provides
//! the job queue, worker pool, and state management for pipeline execution.
//!
//! ## Architecture
//!
//! The worker is organized around:
//!
//! - `queue`: Job queue implementation
//! - `pool`: Worker pool management
//! - `state`: Execution state tracking
//! - `scheduler`: Job scheduling logic
//!
//! ## Example
//!
//! ```rust,ignore
//! use pipeliner_worker::{Worker, JobQueue, WorkerConfig};
//!
//! let queue = JobQueue::new();
//! let config = WorkerConfig::default();
//! let worker = Worker::new(queue.clone(), config);
//! worker.start().await;
//! ```

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

pub mod pool;
pub mod queue;
pub mod scheduler;
pub mod state;

pub use pool::{Worker, WorkerConfig, WorkerPool};
pub use queue::{Job, JobPriority, JobQueue, JobStatus};
pub use scheduler::{Scheduler, SchedulingStrategy};
pub use state::ExecutionState;

/// Re-exports
pub use pipeliner_core::{Pipeline, Stage, Step, pipeline};
pub use pipeliner_executor::{ExecutionContext, ExecutionResult};

/// Worker error types
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct WorkerError(#[from] WorkerErrorKind);

#[derive(Debug, thiserror::Error)]
pub enum WorkerErrorKind {
    #[error("job not found: {id}")]
    JobNotFound { id: String },

    #[error("job cancelled: {id}")]
    JobCancelled { id: String },

    #[error("worker unavailable")]
    WorkerUnavailable,

    #[error("queue full")]
    QueueFull,

    #[error("timeout: {reason}")]
    Timeout { reason: String },

    #[error("execution failed: {reason}")]
    ExecutionFailed { reason: String },
}

/// Worker result type
pub type WorkerResult<T = ()> = Result<T, WorkerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_queue_creation() {
        let queue = JobQueue::new();
        assert!(queue.len() == 0);
    }

    #[test]
    fn test_job_status_variants() {
        assert_ne!(JobStatus::Pending, JobStatus::Running);
        assert_ne!(JobStatus::Running, JobStatus::Completed);
    }
}
