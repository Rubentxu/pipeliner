//! # Pipeliner Runtime - Local Pipeline Execution
//!
//! This crate provides the local execution runtime for Pipeliner pipelines.
//! It executes PipelineSpec on the local machine with support for:
//!
//! - Stage sequencing (run stages in order)
//! - Parallel execution (run independent stages concurrently)
//! - Stage retry logic
//! - Error handling and early exit
//! - Event emission for pipeline progress
//! - Execution reporting
//!
//! ## Architecture
//!
//! The runtime is organized into several modules:
//!
//! - [`local_executor`] - Main executor for running pipelines locally
//! - [`events`] - Event system for pipeline progress tracking
//! - [`report`] - Execution reporting (JSON, human-readable)
//!
//! ## Example
//!
//! ```rust,ignore
//! use pipeliner_runtime::{LocalExecutor, ReportFormat};
//! use pipeliner_core::spec::{PipelineSpec, StageSpec, StepSpec};
//!
//! let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
//!     .with_stage(
//!         StageSpec::new("build", "Build")
//!             .with_steps(vec![StepSpec::Echo(EchoStepSpec {
//!                 message: "Building...".to_string(),
//!             })]),
//!     );
//!
//! let executor = LocalExecutor::new();
//! let result = executor.execute(&spec).await;
//! ```

pub mod events;
pub mod local_executor;
pub mod report;

pub use events::{EventEmitter, EventSubscription, JsonlEventWriter, PipelineEvent};
pub use local_executor::{LocalExecutor, ExecutorConfig, ExecutionResult, StageResult, StepResult, EnvContext, interpolate};
pub use report::{ExecutionReport, ReportGenerator, ReportFormat, StepTiming, StageTiming};
