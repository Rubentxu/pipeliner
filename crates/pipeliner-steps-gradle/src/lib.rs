//! # Pipeliner Steps - Gradle
//!
//! Gradle build tool steps for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod gradle_tool;

pub use gradle_tool::GradleTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // GradleTool Tests
    // =====================================================================

    #[test]
    fn test_gradle_tool_name() {
        let tool = GradleTool::new();
        assert_eq!(tool.name(), "gradle");
    }

    #[test]
    #[ignore = "requires gradle binary"]
    fn test_gradle_tool_build_operation() {
        let tool = GradleTool::new();
        let args = vec![JsonValue::String("build".to_string())];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
        assert_eq!(step.name, "gradle");
    }

    #[test]
    #[ignore = "requires gradle binary"]
    fn test_gradle_tool_test_operation() {
        let tool = GradleTool::new();
        let args = vec![JsonValue::String("test".to_string())];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    #[ignore = "requires gradle binary"]
    fn test_gradle_tool_publish_operation() {
        let tool = GradleTool::new();
        let args = vec![JsonValue::String("publish".to_string())];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_gradle_tool_with_custom_path() {
        let tool = GradleTool::with_gradle_path("/custom/gradle");
        assert_eq!(tool.name(), "gradle");
    }

    #[test]
    fn test_gradle_tool_unknown_operation_returns_error() {
        let tool = GradleTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_gradle_tool_empty_args_returns_error() {
        let tool = GradleTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }
}
