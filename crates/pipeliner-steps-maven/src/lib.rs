//! # Pipeliner Steps - Maven
//!
//! Maven build tool steps for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod maven_tool;

pub use maven_tool::MavenTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // MavenTool Tests
    // =====================================================================

    #[test]
    fn test_maven_tool_name() {
        let tool = MavenTool::new();
        assert_eq!(tool.name(), "maven");
    }

    #[test]
    #[ignore = "requires mvn binary"]
    fn test_maven_tool_build_operation() {
        let tool = MavenTool::new();
        let args = vec![JsonValue::String("build".to_string())];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
        assert_eq!(step.name, "maven");
    }

    #[test]
    #[ignore = "requires mvn binary"]
    fn test_maven_tool_test_operation() {
        let tool = MavenTool::new();
        let args = vec![JsonValue::String("test".to_string())];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    #[ignore = "requires mvn binary"]
    fn test_maven_tool_publish_operation() {
        let tool = MavenTool::new();
        let args = vec![JsonValue::String("publish".to_string())];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    #[ignore = "requires mvn binary"]
    fn test_maven_tool_build_with_profiles_operation() {
        let tool = MavenTool::new();
        let args = vec![
            JsonValue::String("build_with_profiles".to_string()),
            JsonValue::String("profile1,profile2".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_maven_tool_build_with_profiles_empty_returns_error() {
        let tool = MavenTool::new();
        let args = vec![
            JsonValue::String("build_with_profiles".to_string()),
            JsonValue::String("".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_maven_tool_with_custom_path() {
        let tool = MavenTool::with_mvn_path("/custom/mvn");
        assert_eq!(tool.name(), "maven");
    }

    #[test]
    fn test_maven_tool_unknown_operation_returns_error() {
        let tool = MavenTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_maven_tool_empty_args_returns_error() {
        let tool = MavenTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }
}
