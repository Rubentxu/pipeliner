//! StageMarker and StageResult types for pipeline stage tracking.
//!
//! These types represent the lifecycle of pipeline stages and are emitted
//! as structured markers to stdout for consumption by external tools.

use serde::{Deserialize, Serialize};

/// Stage result variants indicating how a stage completed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StageResult {
    /// Stage completed successfully
    Success,
    /// Stage completed with failures
    Failure,
    /// Stage completed but with unstable results
    Unstable,
    /// Stage was aborted
    Aborted,
}

/// Stage lifecycle markers emitted during pipeline execution.
///
/// Each marker contains the stage name, timestamp, and type-specific fields.
/// Markers are serialized to JSON with a `type` field for discriminant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StageMarker {
    /// Stage has started
    Started {
        /// Name of the stage
        name: String,
        /// Unix timestamp when the stage started
        ts: u64,
    },
    /// Stage has completed
    Completed {
        /// Name of the stage
        name: String,
        /// Unix timestamp when the stage completed
        ts: u64,
        /// Duration of the stage in milliseconds
        duration_ms: u64,
        /// Result of the stage
        result: StageResult,
    },
    /// Stage encountered an error
    Error {
        /// Name of the stage
        name: String,
        /// Unix timestamp when the error occurred
        ts: u64,
        /// Error message
        message: String,
    },
    /// Stage was skipped
    Skipped {
        /// Name of the stage
        name: String,
        /// Unix timestamp when the stage was skipped
        ts: u64,
    },
}

