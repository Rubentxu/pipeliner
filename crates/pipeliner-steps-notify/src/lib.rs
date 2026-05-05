//! # Pipeliner Steps - Notify
//!
//! Notification steps for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod notify_tool;

pub use notify_tool::NotifyTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // NotifyTool Tests
    // =====================================================================

    #[test]
    fn test_notify_tool_name() {
        let tool = NotifyTool::new();
        assert_eq!(tool.name(), "notify");
    }

    #[test]
    fn test_notify_tool_send_email_operation() {
        let tool = NotifyTool::new();
        let args = vec![
            JsonValue::String("sendEmail".to_string()),
            JsonValue::String("user@example.com".to_string()),
            JsonValue::String("Build Complete".to_string()),
            JsonValue::String("The build finished successfully.".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
        assert_eq!(step.name, "notify");
    }

    #[test]
    fn test_notify_tool_send_email_missing_to_returns_error() {
        let tool = NotifyTool::new();
        let args = vec![
            JsonValue::String("sendEmail".to_string()),
            JsonValue::String("Build Complete".to_string()),
            JsonValue::String("The build finished successfully.".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_notify_tool_send_email_missing_subject_returns_error() {
        let tool = NotifyTool::new();
        let args = vec![
            JsonValue::String("sendEmail".to_string()),
            JsonValue::String("user@example.com".to_string()),
            JsonValue::String("The build finished successfully.".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_notify_tool_send_email_missing_body_returns_error() {
        let tool = NotifyTool::new();
        let args = vec![
            JsonValue::String("sendEmail".to_string()),
            JsonValue::String("user@example.com".to_string()),
            JsonValue::String("Build Complete".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_notify_tool_with_webhook() {
        let tool = NotifyTool::with_webhook("https://hooks.example.com/notify");
        assert_eq!(tool.name(), "notify");
    }

    #[test]
    fn test_notify_tool_unknown_operation_returns_error() {
        let tool = NotifyTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_notify_tool_empty_args_returns_error() {
        let tool = NotifyTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_notify_tool_output_contains_email_details() {
        let tool = NotifyTool::new();
        let args = vec![
            JsonValue::String("sendEmail".to_string()),
            JsonValue::String("user@example.com".to_string()),
            JsonValue::String("Build Complete".to_string()),
            JsonValue::String("The build finished successfully.".to_string()),
        ];
        let step = tool.create(&args).unwrap();
        assert!(step.output.is_some());
        let output_json: serde_json::Value = serde_json::from_str(&step.output.unwrap()).unwrap();
        assert_eq!(output_json["operation"], "sendEmail");
        assert_eq!(output_json["to"], "user@example.com");
        assert_eq!(output_json["subject"], "Build Complete");
        assert_eq!(output_json["body"], "The build finished successfully.");
    }
}
