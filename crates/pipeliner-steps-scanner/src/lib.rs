//! # Pipeliner Steps - Scanner
//!
//! Security scanner steps for Pipeliner.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

mod scanner_tool;

pub use scanner_tool::ScannerTool;

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::registry::{StepError, StepFactory};
    use serde_json::Value as JsonValue;

    // =====================================================================
    // ScannerTool Tests
    // =====================================================================

    #[test]
    fn test_scanner_tool_name() {
        let tool = ScannerTool::new();
        assert_eq!(tool.name(), "scanner");
    }

    #[test]
    #[ignore = "requires scanner binary"]
    fn test_scanner_tool_execute_operation() {
        let tool = ScannerTool::new();
        let args = vec![
            JsonValue::String("execute".to_string()),
            JsonValue::String("trivy".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
        assert_eq!(step.name, "scanner");
    }

    #[test]
    #[ignore = "requires scanner binary"]
    fn test_scanner_tool_execute_with_config() {
        let tool = ScannerTool::new();
        let args = vec![
            JsonValue::String("execute".to_string()),
            JsonValue::String("trivy".to_string()),
            JsonValue::String("./config/trivy.yaml".to_string()),
        ];
        let result = tool.create(&args);
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(step.success);
    }

    #[test]
    fn test_scanner_tool_execute_missing_tool_returns_error() {
        let tool = ScannerTool::new();
        let args = vec![JsonValue::String("execute".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_scanner_tool_with_custom_path() {
        let tool = ScannerTool::with_scanner_path("/custom/trivy");
        assert_eq!(tool.name(), "scanner");
    }

    #[test]
    fn test_scanner_tool_unknown_operation_returns_error() {
        let tool = ScannerTool::new();
        let args = vec![JsonValue::String("unknown".to_string())];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }

    #[test]
    fn test_scanner_tool_empty_args_returns_error() {
        let tool = ScannerTool::new();
        let args: Vec<JsonValue> = vec![];
        let result = tool.create(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StepError::InvalidArgs { .. }));
    }
}
