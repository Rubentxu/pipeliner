//! # Pipeliner Steps - Policy
//!
//! Policy validation steps for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod policy_tool;

pub use policy_tool::PolicyTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // PolicyTool Tests
    // =====================================================================

    #[test]
    fn test_policy_tool_name() {
        let tool = PolicyTool::new();
        assert_eq!(tool.name(), "policy");
    }

    #[test]
    #[ignore = "requires policy engine binary"]
    fn test_policy_tool_validate_operation() {
        let tool = PolicyTool::new();
        let args = vec![
            JsonValue::String("validate".to_string()),
            JsonValue::String("./policies".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
        assert_eq!(step.name, "policy");
    }

    #[test]
    fn test_policy_tool_validate_missing_dir_returns_error() {
        let tool = PolicyTool::new();
        let args = vec![JsonValue::String("validate".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_policy_tool_with_custom_path() {
        let tool = PolicyTool::with_policy_path("/custom/conftest");
        assert_eq!(tool.name(), "policy");
    }

    #[test]
    fn test_policy_tool_unknown_operation_returns_error() {
        let tool = PolicyTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_policy_tool_empty_args_returns_error() {
        let tool = PolicyTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }
}
