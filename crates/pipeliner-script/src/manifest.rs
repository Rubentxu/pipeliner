//! # Manifest Module
//!
//! Parses Rust script manifest comments from script files.
//!
//! Scripts can declare dependencies using manifest comments:
//!
//! ```rust
//! #!/usr/bin/env rustline-run
//! //! [dependencies]
//! //! serde = "1.0"
//! //! serde_json = "1.0"
//!
//! fn main() {
//!     println!("Hello from script!");
//! }
//! ```
//!
//! The parser extracts lines matching the pattern `//! [key] value` or `//! key = value`.

use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Manifest section types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section {
    /// Dependencies section: `//! [dependencies]`
    Dependencies,
    /// Dev-dependencies section: `//! [dev-dependencies]`
    DevDependencies,
    /// Build-dependencies section: `//! [build-dependencies]`
    BuildDependencies,
    /// Unknown section: `//! [unknown]`
    Unknown(String),
}

impl Section {
    fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "dependencies" => Section::Dependencies,
            "dev-dependencies" => Section::DevDependencies,
            "build-dependencies" => Section::BuildDependencies,
            other => Section::Unknown(other.to_string()),
        }
    }
}

/// A parsed manifest from a Rust script.
#[derive(Debug, Clone, Default)]
pub struct Manifest {
    /// Raw dependencies from `[dependencies]` section
    pub dependencies: Vec<String>,
    /// Raw dev-dependencies from `[dev-dependencies]` section
    pub dev_dependencies: Vec<String>,
    /// Raw build-dependencies from `[build-dependencies]` section
    pub build_dependencies: Vec<String>,
    /// Additional metadata extracted from comments
    pub metadata: HashMap<String, String>,
}

impl Manifest {
    /// Creates a new empty manifest.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a manifest from a script file path.
    ///
    /// # Errors
    ///
    /// Returns `ManifestError` if the file cannot be read or parsed.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| ManifestError::IoError(path.as_ref().display().to_string(), e.to_string()))?;
        Self::parse(&content)
    }

    /// Parses a manifest from script content string.
    ///
    /// # Errors
    ///
    /// Returns `ManifestError` if the content cannot be parsed.
    pub fn parse(content: &str) -> Result<Self, ManifestError> {
        let mut manifest = Self::new();
        let mut current_section: Option<Section> = None;

        // Regex for section headers: //! [section-name]
        let section_re = Regex::new(r"^//!\s*\[\s*([\w-]+)\s*\]").unwrap();

        // Regex for key = "value" or key = value
        let kv_re = Regex::new(r#"^//!\s*(\w+)\s*=\s*(.+)$"#).unwrap();

        for line in content.lines() {
            let trimmed = line.trim();

            // Check for section header
            if let Some(caps) = section_re.captures(trimmed) {
                let section_name = &caps[1];
                current_section = Some(Section::from_name(section_name));
                continue;
            }

            // Skip non-manifest lines
            if !trimmed.starts_with("//!") {
                continue;
            }

            // Extract the content after "//!"
            let manifest_line = trimmed.trim_start_matches("//!").trim();

            // Skip empty lines
            if manifest_line.is_empty() {
                continue;
            }

            // Try to parse key = value
            if let Some(caps) = kv_re.captures(trimmed) {
                let key = caps[1].to_string();
                let value = caps[2].trim().to_string();

                match current_section.as_ref() {
                    Some(Section::Dependencies) => {
                        manifest.dependencies.push(format!("{} = {}", key, value));
                    }
                    Some(Section::DevDependencies) => {
                        manifest.dev_dependencies.push(format!("{} = {}", key, value));
                    }
                    Some(Section::BuildDependencies) => {
                        manifest.build_dependencies.push(format!("{} = {}", key, value));
                    }
                    _ => {
                        // Outside of any section, treat as dependency
                        manifest.dependencies.push(format!("{} = {}", key, value));
                    }
                }
                continue;
            }
            // Lines with only a crate name (no = version) are not dependencies - skip them
            // Only key=value pairs are actual dependencies
        }

        Ok(manifest)
    }

    /// Returns all dependencies as a single vector.
    #[must_use]
    pub fn all_dependencies(&self) -> Vec<String> {
        let mut deps = self.dependencies.clone();
        deps.extend(self.dev_dependencies.clone());
        deps.extend(self.build_dependencies.clone());
        deps
    }

    /// Returns true if the manifest has no dependencies.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
            && self.dev_dependencies.is_empty()
            && self.build_dependencies.is_empty()
    }
}

