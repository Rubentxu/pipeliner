//! # Pipeliner Steps - Core
//!
//! Config and workspace step tools for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod config_tool;
mod workspace_tool;

pub use config_tool::ConfigTool;
pub use workspace_tool::WorkspaceTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // ConfigTool Tests
    // =====================================================================

    #[test]
    fn test_config_tool_name() {
        let tool = ConfigTool;
        assert_eq!(tool.name(), "config");
    }

    #[test]
    fn test_config_tool_create_load_operation() {
        let tool = ConfigTool;
        let args = vec![JsonValue::String("load".to_string())];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
        assert_eq!(step.name, "config");
    }

    #[test]
    fn test_config_tool_create_load_with_profiles() {
        let tool = ConfigTool;
        let args = vec![
            JsonValue::String("load".to_string()),
            JsonValue::Array(vec![JsonValue::String("dev".to_string())]),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_config_tool_create_with_invalid_args_returns_error() {
        let tool = ConfigTool;
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_config_tool_create_with_unknown_operation_returns_error() {
        let tool = ConfigTool;
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    // =====================================================================
    // WorkspaceTool Tests
    // =====================================================================

    #[test]
    fn test_workspace_tool_name() {
        let tool = WorkspaceTool;
        assert_eq!(tool.name(), "workspace");
    }

    #[test]
    fn test_workspace_tool_create_check_files_operation() {
        let tool = WorkspaceTool;
        let args = vec![
            JsonValue::String("checkFiles".to_string()),
            JsonValue::Array(vec![JsonValue::String("**/*.rs".to_string())]),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_workspace_tool_create_list_files_operation() {
        let tool = WorkspaceTool;
        let args = vec![JsonValue::String("listFiles".to_string())];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_workspace_tool_create_clean_operation() {
        let tool = WorkspaceTool;
        let args = vec![
            JsonValue::String("clean".to_string()),
            JsonValue::String("target".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_workspace_tool_create_with_invalid_args_returns_error() {
        let tool = WorkspaceTool;
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_workspace_tool_create_with_unknown_operation_returns_error() {
        let tool = WorkspaceTool;
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    // =====================================================================
    // CustomStep Integration Tests
    // =====================================================================

    #[test]
    fn test_custom_step_from_config_tool_carries_output() {
        let tool = ConfigTool;
        let args = vec![JsonValue::String("load".to_string())];
        let step = tool.create(&args).unwrap();
        assert!(step.success);
        assert!(step.output.is_some());
    }

    #[test]
    fn test_custom_step_from_workspace_tool_carries_output() {
        let tool = WorkspaceTool;
        let args = vec![JsonValue::String("listFiles".to_string())];
        let step = tool.create(&args).unwrap();
        assert!(step.success);
        assert!(step.output.is_some());
    }
}
