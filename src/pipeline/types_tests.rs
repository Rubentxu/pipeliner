//! Tests for pipeline types
//!
//! This module contains comprehensive tests for all pipeline domain types

#[cfg(test)]
mod types_tests {
    use super::super::*;
    use serde_json;

    #[test]
    fn test_stage_result_is_success() {
        assert!(StageResult::Success.is_success());
        assert!(!StageResult::Failure.is_success());
        assert!(!StageResult::Unstable.is_success());
        assert!(!StageResult::Skipped.is_success());
    }

    #[test]
    fn test_stage_result_is_failure() {
        assert!(!StageResult::Success.is_failure());
        assert!(StageResult::Failure.is_failure());
        assert!(!StageResult::Unstable.is_failure());
        assert!(!StageResult::Skipped.is_failure());
    }

    #[test]
    fn test_stage_result_is_unstable() {
        assert!(!StageResult::Success.is_unstable());
        assert!(!StageResult::Failure.is_unstable());
        assert!(StageResult::Unstable.is_unstable());
        assert!(!StageResult::Skipped.is_unstable());
    }

    #[test]
    fn test_stage_result_is_skipped() {
        assert!(!StageResult::Success.is_skipped());
        assert!(!StageResult::Failure.is_skipped());
        assert!(!StageResult::Unstable.is_skipped());
        assert!(StageResult::Skipped.is_skipped());
    }

    #[test]
    fn test_stage_result_display() {
        assert_eq!(StageResult::Success.to_string(), "SUCCESS");
        assert_eq!(StageResult::Failure.to_string(), "FAILURE");
        assert_eq!(StageResult::Unstable.to_string(), "UNSTABLE");
        assert_eq!(StageResult::Skipped.to_string(), "SKIPPED");
    }

    #[test]
    fn test_stage_result_serialize() {
        let result = StageResult::Success;
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(json, r#""success""#);
    }

    #[test]
    fn test_stage_result_deserialize() {
        let json = r#""failure""#;
        let result: StageResult = serde_json::from_str(json).unwrap();
        assert_eq!(result, StageResult::Failure);
    }

    #[test]
    fn test_validation_error_empty_name() {
        let err = ValidationError::EmptyName;
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_validation_error_name_too_long() {
        let err = ValidationError::NameTooLong { max: 100, len: 150 };
        assert!(err.to_string().contains("too long"));
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("150"));
    }

    #[test]
    fn test_validation_error_invalid_name_chars() {
        let err = ValidationError::InvalidNameChars { name: "My Stage".to_string() };
        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains("My Stage"));
    }

    #[test]
    fn test_validation_error_empty_pipeline() {
        let err = ValidationError::EmptyPipeline;
        assert!(err.to_string().contains("at least one stage"));
    }

    #[test]
    fn test_validation_error_empty_stage() {
        let err = ValidationError::EmptyStage { stage: "Build".to_string() };
        assert!(err.to_string().contains("Build"));
        assert!(err.to_string().contains("at least one step"));
    }

    #[test]
    fn test_pipeline_error_from_validation() {
        let validation_err = ValidationError::EmptyPipeline;
        let pipeline_err = PipelineError::Validation(validation_err);
        assert!(matches!(pipeline_err, PipelineError::Validation(_)));
    }

    #[test]
    fn test_pipeline_error_stage_failed() {
        let err = PipelineError::StageFailed {
            stage: "Build".to_string(),
            error: "Command failed".to_string(),
        };
        assert!(err.to_string().contains("Build"));
        assert!(err.to_string().contains("Command failed"));
    }

    #[test]
    fn test_pipeline_error_command_failed() {
        let err = PipelineError::CommandFailed {
            code: 1,
            stderr: "error".to_string(),
        };
        assert!(err.to_string().contains("1"));
        assert!(err.to_string().contains("error"));
    }

    #[test]
    fn test_pipeline_error_timeout() {
        let err = PipelineError::Timeout {
            duration: std::time::Duration::from_secs(60),
        };
        assert!(err.to_string().contains("timeout"));
        assert!(err.to_string().contains("60"));
    }

    #[test]
    fn test_pipeline_error_io() {
        let err = PipelineError::Io("file not found".to_string());
        assert!(err.to_string().contains("IO error"));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_pipeline_error_from_io_error() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        );
        let pipeline_err = PipelineError::from(io_err);
        assert!(matches!(pipeline_err, PipelineError::Io(_)));
    }
}
