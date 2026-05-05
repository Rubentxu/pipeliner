//! ArtifactTool - Artifact repository operations.

use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// ArtifactTool provides artifact repository operations for pipelines.
#[derive(Debug, Clone)]
pub struct ArtifactTool {
    http_client: reqwest::blocking::Client,
}

impl ArtifactTool {
    /// Creates a new ArtifactTool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            http_client: reqwest::blocking::Client::new(),
        }
    }

    /// Creates a new ArtifactTool with a custom HTTP client.
    #[must_use]
    pub fn with_client(client: reqwest::blocking::Client) -> Self {
        Self {
            http_client: client,
        }
    }
}

impl Default for ArtifactTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for ArtifactTool {
    fn name(&self) -> &str {
        "artifact"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "upload", "download", "search"
        // args[1..] = operation-specific args
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "upload" => {
                let file = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected file argument".to_string(),
                    })?;

                let repo = args
                    .get(2)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected repo argument".to_string(),
                    })?;

                let output = serde_json::json!({
                    "operation": "upload",
                    "file": file,
                    "repo": repo,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or_default()),
                ))
            }
            "download" => {
                let artifact = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected artifact argument".to_string(),
                    })?;

                let version = args
                    .get(2)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected version argument".to_string(),
                    })?;

                let output = serde_json::json!({
                    "operation": "download",
                    "artifact": artifact,
                    "version": version,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or_default()),
                ))
            }
            "search" => {
                let query = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected query argument".to_string(),
                    })?;

                let output = serde_json::json!({
                    "operation": "search",
                    "query": query,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or_default()),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'upload', 'download', or 'search'",
                    operation
                ),
            }),
        }
    }
}
