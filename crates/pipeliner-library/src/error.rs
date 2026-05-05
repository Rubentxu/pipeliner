//! Library error types for the pipeliner library system.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when loading or processing libraries.
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[serde(rename_all = "camelCase")]
pub enum LibraryError {
    /// Source directory or repository not found
    #[error("Source not found: {0}")]
    SourceNotFound(String),

    /// Git clone operation failed
    #[error("Git clone failed for {url}. {reason}. Ensure git is installed and the URL is accessible")]
    GitCloneFailed {
        /// The URL that failed
        url: String,
        /// The reason for failure
        reason: String,
    },

    /// Invalid library configuration
    #[error("Invalid library configuration: {0}")]
    InvalidConfig(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(
        /// The I/O error message
        String,
    ),
}

impl LibraryError {
    /// Creates an I/O error from a std::io::Error
    pub fn from_io_error(err: std::io::Error) -> Self {
        LibraryError::IoError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // ===================================================================
    // A2: LibraryError Display and Variant Tests (RED → GREEN)
    // ===================================================================

    #[test]
    fn test_source_not_found_display() {
        // SCN-LS-004: LocalSource with nonexistent path returns SourceNotFound
        let err = LibraryError::SourceNotFound("/nonexistent/path".to_string());
        let display = format!("{}", err);
        assert!(display.contains("/nonexistent/path"), "Display should contain the path");
        assert!(display.contains("Source not found"), "Display should contain 'Source not found'");
    }

    #[test]
    fn test_git_clone_failed_display() {
        // SCN-LS-002: GitSource with invalid URL returns GitCloneFailed
        let err = LibraryError::GitCloneFailed {
            url: "https://github.com/invalid/repo".to_string(),
            reason: "remote: Repository not found".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("https://github.com/invalid/repo"));
        assert!(display.contains("remote: Repository not found"));
        assert!(display.contains("Git clone failed"));
    }

    #[test]
    fn test_invalid_config_display() {
        let err = LibraryError::InvalidConfig("Missing required field 'name'".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Missing required field 'name'"));
        assert!(display.contains("Invalid library configuration"));
    }

    #[test]
    fn test_io_error_display() {
        let err = LibraryError::IoError("No such file or directory".to_string());
        let display = format!("{}", err);
        assert!(display.contains("No such file or directory"));
        assert!(display.contains("I/O error"));
    }

    // ===================================================================
    // A3: LibraryError Source Propagation and Clone Tests (TRIANGULATE)
    // ===================================================================

    #[test]
    fn test_library_error_clone() {
        // A3: LibraryError is Clone
        let err = LibraryError::SourceNotFound("test".to_string());
        let cloned = err.clone();
        assert_eq!(format!("{}", err), format!("{}", cloned));
    }

    #[test]
    fn test_library_error_debug() {
        // A3: LibraryError implements Debug
        let err = LibraryError::SourceNotFound("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("SourceNotFound"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_git_clone_failed_from_error() {
        // Test that GitCloneFailed can be created from components
        let url = "https://github.com/example/repo".to_string();
        let reason = "connection timeout".to_string();
        let err = LibraryError::GitCloneFailed {
            url: url.clone(),
            reason: reason.clone(),
        };
        assert!(matches!(err, LibraryError::GitCloneFailed { url: u, reason: r } if u == url && r == reason));
    }

    #[test]
    fn test_io_error_from_std_io_error() {
        // A3: IoError can be created from std::io::Error
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
        let err = LibraryError::from_io_error(io_err);
        
        let display = format!("{}", err);
        assert!(display.contains("Permission denied"));
        assert!(display.contains("I/O error"));
        assert!(matches!(err, LibraryError::IoError(msg) if msg.contains("Permission denied")));
    }

    #[test]
    fn test_library_error_serialize() {
        // Test that LibraryError can be serialized to JSON
        let err = LibraryError::SourceNotFound("/path/to/lib".to_string());
        let json = serde_json::to_string(&err).expect("Should serialize");
        assert!(json.contains("sourceNotFound"));
        assert!(json.contains("/path/to/lib"));
    }

    #[test]
    fn test_library_error_deserialize() {
        // Test that LibraryError can be deserialized from JSON
        let json = r#"{"sourceNotFound": "/path/to/lib"}"#;
        let err: LibraryError = serde_json::from_str(json).expect("Should deserialize");
        assert!(matches!(err, LibraryError::SourceNotFound(p) if p == "/path/to/lib"));
    }

    #[test]
    fn test_git_clone_failed_serde() {
        let err = LibraryError::GitCloneFailed {
            url: "https://github.com/example/repo".to_string(),
            reason: "not found".to_string(),
        };
        let json = serde_json::to_string(&err).expect("Should serialize");
        assert!(json.contains("gitCloneFailed"));
        assert!(json.contains("https://github.com/example/repo"));
        
        let parsed: LibraryError = serde_json::from_str(&json).expect("Should deserialize");
        assert!(matches!(parsed, LibraryError::GitCloneFailed { url, reason } 
            if url == "https://github.com/example/repo" && reason == "not found"));
    }
}
