//! # Pipeliner Steps - Helm
//!
//! Helm chart operations steps for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod helm_tool;

pub use helm_tool::HelmTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // HelmTool Tests
    // =====================================================================

    #[test]
    fn test_helm_tool_name() {
        let tool = HelmTool::new();
        assert_eq!(tool.name(), "helm");
    }

    #[test]
    #[ignore = "requires helm binary"]
    fn test_helm_tool_build_operation() {
        let tool = HelmTool::new();
        let args = vec![
            JsonValue::String("build".to_string()),
            JsonValue::String("./charts/myapp".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
        assert_eq!(step.name, "helm");
    }

    #[test]
    fn test_helm_tool_build_missing_chart_returns_error() {
        let tool = HelmTool::new();
        let args = vec![JsonValue::String("build".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    #[ignore = "requires helm binary"]
    fn test_helm_tool_test_operation() {
        let tool = HelmTool::new();
        let args = vec![
            JsonValue::String("test".to_string()),
            JsonValue::String("./charts/myapp".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    #[ignore = "requires helm binary"]
    fn test_helm_tool_deploy_operation() {
        let tool = HelmTool::new();
        let args = vec![
            JsonValue::String("deploy".to_string()),
            JsonValue::String("./charts/myapp".to_string()),
            JsonValue::String("my-namespace".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_helm_tool_deploy_missing_namespace_returns_error() {
        let tool = HelmTool::new();
        let args = vec![
            JsonValue::String("deploy".to_string()),
            JsonValue::String("./charts/myapp".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "requires helm binary"]
    fn test_helm_tool_promote_operation() {
        let tool = HelmTool::new();
        let args = vec![
            JsonValue::String("promote".to_string()),
            JsonValue::String("./charts/myapp".to_string()),
            JsonValue::String("oci://registry.example.com".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_helm_tool_promote_missing_registry_returns_error() {
        let tool = HelmTool::new();
        let args = vec![
            JsonValue::String("promote".to_string()),
            JsonValue::String("./charts/myapp".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_helm_tool_with_custom_path() {
        let tool = HelmTool::with_helm_path("/usr/local/bin/helm");
        assert_eq!(tool.name(), "helm");
    }

    #[test]
    fn test_helm_tool_unknown_operation_returns_error() {
        let tool = HelmTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_helm_tool_empty_args_returns_error() {
        let tool = HelmTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }
}
