//! Stage marker emission and parsing for pipeline stage tracking.
//!
//! This module provides utilities for emitting stage markers to a `Write` destination
//! and parsing stage markers from lines of output.

use crate::types::markers::{StageMarker, StageResult};
use std::io::{Result as IoResult, Write};

/// Prefix used to identify stage marker lines in output.
pub const STAGE_MARKER_PREFIX: &str = "__STAGE__";

/// Emitter for writing stage markers to a writer.
pub struct StageMarkerEmitter;

impl StageMarkerEmitter {
    /// Emit a Started marker.
    pub fn started(writer: &mut impl Write, name: &str) -> IoResult<()> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let marker = StageMarker::Started {
            name: name.to_string(),
            ts,
        };
        let json = serde_json::to_string(&marker).unwrap();
        writeln!(writer, "{}{}", STAGE_MARKER_PREFIX, json)
    }

    /// Emit a Completed marker.
    pub fn completed(
        writer: &mut impl Write,
        name: &str,
        duration_ms: u64,
        result: StageResult,
    ) -> IoResult<()> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let marker = StageMarker::Completed {
            name: name.to_string(),
            ts,
            duration_ms,
            result,
        };
        let json = serde_json::to_string(&marker).unwrap();
        writeln!(writer, "{}{}", STAGE_MARKER_PREFIX, json)
    }

    /// Emit an Error marker.
    pub fn error(writer: &mut impl Write, name: &str, message: &str) -> IoResult<()> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let marker = StageMarker::Error {
            name: name.to_string(),
            ts,
            message: message.to_string(),
        };
        let json = serde_json::to_string(&marker).unwrap();
        writeln!(writer, "{}{}", STAGE_MARKER_PREFIX, json)
    }

    /// Emit a Skipped marker.
    pub fn skipped(writer: &mut impl Write, name: &str) -> IoResult<()> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let marker = StageMarker::Skipped {
            name: name.to_string(),
            ts,
        };
        let json = serde_json::to_string(&marker).unwrap();
        writeln!(writer, "{}{}", STAGE_MARKER_PREFIX, json)
    }
}

/// Parser for extracting stage markers from lines of output.
pub struct StageMarkerParser;

