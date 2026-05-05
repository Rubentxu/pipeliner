//! WorkspaceTool - Workspace file operations.

use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// WorkspaceTool provides operations for workspace file management.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceTool;

impl WorkspaceTool {
    /// Creates a new WorkspaceTool instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl StepFactory for WorkspaceTool {
    fn name(&self) -> &str {
        "workspace"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "checkFiles", "clean", "listFiles"
        // args[1..] = operation-specific args
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "checkFiles" => {
                let patterns: Vec<String> = args
                    .get(1)
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                if patterns.is_empty() {
                    return Err(StepError::InvalidArgs {
                        message: "Expected at least one glob pattern".to_string(),
                    });
                }

                let mut found_files = Vec::new();
                for pattern in &patterns {
                    if let Ok(matches) = glob::glob(pattern) {
                        for entry in matches.flatten() {
                            found_files.push(entry.display().to_string());
                        }
                    }
                }

                let output = serde_json::json!({
                    "operation": "checkFiles",
                    "patterns": patterns,
                    "found": found_files.len(),
                    "files": found_files
                });

                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or_default()),
                ))
            }
            "clean" => {
                let target = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("target");

                let path = std::path::Path::new(target);
                let removed = if path.exists() && path.is_dir() {
                    match remove_dir_all::remove_dir_all(path) {
                        Ok(_) => 1,
                        Err(_) => 0,
                    }
                } else {
                    0
                };

                let output = serde_json::json!({
                    "operation": "clean",
                    "target": target,
                    "removed": removed
                });

                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or_default()),
                ))
            }
            "listFiles" => {
                let output = serde_json::json!({
                    "operation": "listFiles"
                });

                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or_default()),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'checkFiles', 'clean', or 'listFiles'",
                    operation
                ),
            }),
        }
    }
}
