//! # Rustline - A Jenkins Pipeline DSL in Rust
//!
//! Rustline is a Domain Specific Language (DSL) that replicates the syntax
//! and semantics of the Jenkins Pipeline DSL, executable via rust-script
//! for CI/CD automation directly from the Rust ecosystem.
//!
//! ## Quick Start
//!
//! For usage examples, see the [examples directory](https://github.com/rustline-org/rustline/tree/main/examples).
//!
//! ## Features
//!
//! - **Type-safe DSL**: Leverage Rust's type system for compile-time validation
//! - **Multiple Executors**: Local, Docker, Kubernetes, GitHub Actions, GitLab CI
//! - **Parallel Execution**: Run stages concurrently with `parallel!` and `matrix!`
//! - **Control Flow**: `retry`, `timeout`, `stash`, `unstash`, `when` conditions
//! - **Observability**: Built-in metrics and tracing support
//!
//! ## Documentation
//!
//! - [Full Documentation](https://docs.rs/rustline)
//! - [GitHub Repository](https://github.com/rustline-org/rustline)
//! - [Study Document](https://github.com/rustline-org/rustline/docs/rust-jenkins-dsl-study.md)
//!
//! ## License
//!
//! Licensed under either of
//! - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
//! - MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)
//!
//! at your option.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod macros;

pub mod executor;
pub mod infrastructure;
pub mod pipeline;

// Prelude module for common imports
pub mod prelude;

// Re-export commonly used types
pub use executor::{
    ExecutorCapabilities, HealthStatus, JenkinsPathResolver, LocalExecutor, PipelineContext,
    PipelineExecutor, ShellCommand, ShellConfig, ShellResult, TempFileManager, expand_variables,
    jenkins_shell_config,
};
pub use infrastructure::{
    Config, ContainerExecutor, ContainerRuntime, DockerExecutor, GitHubActionsBackend,
    GitLabCIBackend, KubernetesExecutor, MetricsCollector, PipelineMetrics, PodmanError,
    PodmanExecutor,
};
pub use pipeline::{
    AgentType, DockerConfig, Environment, KubernetesConfig, Parameters, Pipeline, PipelineBuilder,
    PipelineOptions, PodmanConfig, PostCondition, Stage, StageBuilder, StageResult, Step, StepType,
    Trigger, Validate, WhenCondition,
};

/// Version of the rustline crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
