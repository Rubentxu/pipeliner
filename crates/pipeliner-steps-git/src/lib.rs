//! # Pipeliner Steps - Git
//!
//! Git step tool for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod git_tool;

pub use git_tool::GitTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // GitTool Tests
    // =====================================================================

    #[test]
    fn test_git_tool_name() {
        let tool = GitTool::new();
        assert_eq!(tool.name(), "gitTool");
    }

    #[test]
    fn test_git_tool_create_clone_operation() {
        let tool = GitTool::new();
        let args = vec![
            JsonValue::String("clone".to_string()),
            JsonValue::String("https://github.com/example/repo".to_string()),
            JsonValue::String("/tmp/repo".to_string()),
        ];
        let result = tool.create(&args);
        // clone will fail without network, but should parse args correctly
        // In test environment, we just verify arg parsing
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_git_tool_create_create_tag_operation() {
        let tool = GitTool::new();
        let args = vec![
            JsonValue::String("createTag".to_string()),
            JsonValue::String("v1.0.0".to_string()),
        ];
        let result = tool.create(&args);
        // May fail without git repo, but verifies arg parsing
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_git_tool_create_tag_exists_operation() {
        let tool = GitTool::new();
        let args = vec![
            JsonValue::String("tagExists".to_string()),
            JsonValue::String("v1.0.0".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_git_tool_create_current_branch_operation() {
        let tool = GitTool::new();
        let args = vec![JsonValue::String("currentBranch".to_string())];
        let result = tool.create(&args);
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_git_tool_create_with_invalid_args_returns_error() {
        let tool = GitTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_git_tool_create_with_unknown_operation_returns_error() {
        let tool = GitTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }
}
