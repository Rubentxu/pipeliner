//! ScannerTool - Security scanner operations.

use std::process::Command;
use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// ScannerTool provides security scanner operations for pipelines.
#[derive(Debug, Clone)]
pub struct ScannerTool {
    /// Path to the scanner binary.
    pub scanner_path: String,
}

impl ScannerTool {
    /// Creates a new ScannerTool with default scanner path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scanner_path: "trivy".to_string(),
        }
    }

    /// Creates a new ScannerTool with a custom scanner path.
    #[must_use]
    pub fn with_scanner_path(scanner_path: impl Into<String>) -> Self {
        Self {
            scanner_path: scanner_path.into(),
        }
    }

    fn run_scanner_command(&self, args: &[&str]) -> Result<String, StepError> {
        let output = Command::new(&self.scanner_path)
            .args(args)
            .output()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to execute {}: {}", self.scanner_path, e),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(StepError::CreationFailed {
                message: format!("Scanner command failed: {}", stderr),
            })
        }
    }
}

impl Default for ScannerTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for ScannerTool {
    fn name(&self) -> &str {
        "scanner"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "execute"
        // args[1] = tool name
        // args[2] = optional config path
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "execute" => {
                let tool = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected tool argument".to_string(),
                    })?;

                let config = args.get(2).and_then(|v| v.as_str());

                let mut scanner_args = vec![tool];
                if let Some(cfg) = config {
                    scanner_args.push("--config");
                    scanner_args.push(cfg);
                }

                let result = self.run_scanner_command(&scanner_args)?;
                let output = serde_json::json!({
                    "operation": "execute",
                    "tool": tool,
                    "config": config,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'execute'",
                    operation
                ),
            }),
        }
    }
}
