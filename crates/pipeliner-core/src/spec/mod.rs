//! Pipeline specification types for declarative pipeline definitions.
//!
//! This module provides types for defining pipelines in a declarative format
//! that can be serialized to/from JSON. These specifications are distinct
//! from the runtime `Pipeline` type in the parent `pipeline` module.
//!
//! ## Example
//!
//! ```rust
//! use pipeliner_core::spec::{
//!     PipelineSpec, StageSpec, PostSpec,
//!     step_spec::{StepSpec, EchoStepSpec, ShellStepSpec, ShellKind},
//! };
//!
//! let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
//!     .with_stage(
//!         StageSpec::new("build", "Build")
//!             .with_steps(vec![
//!                 StepSpec::Shell(ShellStepSpec::new("cargo build")),
//!             ]),
//!     );
//!
//! // Serialize to JSON
//! let json = spec.to_json().unwrap();
//! ```

pub mod env_spec;
pub mod pipeline_spec;
pub mod stage_spec;
pub mod step_spec;

// Re-exports for convenient access
pub use env_spec::EnvSpec;
pub use pipeline_spec::{PipelineSpec, PostSpec};
pub use stage_spec::{minutes, seconds, OptionsSpec, StageExecution, StageSpec};
pub use step_spec::{
    ArchiveStepSpec, CredentialBinding, DirStepSpec, EchoStepSpec, InterpolationMode,
    JUnitStepSpec, LetOutputStepSpec, ShellKind, ShellStepSpec, StepSpec, WithCredentialsStepSpec,
    WithEnvStepSpec,
};
