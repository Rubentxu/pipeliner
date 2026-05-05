//! # Pipeliner Steps - Container
//!
//! Container build and push steps for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod container_tool;

pub use container_tool::ContainerTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // ContainerTool Tests
    // =====================================================================

    #[test]
    fn test_container_tool_name() {
        let tool = ContainerTool::new();
        assert_eq!(tool.name(), "container");
    }

    #[test]
    #[ignore = "requires docker binary"]
    fn test_container_tool_build_operation() {
        let tool = ContainerTool::new();
        let args = vec![
            JsonValue::String("build".to_string()),
            JsonValue::String("my-image:latest".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
        assert_eq!(step.name, "container");
    }

    #[test]
    #[ignore = "requires docker binary"]
    fn test_container_tool_build_with_dockerfile() {
        let tool = ContainerTool::new();
        let args = vec![
            JsonValue::String("build".to_string()),
            JsonValue::String("my-image:latest".to_string()),
            JsonValue::String("Dockerfile.dev".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_container_tool_build_missing_image_returns_error() {
        let tool = ContainerTool::new();
        let args = vec![JsonValue::String("build".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    #[ignore = "requires docker binary"]
    fn test_container_tool_promote_operation() {
        let tool = ContainerTool::new();
        let args = vec![
            JsonValue::String("promote".to_string()),
            JsonValue::String("my-image:latest".to_string()),
            JsonValue::String("registry.example.com".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_container_tool_promote_missing_args_returns_error() {
        let tool = ContainerTool::new();
        let args = vec![JsonValue::String("promote".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_container_tool_with_custom_path() {
        let tool = ContainerTool::with_container_path("podman");
        assert_eq!(tool.name(), "container");
    }

    #[test]
    fn test_container_tool_unknown_operation_returns_error() {
        let tool = ContainerTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_container_tool_empty_args_returns_error() {
        let tool = ContainerTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }
}
