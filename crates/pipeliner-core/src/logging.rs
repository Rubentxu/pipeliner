//! Logging types for structured logging in pipelines.
//!
//! This module provides LogLevel enum and related utilities for
//! controlling and filtering log output.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Log level for pipeline execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    /// Debug level - detailed diagnostic information
    Debug,
    /// Info level - general information about execution
    Info,
    /// Warn level - warning about potential issues
    Warn,
    /// Error level - error conditions
    Error,
    /// Fatal level - critical errors that may abort execution
    Fatal,
}

impl LogLevel {
    /// Determines if a message at the given level should be logged
    /// based on the minimum log level configured.
    ///
    /// Returns `true` if the message should be logged, `false` if it should be skipped.
    ///
    /// # Examples
    ///
    /// ```
    /// use pipeliner_core::logging::LogLevel;
    ///
    /// // Debug message should not be logged when min level is Warn
    /// assert!(!LogLevel::should_log(LogLevel::Debug, LogLevel::Warn));
    ///
    /// // Error message should be logged when min level is Warn
    /// assert!(LogLevel::should_log(LogLevel::Error, LogLevel::Warn));
    /// ```
    #[must_use]
    pub fn should_log(message_level: LogLevel, min_level: LogLevel) -> bool {
        message_level >= min_level
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Fatal => write!(f, "FATAL"),
        }
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "fatal" | "critical" => Ok(LogLevel::Fatal),
            _ => Err(format!("Unknown log level: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // =========================================================================
    // Ordering Tests
    // =========================================================================

    #[test]
    fn test_log_level_ordering_debug_less_than_info() {
        assert!(LogLevel::Debug < LogLevel::Info);
    }

    #[test]
    fn test_log_level_ordering_info_less_than_warn() {
        assert!(LogLevel::Info < LogLevel::Warn);
    }

    #[test]
    fn test_log_level_ordering_warn_less_than_error() {
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_level_ordering_error_less_than_fatal() {
        assert!(LogLevel::Error < LogLevel::Fatal);
    }

    #[test]
    fn test_log_level_ordering_all_levels_ordered() {
        let levels = vec![
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Fatal,
        ];

        for i in 0..levels.len() {
            for j in (i + 1)..levels.len() {
                assert!(levels[i] < levels[j], "{:?} should be < {:?}", levels[i], levels[j]);
            }
        }
    }

    // =========================================================================
    // Default Level Test
    // =========================================================================

    #[test]
    fn test_log_level_default_is_info() {
        let default_level = LogLevel::default();
        assert_eq!(default_level, LogLevel::Info);
    }

    // =========================================================================
    // should_log Filtering Tests
    // =========================================================================

    #[test]
    fn test_should_log_debug_message_at_debug_level() {
        assert!(LogLevel::should_log(LogLevel::Debug, LogLevel::Debug));
    }

    #[test]
    fn test_should_log_debug_message_at_info_level() {
        // Debug < Info, so should NOT log
        assert!(!LogLevel::should_log(LogLevel::Debug, LogLevel::Info));
    }

    #[test]
    fn test_should_log_debug_message_at_warn_level() {
        // Debug < Warn, so should NOT log
        assert!(!LogLevel::should_log(LogLevel::Debug, LogLevel::Warn));
    }

    #[test]
    fn test_should_log_info_message_at_warn_level() {
        // Info < Warn, so should NOT log
        assert!(!LogLevel::should_log(LogLevel::Info, LogLevel::Warn));
    }

    #[test]
    fn test_should_log_error_message_at_warn_level() {
        // Error >= Warn, so SHOULD log
        assert!(LogLevel::should_log(LogLevel::Error, LogLevel::Warn));
    }

    #[test]
    fn test_should_log_fatal_message_at_any_level() {
        // Fatal >= any level, so SHOULD always log
        assert!(LogLevel::should_log(LogLevel::Fatal, LogLevel::Debug));
        assert!(LogLevel::should_log(LogLevel::Fatal, LogLevel::Info));
        assert!(LogLevel::should_log(LogLevel::Fatal, LogLevel::Warn));
        assert!(LogLevel::should_log(LogLevel::Fatal, LogLevel::Error));
        assert!(LogLevel::should_log(LogLevel::Fatal, LogLevel::Fatal));
    }

    #[test]
    fn test_should_log_warn_message_at_error_level() {
        // Warn < Error, so should NOT log
        assert!(!LogLevel::should_log(LogLevel::Warn, LogLevel::Error));
    }

    #[test]
    fn test_should_log_same_level() {
        // Same level should log
        assert!(LogLevel::should_log(LogLevel::Debug, LogLevel::Debug));
        assert!(LogLevel::should_log(LogLevel::Info, LogLevel::Info));
        assert!(LogLevel::should_log(LogLevel::Warn, LogLevel::Warn));
        assert!(LogLevel::should_log(LogLevel::Error, LogLevel::Error));
        assert!(LogLevel::should_log(LogLevel::Fatal, LogLevel::Fatal));
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn test_log_level_display_debug() {
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
    }

    #[test]
    fn test_log_level_display_info() {
        assert_eq!(LogLevel::Info.to_string(), "INFO");
    }

    #[test]
    fn test_log_level_display_warn() {
        assert_eq!(LogLevel::Warn.to_string(), "WARN");
    }

    #[test]
    fn test_log_level_display_error() {
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn test_log_level_display_fatal() {
        assert_eq!(LogLevel::Fatal.to_string(), "FATAL");
    }

    // =========================================================================
    // Serialization/Deserialization Tests
    // =========================================================================

    #[test]
    fn test_log_level_serialize_debug() {
        let json = serde_json::to_string(&LogLevel::Debug).unwrap();
        assert_eq!(json, "\"debug\"");
    }

    #[test]
    fn test_log_level_deserialize_debug() {
        let level: LogLevel = serde_json::from_str("\"debug\"").unwrap();
        assert_eq!(level, LogLevel::Debug);
    }

    #[test]
    fn test_log_level_serialize_info() {
        let json = serde_json::to_string(&LogLevel::Info).unwrap();
        assert_eq!(json, "\"info\"");
    }

    #[test]
    fn test_log_level_deserialize_info() {
        let level: LogLevel = serde_json::from_str("\"info\"").unwrap();
        assert_eq!(level, LogLevel::Info);
    }

    #[test]
    fn test_log_level_serialize_warn() {
        let json = serde_json::to_string(&LogLevel::Warn).unwrap();
        assert_eq!(json, "\"warn\"");
    }

    #[test]
    fn test_log_level_deserialize_warn() {
        let level: LogLevel = serde_json::from_str("\"warn\"").unwrap();
        assert_eq!(level, LogLevel::Warn);
    }

    #[test]
    fn test_log_level_serialize_error() {
        let json = serde_json::to_string(&LogLevel::Error).unwrap();
        assert_eq!(json, "\"error\"");
    }

    #[test]
    fn test_log_level_deserialize_error() {
        let level: LogLevel = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(level, LogLevel::Error);
    }

    #[test]
    fn test_log_level_serialize_fatal() {
        let json = serde_json::to_string(&LogLevel::Fatal).unwrap();
        assert_eq!(json, "\"fatal\"");
    }

    #[test]
    fn test_log_level_deserialize_fatal() {
        let level: LogLevel = serde_json::from_str("\"fatal\"").unwrap();
        assert_eq!(level, LogLevel::Fatal);
    }

    #[test]
    fn test_log_level_roundtrip_serialization() {
        let levels = vec![
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Fatal,
        ];

        for level in levels {
            let json = serde_json::to_string(&level).unwrap();
            let parsed: LogLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, parsed);
        }
    }

    // =========================================================================
    // FromStr Tests
    // =========================================================================

    #[test]
    fn test_log_level_from_str_debug() {
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("Debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
    }

    #[test]
    fn test_log_level_from_str_info() {
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("INFO".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("Info".parse::<LogLevel>().unwrap(), LogLevel::Info);
    }

    #[test]
    fn test_log_level_from_str_warn() {
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("WARN".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
    }

    #[test]
    fn test_log_level_from_str_error() {
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("ERROR".parse::<LogLevel>().unwrap(), LogLevel::Error);
    }

    #[test]
    fn test_log_level_from_str_fatal() {
        assert_eq!("fatal".parse::<LogLevel>().unwrap(), LogLevel::Fatal);
        assert_eq!("FATAL".parse::<LogLevel>().unwrap(), LogLevel::Fatal);
        assert_eq!("critical".parse::<LogLevel>().unwrap(), LogLevel::Fatal);
    }

    #[test]
    fn test_log_level_from_str_invalid() {
        assert!("unknown".parse::<LogLevel>().is_err());
        assert!("".parse::<LogLevel>().is_err());
        assert!("trace".parse::<LogLevel>().is_err());
    }

    // =========================================================================
    // Derived Trait Tests
    // =========================================================================

    #[test]
    fn test_log_level_clone() {
        let level = LogLevel::Info;
        let cloned = level;
        assert_eq!(level, cloned);
    }

    #[test]
    fn test_log_level_copy() {
        let level = LogLevel::Warn;
        let copied = level;
        assert_eq!(level, copied);
    }

    #[test]
    fn test_log_level_debug() {
        let level = LogLevel::Error;
        let debug_str = format!("{:?}", level);
        assert!(debug_str.contains("Error"));
    }

    #[test]
    fn test_log_level_in_hashmap() {
        let mut map = HashMap::new();
        map.insert(LogLevel::Debug, "debug value");
        map.insert(LogLevel::Info, "info value");

        assert_eq!(map.get(&LogLevel::Debug), Some(&"debug value"));
        assert_eq!(map.get(&LogLevel::Info), Some(&"info value"));
    }
}