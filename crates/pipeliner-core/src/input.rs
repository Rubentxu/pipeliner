//! Pipeline input detection and parsing.
//!
//! This module provides the `PipelineInput` enum and related types for detecting
//! and parsing pipeline input from various file formats (.rs, .json, .toml) and
//! sources (files, URL, STDIN, inline expressions).
//!
//! ## Example
//!
//! ```rust
//! use pipeliner_core::input::PipelineInput;
//! use std::path::Path;
//!
//! // Detect input type from file extension
//! let input = PipelineInput::detect(Path::new("pipeline.json")).unwrap();
//! assert!(matches!(input, PipelineInput::JsonFile(_)));
//!
//! let input = PipelineInput::detect(Path::new("pipeline.rs")).unwrap();
//! assert!(matches!(input, PipelineInput::RustScript(_)));
//!
//! let input = PipelineInput::detect(Path::new("pipeline.toml")).unwrap();
//! assert!(matches!(input, PipelineInput::TomlFile(_)));
//! ```

use std::path::PathBuf;

use crate::config::PipelineConfig;
use crate::pipeline::Pipeline;
use crate::runtime::RuntimeError;
use crate::validation::Validate;

/// Error type for pipeline input operations.
#[derive(Debug)]
pub enum InputError {
    /// The file format is not recognized
    UnknownFormat(PathBuf),
    /// The file was not found
    FileNotFound(PathBuf),
    /// I/O error reading the file
    Io(std::io::Error),
    /// TOML parsing error
    Toml(String),
    /// Validation error
    Validation(String),
    /// Unsupported input source
    UnsupportedSource(String),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::UnknownFormat(path) => {
                write!(f, "Unknown pipeline file format: {}", path.display())
            }
            InputError::FileNotFound(path) => {
                write!(f, "Pipeline file not found: {}", path.display())
            }
            InputError::Io(err) => write!(f, "I/O error: {err}"),
            InputError::Toml(msg) => write!(f, "TOML parsing error: {msg}"),
            InputError::Validation(msg) => write!(f, "Validation error: {msg}"),
            InputError::UnsupportedSource(msg) => {
                write!(f, "Unsupported input source: {msg}")
            }
        }
    }
}

impl std::error::Error for InputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InputError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for InputError {
    fn from(err: std::io::Error) -> Self {
        InputError::Io(err)
    }
}

impl From<crate::config::ConfigError> for InputError {
    fn from(err: crate::config::ConfigError) -> Self {
        match err {
            crate::config::ConfigError::Json(e) => InputError::Validation(e.to_string()),
            crate::config::ConfigError::Validation(msg) => InputError::Validation(msg),
        }
    }
}

impl From<RuntimeError> for InputError {
    fn from(err: RuntimeError) -> Self {
        match err {
            RuntimeError::ConfigError(msg) => InputError::Validation(msg),
            RuntimeError::PhaseFailed { source, .. } => InputError::Validation(source),
        }
    }
}

/// Represents a pipeline input source.
///
/// `PipelineInput` detects the input type from a file path and can parse
/// the input into a `Pipeline` (for JSON/TOML) or forward it for
/// Rust script execution.
#[derive(Debug, Clone)]
pub enum PipelineInput {
    /// A `.rs` file with shebang or `pipeline!` macro
    RustScript(PathBuf),
    /// A JSON configuration file (`.json`)
    JsonFile(PathBuf),
    /// A TOML configuration file (`.toml`)
    TomlFile(PathBuf),
    /// An inline expression string
    Expr(String),
    /// A URL to download
    Url(String),
    /// Pipeline from STDIN
    Stdin,
}

