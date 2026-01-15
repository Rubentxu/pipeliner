//! # Pipeliner Core
//!
//! Core domain types and Pipeline DSL for Pipeliner.
//!
//! This crate defines the fundamental types for pipeline definition,
//! validation, and execution strategies.
//!
//! ## Architecture
//!
//! The core is organized in bounded contexts:
//!
//! - `pipeline`: Pipeline, Stage, Step definitions
//! - `agent`: Agent types (Docker, Kubernetes, Podman, etc.)
//! - `environment`: Environment variable handling
//! - `validation`: Pipeline validation rules
//! - `matrix`: Matrix execution configuration
//!
//! ## Example
//!
//! ```rust
//! use pipeliner_core::pipeline::{Pipeline, Stage, Step, StepType};
//! use pipeliner_core::agent::AgentType;
//! use pipeliner_core::environment::Environment;
//! use pipeliner_core::Validate;
//!
//! let pipeline = Pipeline::new()
//!     .with_name("My Pipeline")
//!     .with_agent(AgentType::any())
//!     .with_stage(
//!         Stage {
//!             name: "Build".to_string(),
//!             agent: None,
//!             environment: Environment::new(),
//!             options: None,
//!             when: None,
//!             post: None,
//!             steps: vec![Step {
//!                 step_type: StepType::Shell {
//!                     command: "cargo build --release".to_string(),
//!                 },
//!                 name: Some("build".to_string()),
//!                 timeout: None,
//!                 retry: None,
//!             }],
//!         },
//!     )
//!     .with_stage(
//!         Stage {
//!             name: "Test".to_string(),
//!             agent: None,
//!             environment: Environment::new(),
//!             options: None,
//!             when: None,
//!             post: None,
//!             steps: vec![Step {
//!                 step_type: StepType::Shell {
//!                     command: "cargo test".to_string(),
//!                 },
//!                 name: Some("test".to_string()),
//!                 timeout: None,
//!                 retry: None,
//!             }],
//!         },
//!     );
//!
//! assert!(pipeline.validate().is_ok());
//! ```

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

pub mod agent;
pub mod environment;
pub mod matrix;
pub mod options;
pub mod parameters;
pub mod pipeline;
pub mod prelude;
pub mod validation;

// Re-exports for common use
pub use agent::{AgentConfig, AgentType, DockerConfig, KubernetesConfig, PodmanConfig};
pub use environment::{Environment, VariableResolver};
pub use matrix::{MatrixAxis, MatrixConfig, MatrixExclude};
pub use options::{PipelineOptions, Retry, Timeout, Trigger};
pub use parameters::{ParameterType, Parameters};
pub use pipeline::{Pipeline, Stage, Step, StepType};
pub use validation::{Validate, ValidationError, ValidationResult};

// Version
/// Pipeliner Core version
pub const VERSION: &str = "0.1.0";
