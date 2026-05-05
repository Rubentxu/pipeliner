//! MavenTool - Maven build tool operations.

use std::process::Command;
use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// MavenTool provides Maven build operations for pipelines.
#[derive(Debug, Clone)]
pub struct MavenTool {
    /// Path to the Maven binary.
    pub mvn_path: String,
}

impl MavenTool {
    /// Creates a new MavenTool with default mvn path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mvn_path: "mvn".to_string(),
        }
    }

    /// Creates a new MavenTool with a custom mvn path.
    #[must_use]
    pub fn with_mvn_path(mvn_path: impl Into<String>) -> Self {
        Self {
            mvn_path: mvn_path.into(),
        }
    }

    fn run_maven_command(&self, args: &[&str]) -> Result<String, StepError> {
        let output = Command::new(&self.mvn_path)
            .args(args)
            .output()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to execute mvn: {}", e),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(StepError::CreationFailed {
                message: format!("Maven command failed: {}", stderr),
            })
        }
    }
}

impl Default for MavenTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for MavenTool {
    fn name(&self) -> &str {
        "maven"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "build", "test", "publish", "build_with_profiles"
        // args[1..] = operation-specific args
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "build" => {
                let result = self.run_maven_command(&["clean", "package", "-B"])?;
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
                let result = self.run_maven_command(&["test", "-B"])?;
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
                let result = self.run_maven_command(&["deploy", "-DskipTests", "-B"])?;
                let output = serde_json::json!({
                    "operation": "publish",
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            "build_with_profiles" => {
                let profiles = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected profiles string argument".to_string(),
                    })?
                    .to_string();

                if profiles.is_empty() {
                    return Err(StepError::InvalidArgs {
                        message: "Expected profiles string (comma-separated)".to_string(),
                    });
                }

                let mut maven_args = vec!["clean", "package"];
                let profile_flag = format!("-P{}", profiles);
                maven_args.push(&profile_flag);
                maven_args.push("-B");

                let result = self.run_maven_command(&maven_args)?;
                let output = serde_json::json!({
                    "operation": "build_with_profiles",
                    "profiles": profiles,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'build', 'test', 'publish', or 'build_with_profiles'",
                    operation
                ),
            }),
        }
    }
}