/// Manifest parsing errors.
#[derive(Debug, Clone)]
pub enum ManifestError {
    /// I/O error reading the file
    IoError(String, String),
    /// Parse error
    ParseError(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::IoError(path, msg) => {
                write!(f, "Failed to read manifest from '{}': {}", path, msg)
            }
            ManifestError::ParseError(msg) => {
                write!(f, "Manifest parse error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ManifestError {}

impl From<std::io::Error> for ManifestError {
    fn from(err: std::io::Error) -> Self {
        ManifestError::IoError("unknown".to_string(), err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_manifest_new_is_empty() {
        let manifest = Manifest::new();
        assert!(manifest.is_empty());
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn test_parse_simple_dependencies() {
        let content = r#"#!/usr/bin/env rustline-run
//! [dependencies]
//! serde = "1.0"
//! serde_json = "1.0"
//!
//! fn main() {}
"#;
        let manifest = Manifest::parse(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 2);
        assert!(manifest.dependencies.contains(&r#"serde = "1.0""#.to_string()));
        assert!(manifest.dependencies.contains(&r#"serde_json = "1.0""#.to_string()));
    }

    #[test]
    fn test_parse_dev_dependencies() {
        let content = r#"#!/usr/bin/env rustline-run
//! [dev-dependencies]
//! pretty_assertions = "1.0"
//!
//! fn main() {}
"#;
        let manifest = Manifest::parse(content).unwrap();
        assert!(manifest.dependencies.is_empty());
        assert_eq!(manifest.dev_dependencies.len(), 1);
        assert!(manifest.dev_dependencies.contains(&r#"pretty_assertions = "1.0""#.to_string()));
    }

    #[test]
    fn test_parse_multiple_sections() {
        let content = r#"#!/usr/bin/env rustline-run
//! [dependencies]
//! serde = "1.0"
//!
//! [dev-dependencies]
//! pretty_assertions = "1.0"
//!
//! [build-dependencies]
//! built = "0.4"
//!
//! fn main() {}
"#;
        let manifest = Manifest::parse(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dev_dependencies.len(), 1);
        assert_eq!(manifest.build_dependencies.len(), 1);
    }

    #[test]
    fn test_parse_no_manifest_lines() {
        let content = r#"#!/usr/bin/env rustline-run
//!
//! fn main() {
//!     println!("Hello");
//! }
"#;
        let manifest = Manifest::parse(content).unwrap();
        assert!(manifest.is_empty());
    }

    #[test]
    fn test_parse_real_script_example() {
        let content = r#"#!/usr/bin/env rustline-run
//! [dependencies]
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! reqwest = { version = "0.11", features = ["json"] }
//!
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Data {
//!     name: String,
//! }
//!
//! fn main() {
//!     println!("Hello from script!");
//! }
"#;
        let manifest = Manifest::parse(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 3);
        assert!(manifest.dependencies.iter().any(|d| d.contains("serde")));
        assert!(manifest.dependencies.iter().any(|d| d.contains("serde_json")));
        assert!(manifest.dependencies.iter().any(|d| d.contains("reqwest")));
    }

    #[test]
    fn test_from_file() {
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        writeln!(file, "//! [dependencies]").unwrap();
        writeln!(file, "//! serde = \"1.0\"").unwrap();
        writeln!(file, "//!").unwrap();
        writeln!(file, "fn main() {{}}").unwrap();

        let manifest = Manifest::from_file(file.path()).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);
    }

    #[test]
    fn test_from_file_not_found() {
        let result = Manifest::from_file("/nonexistent/path/script.rs");
        assert!(result.is_err());
    }

    #[test]
    fn test_all_dependencies() {
        let mut manifest = Manifest::new();
        manifest.dependencies.push("serde = \"1.0\"".to_string());
        manifest.dev_dependencies.push("pretty_assertions = \"1.0\"".to_string());
        manifest.build_dependencies.push("built = \"0.4\"".to_string());

        let all = manifest.all_dependencies();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_manifest_error_display() {
        let err = ManifestError::IoError("test.rs".to_string(), "file not found".to_string());
        let display = format!("{}", err);
        assert!(display.contains("test.rs"));
        assert!(display.contains("file not found"));

        let err = ManifestError::ParseError("invalid syntax".to_string());
        let display = format!("{}", err);
        assert!(display.contains("invalid syntax"));
    }

    #[test]
    fn test_section_from_name() {
        assert!(matches!(Section::from_name("dependencies"), Section::Dependencies));
        assert!(matches!(Section::from_name("DEV-DEPENDENCIES"), Section::DevDependencies));
        assert!(matches!(Section::from_name("build-dependencies"), Section::BuildDependencies));
        assert!(matches!(Section::from_name("unknown-section"), Section::Unknown(name) if name == "unknown-section"));
    }

    #[test]
    fn test_parse_complex_cargo_format() {
        // Test various cargo dependency formats
        let content = r#"//! [dependencies]
//! serde = { version = "1.0", features = ["derive"] }
//! tokio = { version = "1.0", features = ["full"] }
//! regex = "1.10"
//! once_cell = "1.19"
//!
//! fn main() {}
"#;
        let manifest = Manifest::parse(content).unwrap();
        assert_eq!(manifest.dependencies.len(), 4);
    }
}