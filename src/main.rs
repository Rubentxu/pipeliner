//! rustline - CLI tools for Jenkins Pipeline DSL in Rust
//!
//! This binary delegates to the modern pipeliner CLI.
//! The legacy rustline-specific commands (check, lint, doc, export)
//! for .rs DSL files are available via the pipeliner CLI.

use std::process::ExitCode;

fn main() -> ExitCode {
    // Initialize tracing for debugging
    if std::env::var("RUSTLINE_DEBUG").is_ok() {
        tracing_subscriber::fmt::init();
    }

    // Delegate to the modern workspace CLI
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
        eprintln!("Failed to create tokio runtime: {}", e);
        std::process::exit(1);
    });

    match rt.block_on(pipeliner_cli::commands::run()) {
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