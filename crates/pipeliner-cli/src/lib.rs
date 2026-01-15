//! # Pipeliner CLI
//!
//! Command-line interface for Pipeliner pipeline execution.
//!
//! ## Usage
//!
//! ```bash
//! # Run a pipeline
//! pipeliner run --file pipeline.jenkins
//!
//! # Validate a pipeline
//! pipeliner validate --file pipeline.jenkins
//!
//! # Generate shell completions
//! pipeliner completions --shell bash
//! ```

#![warn(missing_docs)]
#![warn(unused)]

pub mod commands;

pub use commands::run;

/// CLI result type
pub type CliResult<T = ()> = Result<T, anyhow::Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_result_ok() {
        let result: CliResult<()> = Ok(());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_result_err() {
        let result: CliResult<()> = Err(anyhow::anyhow!("test error"));
        assert!(result.is_err());
    }
}