impl StageMarkerParser {
    /// Parse a line of output and extract a StageMarker if present.
    ///
    /// Returns `Some(StageMarker)` if the line contains a valid stage marker,
    /// or `None` if the line is not a stage marker or contains invalid JSON.
    pub fn parse_line(line: &str) -> Option<StageMarker> {
        let prefix = STAGE_MARKER_PREFIX;
        if !line.starts_with(prefix) {
            return None;
        }
        let json = &line[prefix.len()..];
        serde_json::from_str(json).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_started_and_parse() {
        let mut buffer = Vec::new();
        StageMarkerEmitter::started(&mut buffer, "build").unwrap();
        let output = String::from_utf8(buffer).unwrap();

        // Verify prefix
        assert!(output.starts_with("__STAGE__"));

        // Extract JSON part and parse
        let json_part = output.strip_prefix("__STAGE__").unwrap().trim();
        let marker: StageMarker = serde_json::from_str(json_part).unwrap();

        match marker {
            StageMarker::Started { name, ts } => {
                assert_eq!(name, "build");
                assert!(ts > 0);
            }
            other => panic!("Expected Started, got {:?}", other),
        }
    }

    #[test]
    fn test_emit_completed_and_parse() {
        let mut buffer = Vec::new();
        StageMarkerEmitter::completed(&mut buffer, "test", 1500, StageResult::Success).unwrap();
        let output = String::from_utf8(buffer).unwrap();

        // Verify prefix and format
        assert!(output.starts_with("__STAGE__"));

        // Extract JSON part and parse
        let json_part = output.strip_prefix("__STAGE__").unwrap().trim();
        let marker: StageMarker = serde_json::from_str(json_part).unwrap();

        match marker {
            StageMarker::Completed {
                name,
                ts,
                duration_ms,
                result,
            } => {
                assert_eq!(name, "test");
                assert!(ts > 0);
                assert_eq!(duration_ms, 1500);
                assert_eq!(result, StageResult::Success);
            }
            other => panic!("Expected Completed, got {:?}", other),
        }
    }

    #[test]
    fn test_emit_error_and_parse() {
        let mut buffer = Vec::new();
        StageMarkerEmitter::error(&mut buffer, "deploy", "connection refused").unwrap();
        let output = String::from_utf8(buffer).unwrap();

        // Verify prefix
        assert!(output.starts_with("__STAGE__"));

        // Extract JSON part and parse
        let json_part = output.strip_prefix("__STAGE__").unwrap().trim();
        let marker: StageMarker = serde_json::from_str(json_part).unwrap();

        match marker {
            StageMarker::Error {
                name,
                ts,
                message,
            } => {
                assert_eq!(name, "deploy");
                assert!(ts > 0);
                assert_eq!(message, "connection refused");
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_emit_skipped_and_parse() {
        let mut buffer = Vec::new();
        StageMarkerEmitter::skipped(&mut buffer, "lint").unwrap();
        let output = String::from_utf8(buffer).unwrap();

        // Verify prefix
        assert!(output.starts_with("__STAGE__"));

        // Extract JSON part and parse
        let json_part = output.strip_prefix("__STAGE__").unwrap().trim();
        let marker: StageMarker = serde_json::from_str(json_part).unwrap();

        match marker {
            StageMarker::Skipped { name, ts } => {
                assert_eq!(name, "lint");
                assert!(ts > 0);
            }
            other => panic!("Expected Skipped, got {:?}", other),
        }
    }

    #[test]
    fn test_roundtrip_started() {
        let mut buffer = Vec::new();
        let ts = 1000u64;

        // Manually create a Started marker
        let original = StageMarker::Started {
            name: "build".to_string(),
            ts,
        };

        // Emit it
        let json = serde_json::to_string(&original).unwrap();
        writeln!(buffer, "{}{}", STAGE_MARKER_PREFIX, json).unwrap();

        // Parse the line
        let output = String::from_utf8(buffer).unwrap();
        let parsed = StageMarkerParser::parse_line(&output).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_roundtrip_completed() {
        let mut buffer = Vec::new();
        let ts = 2000u64;

        // Manually create a Completed marker
        let original = StageMarker::Completed {
            name: "test".to_string(),
            ts,
            duration_ms: 1500,
            result: StageResult::Failure,
        };

        // Emit it
        let json = serde_json::to_string(&original).unwrap();
        writeln!(buffer, "{}{}", STAGE_MARKER_PREFIX, json).unwrap();

        // Parse the line
        let output = String::from_utf8(buffer).unwrap();
        let parsed = StageMarkerParser::parse_line(&output).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_roundtrip_error() {
        let mut buffer = Vec::new();
        let ts = 3000u64;

        // Manually create an Error marker
        let original = StageMarker::Error {
            name: "deploy".to_string(),
            ts,
            message: "conn refused".to_string(),
        };

        // Emit it
        let json = serde_json::to_string(&original).unwrap();
        writeln!(buffer, "{}{}", STAGE_MARKER_PREFIX, json).unwrap();

        // Parse the line
        let output = String::from_utf8(buffer).unwrap();
        let parsed = StageMarkerParser::parse_line(&output).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_roundtrip_skipped() {
        let mut buffer = Vec::new();
        let ts = 4000u64;

        // Manually create a Skipped marker
        let original = StageMarker::Skipped {
            name: "lint".to_string(),
            ts,
        };

        // Emit it
        let json = serde_json::to_string(&original).unwrap();
        writeln!(buffer, "{}{}", STAGE_MARKER_PREFIX, json).unwrap();

        // Parse the line
        let output = String::from_utf8(buffer).unwrap();
        let parsed = StageMarkerParser::parse_line(&output).unwrap();

        assert_eq!(parsed, original);
    }

    // Task A7: Parser tests

    #[test]
    fn test_parse_valid_started_marker() {
        let line = r#"__STAGE__{"type":"STARTED","name":"build","ts":1000}"#;
        let result = StageMarkerParser::parse_line(line);
        assert!(result.is_some());
        match result.unwrap() {
            StageMarker::Started { name, ts } => {
                assert_eq!(name, "build");
                assert_eq!(ts, 1000);
            }
            other => panic!("Expected Started, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_valid_completed_marker() {
        let line = r#"__STAGE__{"type":"COMPLETED","name":"test","ts":2000,"duration_ms":1500,"result":"SUCCESS"}"#;
        let result = StageMarkerParser::parse_line(line);
        assert!(result.is_some());
        match result.unwrap() {
            StageMarker::Completed {
                name,
                ts,
                duration_ms,
                result,
            } => {
                assert_eq!(name, "test");
                assert_eq!(ts, 2000);
                assert_eq!(duration_ms, 1500);
                assert_eq!(result, StageResult::Success);
            }
            other => panic!("Expected Completed, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_normal_stdout_line() {
        let line = "Building project with cargo...";
        let result = StageMarkerParser::parse_line(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_marker_in_middle_of_output() {
        // Line with marker embedded in the middle of regular output
        let line = r#"Compiling... __STAGE__{"type":"STARTED","name":"build","ts":1000} ...done"#;
        let result = StageMarkerParser::parse_line(line);
        // Since it starts with "Compiling..." not "__STAGE__", it should return None
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_json_after_prefix() {
        let line = r#"__STAGE__not valid json"#;
        let result = StageMarkerParser::parse_line(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_line_with_only_prefix() {
        let line = "__STAGE__";
        let result = StageMarkerParser::parse_line(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_empty_json_after_prefix() {
        let line = r#"__STAGE__{}"#;
        let result = StageMarkerParser::parse_line(line);
        // Empty JSON is valid but won't deserialize to StageMarker
        assert!(result.is_none());
    }
}
