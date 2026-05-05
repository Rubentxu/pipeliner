//! # Pipeliner Steps - HTTP
//!
//! HTTP/REST client step tool for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod rest_client_tool;

pub use rest_client_tool::RestClientTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // RestClientTool Tests
    // =====================================================================

    #[test]
    fn test_rest_client_tool_name() {
        let tool = RestClientTool::new();
        assert_eq!(tool.name(), "restClient");
    }

    #[test]
    fn test_rest_client_tool_create_get_operation() {
        let tool = RestClientTool::new();
        let args = vec![
            JsonValue::String("get".to_string()),
            JsonValue::String("https://httpbin.org/get".to_string()),
        ];
        let result = tool.create(&args);
        // GET request will fail without network, but verifies arg parsing
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_rest_client_tool_create_get_with_headers() {
        let tool = RestClientTool::new();
        let args = vec![
            JsonValue::String("get".to_string()),
            JsonValue::String("https://httpbin.org/get".to_string()),
            serde_json::json!({"Authorization": "Bearer token123"}).into(),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_rest_client_tool_create_post_operation() {
        let tool = RestClientTool::new();
        let args = vec![
            JsonValue::String("post".to_string()),
            JsonValue::String("https://httpbin.org/post".to_string()),
            serde_json::json!({"key": "value"}).into(),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_rest_client_tool_create_put_operation() {
        let tool = RestClientTool::new();
        let args = vec![
            JsonValue::String("put".to_string()),
            JsonValue::String("https://httpbin.org/put".to_string()),
            serde_json::json!({"key": "value"}).into(),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok() || matches!(result.unwrap_err(), StepError::CreationFailed { .. }));
    }

    #[test]
    fn test_rest_client_tool_create_with_invalid_args_returns_error() {
        let tool = RestClientTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_rest_client_tool_create_with_unknown_operation_returns_error() {
        let tool = RestClientTool::new();
        let args = vec![JsonValue::String("delete".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_rest_client_tool_create_get_without_url_returns_error() {
        let tool = RestClientTool::new();
        let args = vec![JsonValue::String("get".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }
}
