//! PolicyTool - Policy validation operations.

use std::process::Command;
use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// PolicyTool provides policy validation operations for pipelines.
#[derive(Debug, Clone)]
pub struct PolicyTool {
    /// Path to the policy engine binary.
    pub policy_path: String,
}

impl PolicyTool {
    /// Creates a new PolicyTool with default policy engine path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            policy_path: "conftest".to_string(),
        }
    }

    /// Creates a new PolicyTool with a custom policy engine path.
    #[must_use]
    pub fn with_policy_path(policy_path: impl Into<String>) -> Self {
        Self {
            policy_path: policy_path.into(),
        }
    }

    fn run_policy_command(&self, args: &[&str]) -> Result<String, StepError> {
        let output = Command::new(&self.policy_path)
            .args(args)
            .output()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to execute {}: {}", self.policy_path, e),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(StepError::CreationFailed {
                message: format!("Policy validation failed: {}", stderr),
            })
        }
    }
}

impl Default for PolicyTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for PolicyTool {
    fn name(&self) -> &str {
        "policy"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "validate"
        // args[1] = policies_dir
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "validate" => {
                let policies_dir = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected policies_dir argument".to_string(),
                    })?;

                let result = self.run_policy_command(&["test", policies_dir])?;
                let output = serde_json::json!({
                    "operation": "validate",
                    "policies_dir": policies_dir,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'validate'",
                    operation
                ),
            }),
        }
    }
}
