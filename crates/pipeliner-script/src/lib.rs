//! # Pipeliner Script - Execute Rust Scripts as Pipeline Steps
//!
//! This crate provides the `ScriptStepFactory` which enables executing
//! Rust scripts as part of a pipeline, similar to how Groovy scripts
//! work in Jenkins.
//!
//! ## Overview
//!
//! The script execution system:
//! 1. Reads a Rust script file (`.rs`)
//! 2. Extracts dependencies from manifest comments (`//!`)
//! 3. Generates a Cargo project for the script
//! 4. Compiles the script to a binary
//! 5. Executes the binary with pipeline context
//! 6. Caches compiled binaries for reuse
//!
//! ## Script Manifest
//!
//! Scripts can declare dependencies using manifest comments:
//!
//! ```rust
//! #!/usr/bin/env rustline-run
//! //! [dependencies]
//! //! serde = "1.0"
//! //! serde_json = "1.0"
//!
//! fn main() {
//!     println!("Hello from script!");
//! }
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pipeliner_script::ScriptStepFactory;
//! use pipeliner_core::registry::StepRegistry;
//! use std::sync::Arc;
//!
//! let mut registry = StepRegistry::new();
//! registry.register(Arc::new(ScriptStepFactory::new()));
//! ```

pub mod cache;
pub mod compiler;
pub mod manifest;
pub mod runner;

pub use cache::ScriptCache;
pub use compiler::ScriptCompiler;
pub use manifest::{Manifest, ManifestError};
pub use runner::{ScriptOutput, ScriptResult, ScriptRunner};
pub use step_factory::ScriptStepFactory;

mod step_factory;

// Re-export StepFactory trait for convenience
pub use pipeliner_core::registry::StepFactory;

/// Script execution error types.
#[derive(Debug, Clone)]
pub enum ScriptError {
    /// Script file not found
    ScriptNotFound(String),
    /// Script compilation failed
    CompilationFailed { script: String, output: String },
    /// Script execution failed
    ExecutionFailed { script: String, exit_code: i32 },
    /// Error parsing manifest
    ManifestParseError(String),
    /// Cache error
    CacheError(String),
    /// I/O error
    IoError(String),
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptError::ScriptNotFound(path) => {
                write!(f, "Script not found: {}", path)
            }
            ScriptError::CompilationFailed { script, output } => {
                write!(f, "Compilation failed for '{}': {}", script, output)
            }
            ScriptError::ExecutionFailed { script, exit_code } => {
                write!(f, "Script '{}' failed with exit code {}", script, exit_code)
            }
            ScriptError::ManifestParseError(msg) => {
                write!(f, "Manifest parse error: {}", msg)
            }
            ScriptError::CacheError(msg) => {
                write!(f, "Cache error: {}", msg)
            }
            ScriptError::IoError(msg) => {
                write!(f, "I/O error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ScriptError {}

impl From<std::io::Error> for ScriptError {
    fn from(err: std::io::Error) -> Self {
        ScriptError::IoError(err.to_string())
    }
}

/// Script step configuration for the DSL.
#[derive(Debug, Clone)]
pub struct ScriptStepConfig {
    /// Path to the script file (relative or absolute)
    pub path: String,
    /// Inline dependencies to merge with manifest deps
    pub deps: Vec<String>,
    /// Working directory for script execution
    pub workdir: Option<String>,
    /// Environment variables to set
    pub env: std::collections::HashMap<String, String>,
}

impl Default for ScriptStepConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            deps: Vec::new(),
            workdir: None,
            env: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_error_display() {
        let err = ScriptError::ScriptNotFound("missing.rs".to_string());
        assert!(err.to_string().contains("missing.rs"));

        let err = ScriptError::CompilationFailed {
            script: "test.rs".to_string(),
            output: "error: expected ;".to_string(),
        };
        assert!(err.to_string().contains("test.rs"));
        assert!(err.to_string().contains("expected ;"));

        let err = ScriptError::ExecutionFailed {
            script: "test.rs".to_string(),
            exit_code: 42,
        };
        assert!(err.to_string().contains("exit code 42"));
    }

    #[test]
    fn test_script_step_config_default() {
        let config = ScriptStepConfig::default();
        assert!(config.path.is_empty());
        assert!(config.deps.is_empty());
        assert!(config.workdir.is_none());
        assert!(config.env.is_empty());
    }

    #[test]
    fn test_script_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let script_err: ScriptError = io_err.into();
        assert!(matches!(script_err, ScriptError::IoError(_)));
    }
}