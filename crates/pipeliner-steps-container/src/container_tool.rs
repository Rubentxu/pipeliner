//! ContainerTool - Container build and push operations.

use std::process::Command;
use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// ContainerTool provides container build and push operations for pipelines.
#[derive(Debug, Clone)]
pub struct ContainerTool {
    /// Path to the container binary (docker/podman).
    pub container_path: String,
}

impl ContainerTool {
    /// Creates a new ContainerTool with default docker path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            container_path: "docker".to_string(),
        }
    }

    /// Creates a new ContainerTool with a custom container binary path.
    #[must_use]
    pub fn with_container_path(container_path: impl Into<String>) -> Self {
        Self {
            container_path: container_path.into(),
        }
    }

    fn run_container_command(&self, args: &[&str]) -> Result<String, StepError> {
        let output = Command::new(&self.container_path)
            .args(args)
            .output()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to execute {}: {}", self.container_path, e),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(StepError::CreationFailed {
                message: format!("Container command failed: {}", stderr),
            })
        }
    }
}

impl Default for ContainerTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for ContainerTool {
    fn name(&self) -> &str {
        "container"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "build", "promote"
        // args[1..] = operation-specific args
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "build" => {
                let image = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected image name argument".to_string(),
                    })?;

                let dockerfile = args.get(2).and_then(|v| v.as_str());

                let mut docker_args = vec!["build"];
                if let Some(df) = dockerfile {
                    docker_args.push("-f");
                    docker_args.push(df);
                }
                docker_args.push("-t");
                docker_args.push(image);
                docker_args.push(".");

                let result = self.run_container_command(&docker_args)?;
                let output = serde_json::json!({
                    "operation": "build",
                    "image": image,
                    "dockerfile": dockerfile,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or(result)),
                ))
            }
            "promote" => {
                let image = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected image argument".to_string(),
                    })?;

                let registry = args
                    .get(2)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected registry argument".to_string(),
                    })?;

                // docker tag image registry/image
                let tag_source = image;
                let tag_target = format!("{}/{}", registry, image);
                self.run_container_command(&["tag", tag_source, &tag_target])?;

                // docker push registry/image
                let result = self.run_container_command(&["push", &tag_target])?;
                let output = serde_json::json!({
                    "operation": "promote",
                    "image": image,
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
                    "Unknown operation: '{}'. Expected 'build' or 'promote'",
                    operation
                ),
            }),
        }
    }
}
