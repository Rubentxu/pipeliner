//! HelmTool - Helm chart operations.

use std::process::Command;
use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// HelmTool provides Helm chart operations for pipelines.
#[derive(Debug, Clone)]
pub struct HelmTool {
    /// Path to the Helm binary.
    pub helm_path: String,
}

impl HelmTool {
    /// Creates a new HelmTool with default helm path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            helm_path: "helm".to_string(),
        }
    }

    /// Creates a new HelmTool with a custom helm path.
    #[must_use]
    pub fn with_helm_path(helm_path: impl Into<String>) -> Self {
        Self {
            helm_path: helm_path.into(),
        }
    }

    fn run_helm_command(&self, args: &[&str]) -> Result<String, StepError> {
        let output = Command::new(&self.helm_path)
            .args(args)
            .output()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to execute helm: {}", e),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(StepError::CreationFailed {
                message: format!("Helm command failed: {}", stderr),
            })
        }
    }
}

impl Default for HelmTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for HelmTool {
    fn name(&self) -> &str {
        "helm"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "build", "test", "deploy", "promote"
        // args[1..] = operation-specific args
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "build" => {
                let chart = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected chart argument".to_string(),
                    })?;

                let result = self.run_helm_command(&["package", chart])?;
                let output = serde_json::json!({
                    "operation": "build",
                    "chart": chart,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            "test" => {
                let chart = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected chart argument".to_string(),
                    })?;

                let result = self.run_helm_command(&["test", chart])?;
                let output = serde_json::json!({
                    "operation": "test",
                    "chart": chart,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            "deploy" => {
                let chart = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected chart argument".to_string(),
                    })?;

                let namespace = args
                    .get(2)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected namespace argument".to_string(),
                    })?;

                let release_name = chart.split('/').last().unwrap_or(chart);
                let mut helm_args = vec!["upgrade", "--install", release_name, chart, "--namespace", namespace];
                helm_args.push("--create-namespace");

                let result = self.run_helm_command(&helm_args)?;
                let output = serde_json::json!({
                    "operation": "deploy",
                    "chart": chart,
                    "namespace": namespace,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            "promote" => {
                let chart = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected chart argument".to_string(),
                    })?;

                let registry = args
                    .get(2)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected registry argument".to_string(),
                    })?;

                // helm registry login and push
                let result = self.run_helm_command(&["push", chart, registry])?;
                let output = serde_json::json!({
                    "operation": "promote",
                    "chart": chart,
                    "registry": registry,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'build', 'test', 'deploy', or 'promote'",
                    operation
                ),
            }),
        }
    }
}
