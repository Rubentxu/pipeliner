//! GradleTool - Gradle build tool operations.

use std::process::Command;
use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// GradleTool provides Gradle build operations for pipelines.
#[derive(Debug, Clone)]
pub struct GradleTool {
    /// Path to the Gradle binary.
    pub gradle_path: String,
}

impl GradleTool {
    /// Creates a new GradleTool with default gradle path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            gradle_path: "gradle".to_string(),
        }
    }

    /// Creates a new GradleTool with a custom gradle path.
    #[must_use]
    pub fn with_gradle_path(gradle_path: impl Into<String>) -> Self {
        Self {
            gradle_path: gradle_path.into(),
        }
    }

    fn run_gradle_command(&self, args: &[&str]) -> Result<String, StepError> {
        let output = Command::new(&self.gradle_path)
            .args(args)
            .output()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to execute gradle: {}", e),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(StepError::CreationFailed {
                message: format!("Gradle command failed: {}", stderr),
            })
        }
    }
}

impl Default for GradleTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for GradleTool {
    fn name(&self) -> &str {
        "gradle"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "build", "test", "publish"
        // args[1..] = operation-specific args
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "build" => {
                let result = self.run_gradle_command(&["build"])?;
                let output = serde_json::json!({
                    "operation": "build",
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            "test" => {
                let result = self.run_gradle_command(&["test"])?;
                let output = serde_json::json!({
                    "operation": "test",
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            "publish" => {
                let result = self.run_gradle_command(&["publish"])?;
                let output = serde_json::json!({
                    "operation": "publish",
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'build', 'test', or 'publish'",
                    operation
                ),
            }),
        }
    }
}
