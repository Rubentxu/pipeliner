//! rustline - CLI tools for Jenkins Pipeline DSL in Rust
//!
//! A set of command-line utilities that work with rustline pipelines
//! and integrate with rust-script for execution.
//!
//! ## Commands
//!
//! - `rustline check` - Validate pipeline syntax via rust-script
//! - `rustline lint` - Analyze pipelines for best practices
//! - `rustline doc` - Generate documentation from pipeline comments
//! - `rustline export` - Convert pipelines to CI/CD formats
//! - `rustline completions` - Generate shell completions
//! - `rustline run` - Execute pipelines via rust-script
//!
//! ## Installation
//!
//! ```bash
//! cargo install rustline
//! ```
//!
//! ## Quick Start
//!
//! ```bash
//! # Validate a pipeline
//! rustline check pipeline.rs
//!
//! # Check for best practices
//! rustline lint pipeline.rs
//!
//! # Generate documentation
//! rustline doc pipeline.rs -o README.md
//!
//! # Export to GitHub Actions
//! rustline export pipeline.rs --format=github -o .github/workflows/ci.yml
//!
//! # Generate shell completions
//! rustline completions bash > /etc/bash_completion.d/rustline
//! ```
//!
//! ## See Also
//!
//! - [rustline crate](https://crates.io/crates/rustline) - The core DSL library
//! - [rust-script](https://rust-script.org) - Script runner for Rust

use anyhow::Result;
use std::process::ExitCode;

mod cli;

fn main() -> ExitCode {
    // Initialize tracing for debugging
    if std::env::var("RUSTLINE_DEBUG").is_ok() {
        tracing_subscriber::fmt::init();
    }

    // Run the CLI
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            if std::env::var("RUSTLINE_VERBOSE").is_ok() {
                eprintln!("{:?}", e);
            }
            ExitCode::FAILURE
        }
    }
}
