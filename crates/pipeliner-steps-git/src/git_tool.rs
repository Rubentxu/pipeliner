//! GitTool - Git operations for pipelines.

use std::process::Command;
use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// GitTool provides Git operations for pipelines.
#[derive(Debug, Clone)]
pub struct GitTool {
    git_path: String,
}

impl GitTool {
    /// Creates a new GitTool instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            git_path: "git".to_string(),
        }
    }

    /// Creates a new GitTool with a custom git path.
    #[must_use]
    pub fn with_git_path(git_path: impl Into<String>) -> Self {
        Self {
            git_path: git_path.into(),
        }
    }

    fn run_git_command(&self, args: &[&str]) -> Result<String, StepError> {
        let output = Command::new(&self.git_path)
            .args(args)
            .output()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to execute git: {}", e),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(StepError::CreationFailed {
                message: format!("Git command failed: {}", stderr),
            })
        }
    }
}

impl Default for GitTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for GitTool {
    fn name(&self) -> &str {
        "gitTool"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "clone", "createTag", "tagExists", "currentBranch"
        // args[1..] = operation-specific args
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "clone" => {
                let url = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected URL argument".to_string(),
                    })?;

                let dir = args.get(2).and_then(|v| v.as_str());

                let mut git_args = vec!["clone"];
                // git clone takes: git clone <url> [<directory>]
                // URL must come before directory
                git_args.push(url);
                if let Some(directory) = dir {
                    git_args.push(directory);
                }

                let result = self.run_git_command(
                    &git_args.iter().map(|s| *s).collect::<Vec<_>>()
                );

                match result {
                    Ok(output) => Ok(CustomStep::success(self.name(), Some(output))),
                    Err(e) => Err(e),
                }
            }
            "createTag" => {
                let tag = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected tag argument".to_string(),
                    })?;

                let output = self.run_git_command(&["tag", "-a", tag, "-m", tag])?;

                let result = serde_json::json!({
                    "operation": "createTag",
                    "tag": tag
                });

                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&result).unwrap_or(output)),
                ))
            }
            "tagExists" => {
                let tag = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected tag argument".to_string(),
                    })?;

                let result = self.run_git_command(&["rev-parse", &format!("tags/{}", tag)]);

                let exists = result.is_ok();
                let output = serde_json::json!({
                    "operation": "tagExists",
                    "tag": tag,
                    "exists": exists
                });

                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or_default()),
                ))
            }
            "currentBranch" => {
                let output = self.run_git_command(&["rev-parse", "--abbrev-ref", "HEAD"])?;

                let branch = output.trim().to_string();
                let result = serde_json::json!({
                    "operation": "currentBranch",
                    "branch": branch
                });

                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&result).unwrap_or(output)),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'clone', 'createTag', 'tagExists', or 'currentBranch'",
                    operation
                ),
            }),
        }
    }
}
