//! # Pipeliner Steps - Tooling
//!
//! Tooling (asdf) step tool for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod asdf_tool;

pub use asdf_tool::AsdfTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // AsdfTool Tests
    // =====================================================================

    #[test]
    fn test_asdf_tool_name() {
        let tool = AsdfTool::new();
        assert_eq!(tool.name(), "asdfTool");
    }

    #[test]
    fn test_asdf_tool_create_install_operation() {
        let tool = AsdfTool::new();
        let args = vec![
            JsonValue::String("install".to_string()),
            JsonValue::String("nodejs".to_string()),
            JsonValue::String("20.0.0".to_string()),
        ];
        let result = tool.create(&args);
        // install will fail if asdf not installed, but verifies arg parsing
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_asdf_tool_create_current_operation() {
        let tool = AsdfTool::new();
        let args = vec![
            JsonValue::String("current".to_string()),
            JsonValue::String("nodejs".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_asdf_tool_create_with_invalid_args_returns_error() {
        let tool = AsdfTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_asdf_tool_create_with_unknown_operation_returns_error() {
        let tool = AsdfTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_asdf_tool_create_install_without_version_returns_error() {
        let tool = AsdfTool::new();
        let args = vec![
            JsonValue::String("install".to_string()),
            JsonValue::String("nodejs".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }
}
