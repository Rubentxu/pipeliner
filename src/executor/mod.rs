//! Pipeline execution layer
//!
//! This module contains traits and implementations for executing pipelines.

mod local;
mod shell;
mod temp_files;
mod traits;

pub use local::{ExecutorConfig, LocalExecutor};
pub use shell::{ShellCommand, ShellConfig, ShellResult, expand_variables, jenkins_shell_config};
pub use temp_files::{JenkinsPathResolver, TempFileManager};
pub use traits::{ExecutorCapabilities, HealthStatus, PipelineContext, PipelineExecutor};
