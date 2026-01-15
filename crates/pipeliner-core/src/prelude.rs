//! prelude - Common imports for Pipeliner
//!
//! This module provides convenient re-exports and macros for
//! everyday pipeline development.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pipeliner_core::prelude::*;
//!
//! let pipeline = Pipeline::new()
//!     .with_agent(AgentType::any())
//!     .with_stage(Stage::new("Build").with_step(Step::shell("echo hello")));
//!
//! run!(pipeline);
//! ```

/// Execute a pipeline with local executor
///
/// # Example
///
/// ```rust,ignore
/// use pipeliner_core::prelude::*;
///
/// let pipeline = Pipeline::new()
///     .with_agent(AgentType::any())
///     .with_stage(Stage::new("Test").with_step(Step::shell("cargo test")));
/// run!(pipeline);
/// ```
#[macro_export]
macro_rules! run {
    ($pipeline:expr) => {{
        use pipeliner_executor::LocalExecutor;
        let executor = LocalExecutor::new();
        let results = executor.execute(&$pipeline).await;
        let success = results.iter().all(|r| r.success);
        if !success {
            eprintln!("Pipeline failed!");
            std::process::exit(1);
        }
    }};
}

/// Execute a pipeline with local executor (blocking)
///
/// For use in non-async contexts
#[macro_export]
macro_rules! run_sync {
    ($pipeline:expr) => {{
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            use pipeliner_executor::LocalExecutor;
            let executor = LocalExecutor::new();
            let results = executor.execute(&$pipeline).await;
            let success = results.iter().all(|r| r.success);
            if !success {
                eprintln!("Pipeline failed!");
                std::process::exit(1);
            }
        })
    }};
}

pub use crate::agent::{AgentConfig, AgentType, DockerConfig, KubernetesConfig, PodmanConfig};
pub use crate::environment::{Environment, VariableResolver};
pub use crate::matrix::{MatrixAxis, MatrixConfig, MatrixExclude};
pub use crate::options::{PipelineOptions, Retry, Timeout, Trigger};
pub use crate::parameters::{ParameterType, Parameters};
pub use crate::pipeline::{Pipeline, Stage, Step, StepType};
pub use crate::validation::{Validate, ValidationError, ValidationResult};
