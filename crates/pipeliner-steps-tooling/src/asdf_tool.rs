//! AsdfTool - asdf version manager operations.

use std::process::Command;
use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// AsdfTool provides asdf version manager operations for pipelines.
#[derive(Debug, Clone)]
pub struct AsdfTool {
    asdf_path: String,
}

impl AsdfTool {
    /// Creates a new AsdfTool instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            asdf_path: "asdf".to_string(),
        }
    }

    /// Creates a new AsdfTool with a custom asdf path.
    #[must_use]
    pub fn with_asdf_path(asdf_path: impl Into<String>) -> Self {
        Self {
            asdf_path: asdf_path.into(),
        }
    }

    fn run_asdf_command(&self, args: &[&str]) -> Result<String, StepError> {
        let output = Command::new(&self.asdf_path)
            .args(args)
            .output()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to execute asdf: {}", e),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(StepError::CreationFailed {
                message: format!("asdf command failed: {}", stderr),
            })
        }
    }
}

impl Default for AsdfTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for AsdfTool {
    fn name(&self) -> &str {
        "asdfTool"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "install", "current"
        // args[1] = tool name
        // args[2] = version (for install)
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "install" => {
                let tool = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected tool name argument".to_string(),
                    })?;

                let version = args
                    .get(2)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected version argument for install operation".to_string(),
                    })?;

                let output = self.run_asdf_command(&["install", tool, version])?;

                let result = serde_json::json!({
                    "operation": "install",
                    "tool": tool,
                    "version": version
                });

                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&result).unwrap_or(output)),
                ))
            }
            "current" => {
                let tool = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected tool name argument".to_string(),
                    })?;

                let output = self.run_asdf_command(&["current", tool])?;

                let version = output.trim().to_string();
                let result = serde_json::json!({
                    "operation": "current",
                    "tool": tool,
                    "version": version
                });

                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&result).unwrap_or(output)),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'install' or 'current'",
                    operation
                ),
            }),
        }
    }
}
