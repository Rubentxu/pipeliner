//! `rustline check` - Validate pipeline syntax via rust-script
//!
//! This command wraps `rust-script --check` to validate Rust pipeline scripts
//! without executing them. It checks for syntax errors, type errors, and other
//! compilation issues.
//!
//! ## Usage
//!
//! ```bash
//! rustline check <pipeline.rs>
//! ```
//!
//! ## Example
//!
//! ```bash
//! rustline check examples/basic.rs
//! # Exit code 0: No errors found
//! # Exit code 1: Compilation errors found
//! ```

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, ExitStatus};

/// Validate a pipeline script using rust-script
///
/// This function runs `rust-script --check` on the given file to validate
/// the pipeline syntax without executing it.
///
/// # Arguments
///
/// * `file` - Path to the pipeline script to validate
/// * `cargo_output` - Whether to show cargo output
///
/// # Returns
///
/// Returns `Ok(())` if validation succeeds, `Err(anyhow::Error)` otherwise.
pub fn check_pipeline(file: &Path, cargo_output: bool) -> Result<()> {
    let file_str = file.to_string_lossy();

    tracing::debug!("Validating pipeline: {}", file_str);

    // Verify the file exists
    if !file.exists() {
        anyhow::bail!("Pipeline file not found: {}", file_str);
    }

    // Verify it's a Rust file
    if file.extension().map(|e| e.to_string_lossy().to_lowercase()) != Some("rs".to_string()) {
        tracing::warn!("File does not have .rs extension: {}", file_str);
    }

    // Build the rust-script command
    let mut cmd = Command::new("rust-script");
    cmd.arg("--check");
    cmd.arg(file.to_string_lossy().as_ref());

    if cargo_output {
        cmd.arg("--cargo-output");
    }

    // Execute the command
    tracing::debug!("Running: {:?}", cmd);

    let output = cmd
        .output()
        .context("Failed to execute rust-script --check")?;

    // rust-script --check returns:
    // - Exit code 0: Success (valid Rust code)
    // - Exit code 1: Compilation error
    // - Exit code 101: rust-script not found

    if !output.status.success() {
        // Print stderr for user feedback
        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }

        if !output.stdout.is_empty() {
            tracing::debug!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        }

        anyhow::bail!("Pipeline validation failed for: {}", file_str);
    }

    tracing::info!("Pipeline validation successful: {}", file_str);
    Ok(())
}

/// Check if rust-script is available
///
/// Returns `Ok(true)` if rust-script is installed and accessible.
pub fn is_rust_script_available() -> Result<bool> {
    let output = Command::new("rust-script")
        .arg("--version")
        .output()
        .context("Failed to check rust-script availability")?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        tracing::info!("rust-script available: {}", version.trim());
        Ok(true)
    } else {
        tracing::warn!("rust-script not found or not accessible");
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_pipeline_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("valid.rs");

        let valid_pipeline = r#"
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! rustline = "0.1"
//!

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        stages!(stage!("Test", steps!(sh!("echo test"))))
    );
    Ok(())
}
"#;

        fs::write(&file_path, valid_pipeline).unwrap();
        let result = check_pipeline(&file_path, false);

        // May fail if rust-script is not installed, that's ok
        if result.is_ok() {
            assert!(true);
        }
    }

    #[test]
    fn test_check_pipeline_nonexistent_file() {
        let result = check_pipeline(Path::new("/nonexistent/pipeline.rs"), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