impl PipelineInput {
    /// Detects the input type from a file path based on its extension.
    ///
    /// # Errors
    ///
    /// Returns `InputError::UnknownFormat` if the file extension is not
    /// recognized as a supported pipeline format.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pipeliner_core::input::PipelineInput;
    /// use std::path::Path;
    ///
    /// let input = PipelineInput::detect(Path::new("ci.json")).unwrap();
    /// assert!(matches!(input, PipelineInput::JsonFile(_)));
    /// ```
    pub fn detect(path: &std::path::Path) -> Result<Self, InputError> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => Ok(Self::RustScript(path.to_path_buf())),
            Some("json") => Ok(Self::JsonFile(path.to_path_buf())),
            Some("toml") => Ok(Self::TomlFile(path.to_path_buf())),
            Some("yaml" | "yml") => Err(InputError::UnknownFormat(path.to_path_buf())),
            _ => Err(InputError::UnknownFormat(path.to_path_buf())),
        }
    }

    /// Detects input type from a string path.
    ///
    /// Convenience wrapper around `detect()` that accepts a string.
    ///
    /// # Errors
    ///
    /// Same as `detect()`.
    pub fn from_path(path: &str) -> Result<Self, InputError> {
        Self::detect(std::path::Path::new(path))
    }

    /// Returns the file path for file-based inputs, or `None` for
    /// `Expr`, `Url`, or `Stdin` inputs.
    #[must_use]
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::RustScript(path)
            | Self::JsonFile(path)
            | Self::TomlFile(path) => Some(path),
            Self::Expr(_) | Self::Url(_) | Self::Stdin => None,
        }
    }

    /// Returns a human-readable name for the input format.
    #[must_use]
    pub fn format_name(&self) -> &str {
        match self {
            Self::RustScript(_) => "Rust Script",
            Self::JsonFile(_) => "JSON",
            Self::TomlFile(_) => "TOML",
            Self::Expr(_) => "Expression",
            Self::Url(_) => "URL",
            Self::Stdin => "STDIN",
        }
    }

    /// Parses the input into a `Pipeline`.
    ///
    /// - For **JSON** files: reads and deserializes via `PipelineConfig::from_json()`
    ///   then extracts the pipeline.
    /// - For **Rust Script** files: returns `InputError::UnsupportedSource` since
    ///   Rust script execution requires the `pipeliner-script` crate.
    /// - For **TOML** files: reads and deserializes via `PipelineConfig::from_json()`
    ///   (TOML support uses JSON-compatible structure).
    /// - For **Expr**, **Url**, **Stdin**: returns `InputError::UnsupportedSource`
    ///   as these are not yet implemented.
    ///
    /// # Errors
    ///
    /// Returns `InputError` variants for file I/O, parsing, or validation failures.
    pub fn parse(&self) -> Result<Pipeline, InputError> {
        match self {
            Self::JsonFile(path) => {
                let content = std::fs::read_to_string(path).map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        InputError::FileNotFound(path.clone())
                    } else {
                        InputError::Io(e)
                    }
                })?;
                let config = PipelineConfig::from_json(&content)?;
                config
                    .spec
                    .pipeline
                    .ok_or_else(|| InputError::Validation("No pipeline definition in config".to_string()))
                    .and_then(|p| {
                        p.validate()
                            .map_err(|e| InputError::Validation(e.to_string()))?;
                        Ok(p)
                    })
            }
            Self::TomlFile(path) => {
                // TOML support - parse as JSON-compatible structure
                let content = std::fs::read_to_string(path).map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        InputError::FileNotFound(path.clone())
                    } else {
                        InputError::Io(e)
                    }
                })?;
                let config = PipelineConfig::from_json(&content)?;
                config
                    .spec
                    .pipeline
                    .ok_or_else(|| InputError::Validation("No pipeline definition in config".to_string()))
                    .and_then(|p| {
                        p.validate()
                            .map_err(|e| InputError::Validation(e.to_string()))?;
                        Ok(p)
                    })
            }
            Self::RustScript(_) => Err(InputError::UnsupportedSource(
                "Rust script execution requires pipeliner-script crate".to_string(),
            )),
            Self::Expr(_) => Err(InputError::UnsupportedSource(
                "Inline expressions not yet supported".to_string(),
            )),
            Self::Url(_) => Err(InputError::UnsupportedSource(
                "URL input not yet supported".to_string(),
            )),
            Self::Stdin => Err(InputError::UnsupportedSource(
                "STDIN input not yet supported".to_string(),
            )),
        }
    }

    /// Checks if this input type requires the `pipeliner-script` crate for execution.
    #[must_use]
    pub fn requires_script_engine(&self) -> bool {
        matches!(self, Self::RustScript(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // =======================================================================
    // PipelineInput::detect() Tests
    // =======================================================================

    #[test]
    fn test_detect_rust_script() {
        let input = PipelineInput::detect(Path::new("pipeline.rs")).unwrap();
        assert!(matches!(input, PipelineInput::RustScript(_)));
        if let PipelineInput::RustScript(path) = input {
            assert_eq!(path, PathBuf::from("pipeline.rs"));
        }
    }

    #[test]
    fn test_detect_json_extension() {
        let input = PipelineInput::detect(Path::new("pipeline.json")).unwrap();
        assert!(matches!(input, PipelineInput::JsonFile(_)));
        if let PipelineInput::JsonFile(path) = input {
            assert_eq!(path, PathBuf::from("pipeline.json"));
        }
    }

    #[test]
    fn test_detect_toml_extension() {
        let input = PipelineInput::detect(Path::new("pipeline.toml")).unwrap();
        assert!(matches!(input, PipelineInput::TomlFile(_)));
        if let PipelineInput::TomlFile(path) = input {
            assert_eq!(path, PathBuf::from("pipeline.toml"));
        }
    }

    #[test]
    fn test_detect_unknown_extension() {
        let result = PipelineInput::detect(Path::new("pipeline.yaml"));
        assert!(result.is_err());
        match result.unwrap_err() {
            InputError::UnknownFormat(path) => {
                assert_eq!(path, PathBuf::from("pipeline.yaml"));
            }
            _ => panic!("Expected UnknownFormat error"),
        }
    }

    #[test]
    fn test_detect_no_extension() {
        let result = PipelineInput::detect(Path::new("pipelinefile"));
        assert!(result.is_err());
        match result.unwrap_err() {
            InputError::UnknownFormat(path) => {
                assert_eq!(path, PathBuf::from("pipelinefile"));
            }
            _ => panic!("Expected UnknownFormat error"),
        }
    }

    #[test]
    fn test_detect_path_with_directory() {
        let result = PipelineInput::detect(Path::new("/home/user/pipelines/ci.yaml"));
        assert!(result.is_err());
        match result.unwrap_err() {
            InputError::UnknownFormat(_) => {}
            _ => panic!("Expected UnknownFormat error"),
        }
    }

    // =======================================================================
    // PipelineInput::from_path() Tests
    // =======================================================================

    #[test]
    fn test_from_path_json() {
        let input = PipelineInput::from_path("ci.json").unwrap();
        assert!(matches!(input, PipelineInput::JsonFile(_)));
    }

    #[test]
    fn test_from_path_rust() {
        let input = PipelineInput::from_path("pipeline.rs").unwrap();
        assert!(matches!(input, PipelineInput::RustScript(_)));
    }

    // =======================================================================
    // PipelineInput::path() Tests
    // =======================================================================

    #[test]
    fn test_path_returns_path_for_file_inputs() {
        let rs_input = PipelineInput::RustScript(PathBuf::from("test.rs"));
        assert_eq!(rs_input.path(), Some(&PathBuf::from("test.rs")));

        let json_input = PipelineInput::JsonFile(PathBuf::from("test.json"));
        assert_eq!(json_input.path(), Some(&PathBuf::from("test.json")));

        let toml_input = PipelineInput::TomlFile(PathBuf::from("test.toml"));
        assert_eq!(toml_input.path(), Some(&PathBuf::from("test.toml")));
    }

    #[test]
    fn test_path_returns_none_for_non_file_inputs() {
        assert!(PipelineInput::Expr("test".to_string()).path().is_none());
        assert!(PipelineInput::Url("http://example.com".to_string()).path().is_none());
        assert!(PipelineInput::Stdin.path().is_none());
    }

    // =======================================================================
    // PipelineInput::format_name() Tests
    // =======================================================================

    #[test]
    fn test_format_name() {
        assert_eq!(PipelineInput::RustScript(PathBuf::from("test.rs")).format_name(), "Rust Script");
        assert_eq!(PipelineInput::JsonFile(PathBuf::from("test.json")).format_name(), "JSON");
        assert_eq!(PipelineInput::TomlFile(PathBuf::from("test.toml")).format_name(), "TOML");
        assert_eq!(PipelineInput::Expr("test".to_string()).format_name(), "Expression");
        assert_eq!(PipelineInput::Url("http://example.com".to_string()).format_name(), "URL");
        assert_eq!(PipelineInput::Stdin.format_name(), "STDIN");
    }

    // =======================================================================
    // PipelineInput::requires_script_engine() Tests
    // =======================================================================

    #[test]
    fn test_requires_script_engine() {
        assert!(PipelineInput::RustScript(PathBuf::from("test.rs")).requires_script_engine());
        assert!(!PipelineInput::JsonFile(PathBuf::from("test.json")).requires_script_engine());
        assert!(!PipelineInput::TomlFile(PathBuf::from("test.toml")).requires_script_engine());
        assert!(!PipelineInput::Expr("test".to_string()).requires_script_engine());
        assert!(!PipelineInput::Url("http://example.com".to_string()).requires_script_engine());
        assert!(!PipelineInput::Stdin.requires_script_engine());
    }

    // =======================================================================
    // PipelineInput::parse() Tests
    // =======================================================================

    #[test]
    fn test_parse_json_file_success() {
        // Create a temp JSON file
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test-pipeline.json");
        let json_content = r#"{
            "version": "1",
            "spec": {
                "pipeline": {
                    "name": "Test Pipeline",
                    "stages": [
                        {
                            "name": "Build",
                            "steps": [
                                {"type": "echo", "message": "Hello"}
                            ]
                        }
                    ]
                }
            }
        }"#;
        std::fs::write(&file_path, json_content).unwrap();

        let input = PipelineInput::JsonFile(file_path);
        let result = input.parse();
        assert!(result.is_ok(), "Expected parse to succeed, got: {:?}", result.err());
        let pipeline = result.unwrap();
        assert_eq!(pipeline.name(), Some("Test Pipeline"));
    }

    #[test]
    fn test_parse_json_file_not_found() {
        let input = PipelineInput::JsonFile(PathBuf::from("/nonexistent/path/pipeline.json"));
        let result = input.parse();
        assert!(result.is_err());
        match result.unwrap_err() {
            InputError::FileNotFound(path) => {
                assert!(path.to_string_lossy().contains("pipeline.json"));
            }
            other => panic!("Expected FileNotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_json_file_invalid_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("bad-pipeline.json");
        std::fs::write(&file_path, "not: valid: json: {{{").unwrap();

        let input = PipelineInput::JsonFile(file_path);
        let result = input.parse();
        // Should error on invalid JSON
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_json_file_no_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("config-only.json");
        let json_content = r#"{
            "version": "1",
            "spec": {
                "libraries": []
            }
        }"#;
        std::fs::write(&file_path, json_content).unwrap();

        let input = PipelineInput::JsonFile(file_path);
        let result = input.parse();
        assert!(result.is_err());
        match result.unwrap_err() {
            InputError::Validation(msg) => {
                assert!(msg.contains("No pipeline definition"));
            }
            other => panic!("Expected Validation error, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_rust_script_unsupported() {
        let input = PipelineInput::RustScript(PathBuf::from("pipeline.rs"));
        let result = input.parse();
        assert!(result.is_err());
        match result.unwrap_err() {
            InputError::UnsupportedSource(msg) => {
                assert!(msg.contains("pipeliner-script"));
            }
            other => panic!("Expected UnsupportedSource, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_expr_unsupported() {
        let input = PipelineInput::Expr("pipeline! {}".to_string());
        let result = input.parse();
        assert!(result.is_err());
        match result.unwrap_err() {
            InputError::UnsupportedSource(msg) => {
                assert!(msg.contains("not yet supported"));
            }
            other => panic!("Expected UnsupportedSource, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_url_unsupported() {
        let input = PipelineInput::Url("https://example.com/pipeline.json".to_string());
        let result = input.parse();
        assert!(result.is_err());
        match result.unwrap_err() {
            InputError::UnsupportedSource(msg) => {
                assert!(msg.contains("not yet supported"));
            }
            other => panic!("Expected UnsupportedSource, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_stdin_unsupported() {
        let result = PipelineInput::Stdin.parse();
        assert!(result.is_err());
        match result.unwrap_err() {
            InputError::UnsupportedSource(msg) => {
                assert!(msg.contains("STDIN"));
            }
            other => panic!("Expected UnsupportedSource, got: {:?}", other),
        }
    }

    // =======================================================================
    // InputError Tests
    // =======================================================================

    #[test]
    fn test_input_error_display_unknown_format() {
        let err = InputError::UnknownFormat(PathBuf::from("test.json"));
        let display = format!("{}", err);
        assert!(display.contains("Unknown pipeline file format"));
        assert!(display.contains("test.json"));
    }

    #[test]
    fn test_input_error_display_file_not_found() {
        let err = InputError::FileNotFound(PathBuf::from("/missing.json"));
        let display = format!("{}", err);
        assert!(display.contains("not found"));
    }

    #[test]
    fn test_input_error_display_io() {
        let err = InputError::Io(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no access"));
        let display = format!("{}", err);
        assert!(display.contains("I/O error"));
    }

    #[test]
    fn test_input_error_display_validation() {
        let err = InputError::Validation("bad config".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Validation error"));
        assert!(display.contains("bad config"));
    }

    #[test]
    fn test_input_error_display_unsupported_source() {
        let err = InputError::UnsupportedSource("not supported".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Unsupported input source"));
    }

    #[test]
    fn test_input_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let input_err: InputError = io_err.into();
        assert!(matches!(input_err, InputError::Io(_)));
    }

    // =======================================================================
    // Integration: detect → parse round-trip
    // =======================================================================

    #[test]
    fn test_detect_and_parse_json_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("round-trip.json");
        let json_content = r#"{
            "version": "1",
            "spec": {
                "pipeline": {
                    "name": "Round Trip Pipeline",
                    "stages": [
                        {
                            "name": "Build",
                            "steps": [
                                {"type": "echo", "message": "Round trip works"}
                            ]
                        }
                    ]
                }
            }
        }"#;
        std::fs::write(&file_path, json_content).unwrap();

        // Detect the input type
        let input = PipelineInput::detect(&file_path).unwrap();
        assert!(matches!(input, PipelineInput::JsonFile(_)));

        // Parse it
        let result = input.parse();
        assert!(result.is_ok());
        let pipeline = result.unwrap();
        assert_eq!(pipeline.name(), Some("Round Trip Pipeline"));
        assert_eq!(pipeline.stages.len(), 1);
        assert_eq!(pipeline.stages[0].name, "Build");
    }
}