/// Prefix used to identify stage marker lines in output.
pub const STAGE_MARKER_PREFIX: &str = "__STAGE__";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_result_serialization() {
        // Test all StageResult variants serialize to SCREAMING_SNAKE_CASE
        assert_eq!(serde_json::to_string(&StageResult::Success).unwrap(), "\"SUCCESS\"");
        assert_eq!(serde_json::to_string(&StageResult::Failure).unwrap(), "\"FAILURE\"");
        assert_eq!(serde_json::to_string(&StageResult::Unstable).unwrap(), "\"UNSTABLE\"");
        assert_eq!(serde_json::to_string(&StageResult::Aborted).unwrap(), "\"ABORTED\"");
    }

    #[test]
    fn test_stage_result_deserialization() {
        // Test all StageResult variants deserialize from SCREAMING_SNAKE_CASE
        assert_eq!(serde_json::from_str::<StageResult>("\"SUCCESS\"").unwrap(), StageResult::Success);
        assert_eq!(serde_json::from_str::<StageResult>("\"FAILURE\"").unwrap(), StageResult::Failure);
        assert_eq!(serde_json::from_str::<StageResult>("\"UNSTABLE\"").unwrap(), StageResult::Unstable);
        assert_eq!(serde_json::from_str::<StageResult>("\"ABORTED\"").unwrap(), StageResult::Aborted);
    }

    #[test]
    fn test_started_serialization() {
        let marker = StageMarker::Started {
            name: "build".to_string(),
            ts: 1000,
        };
        let json = serde_json::to_string(&marker).unwrap();
        assert!(json.contains("\"type\":\"STARTED\""));
        assert!(json.contains("\"name\":\"build\""));
        assert!(json.contains("\"ts\":1000"));
    }

    #[test]
    fn test_started_deserialization() {
        let json = r#"{"type":"STARTED","name":"build","ts":1000}"#;
        let marker = serde_json::from_str::<StageMarker>(json).unwrap();
        match marker {
            StageMarker::Started { name, ts } => {
                assert_eq!(name, "build");
                assert_eq!(ts, 1000);
            }
            other => panic!("Expected Started, got {:?}", other),
        }
    }

    #[test]
    fn test_completed_serialization() {
        let marker = StageMarker::Completed {
            name: "test".to_string(),
            ts: 2000,
            duration_ms: 1500,
            result: StageResult::Success,
        };
        let json = serde_json::to_string(&marker).unwrap();
        assert!(json.contains("\"type\":\"COMPLETED\""));
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"ts\":2000"));
        assert!(json.contains("\"duration_ms\":1500"));
        assert!(json.contains("\"result\":\"SUCCESS\""));
    }

    #[test]
    fn test_completed_deserialization() {
        let json = r#"{"type":"COMPLETED","name":"test","ts":2000,"duration_ms":1500,"result":"SUCCESS"}"#;
        let marker = serde_json::from_str::<StageMarker>(json).unwrap();
        match marker {
            StageMarker::Completed { name, ts, duration_ms, result } => {
                assert_eq!(name, "test");
                assert_eq!(ts, 2000);
                assert_eq!(duration_ms, 1500);
                assert_eq!(result, StageResult::Success);
            }
            other => panic!("Expected Completed, got {:?}", other),
        }
    }

    #[test]
    fn test_error_serialization() {
        let marker = StageMarker::Error {
            name: "deploy".to_string(),
            ts: 3000,
            message: "connection refused".to_string(),
        };
        let json = serde_json::to_string(&marker).unwrap();
        assert!(json.contains("\"type\":\"ERROR\""));
        assert!(json.contains("\"name\":\"deploy\""));
        assert!(json.contains("\"ts\":3000"));
        assert!(json.contains("\"message\":\"connection refused\""));
    }

    #[test]
    fn test_error_deserialization() {
        let json = r#"{"type":"ERROR","name":"deploy","ts":3000,"message":"connection refused"}"#;
        let marker = serde_json::from_str::<StageMarker>(json).unwrap();
        match marker {
            StageMarker::Error { name, ts, message } => {
                assert_eq!(name, "deploy");
                assert_eq!(ts, 3000);
                assert_eq!(message, "connection refused");
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_skipped_serialization() {
        let marker = StageMarker::Skipped {
            name: "lint".to_string(),
            ts: 4000,
        };
        let json = serde_json::to_string(&marker).unwrap();
        assert!(json.contains("\"type\":\"SKIPPED\""));
        assert!(json.contains("\"name\":\"lint\""));
        assert!(json.contains("\"ts\":4000"));
    }

    #[test]
    fn test_skipped_deserialization() {
        let json = r#"{"type":"SKIPPED","name":"lint","ts":4000}"#;
        let marker = serde_json::from_str::<StageMarker>(json).unwrap();
        match marker {
            StageMarker::Skipped { name, ts } => {
                assert_eq!(name, "lint");
                assert_eq!(ts, 4000);
            }
            other => panic!("Expected Skipped, got {:?}", other),
        }
    }

    #[test]
    fn test_stage_marker_roundtrip_started() {
        let original = StageMarker::Started {
            name: "build".to_string(),
            ts: 1000,
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: StageMarker = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_stage_marker_roundtrip_completed() {
        let original = StageMarker::Completed {
            name: "test".to_string(),
            ts: 2000,
            duration_ms: 1500,
            result: StageResult::Failure,
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: StageMarker = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_stage_marker_roundtrip_error() {
        let original = StageMarker::Error {
            name: "deploy".to_string(),
            ts: 3000,
            message: "conn refused".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: StageMarker = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_stage_marker_roundtrip_skipped() {
        let original = StageMarker::Skipped {
            name: "lint".to_string(),
            ts: 4000,
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: StageMarker = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_stage_marker_prefix_constant() {
        assert_eq!(STAGE_MARKER_PREFIX, "__STAGE__");
    }

    #[test]
    fn test_all_variants_have_required_derive() {
        use std::fmt::Debug;
        // Compile-time verification that StageMarker has required derives
        fn has_debug_clone<T: Debug + Clone>() {}
        fn has_serde<T: Serialize + for<'de> Deserialize<'de>>() {}

        has_debug_clone::<StageMarker>();
        has_serde::<StageMarker>();
        has_debug_clone::<StageResult>();
        has_serde::<StageResult>();
    }
}
