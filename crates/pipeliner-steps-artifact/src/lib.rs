//! # Pipeliner Steps - Artifact
//!
//! Artifact repository steps for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod artifact_tool;

pub use artifact_tool::ArtifactTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // ArtifactTool Tests
    // =====================================================================

    #[test]
    fn test_artifact_tool_name() {
        let tool = ArtifactTool::new();
        assert_eq!(tool.name(), "artifact");
    }

    #[test]
    fn test_artifact_tool_upload_operation() {
        let tool = ArtifactTool::new();
        let args = vec![
            JsonValue::String("upload".to_string()),
            JsonValue::String("target/myapp.jar".to_string()),
            JsonValue::String("https://repo.example.com".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
        assert_eq!(step.name, "artifact");
    }

    #[test]
    fn test_artifact_tool_upload_missing_args_returns_error() {
        let tool = ArtifactTool::new();
        let args = vec![
            JsonValue::String("upload".to_string()),
            JsonValue::String("target/myapp.jar".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_artifact_tool_download_operation() {
        let tool = ArtifactTool::new();
        let args = vec![
            JsonValue::String("download".to_string()),
            JsonValue::String("com.example:myapp".to_string()),
            JsonValue::String("1.0.0".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_artifact_tool_download_missing_args_returns_error() {
        let tool = ArtifactTool::new();
        let args = vec![
            JsonValue::String("download".to_string()),
            JsonValue::String("com.example:myapp".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_artifact_tool_search_operation() {
        let tool = ArtifactTool::new();
        let args = vec![
            JsonValue::String("search".to_string()),
            JsonValue::String("myapp".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_artifact_tool_search_missing_query_returns_error() {
        let tool = ArtifactTool::new();
        let args = vec![JsonValue::String("search".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_artifact_tool_unknown_operation_returns_error() {
        let tool = ArtifactTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_artifact_tool_empty_args_returns_error() {
        let tool = ArtifactTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_artifact_tool_output_contains_operation_details() {
        let tool = ArtifactTool::new();
        let args = vec![
            JsonValue::String("upload".to_string()),
            JsonValue::String("myapp.jar".to_string()),
            JsonValue::String("https://repo.example.com".to_string()),
        ];
        let step = tool.create(&args).unwrap();
        assert!(step.output.is_some());
        let output_json: serde_json::Value = serde_json::from_str(&step.output.unwrap()).unwrap();
        assert_eq!(output_json["operation"], "upload");
        assert_eq!(output_json["file"], "myapp.jar");
        assert_eq!(output_json["repo"], "https://repo.example.com");
    }
}
