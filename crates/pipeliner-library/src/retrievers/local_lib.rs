//! Local library retriever for recursive library discovery.

use std::path::PathBuf;

use async_trait::async_trait;
use pipeliner_core::config::{LibraryConfig, RetrieverType};

use crate::artifacts::LibraryArtifacts;
use crate::error::LibraryError;
use crate::SourceRetriever;

/// Retriever that discovers library artifacts recursively from a local directory.
///
/// LocalLib is similar to LocalSource but performs recursive discovery,
/// finding all library artifacts under the base path rather than in a specific
/// subdirectory structure.
#[derive(Debug, Clone)]
pub struct LocalLib {
    /// The base path to search recursively from
    pub base_path: PathBuf,
}

impl LocalLib {
    /// Creates a new LocalLib retriever.
    ///
    /// # Arguments
    ///
    /// * `base_path` - The base directory path to recursively search for library artifacts
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Validates that the base path exists and is a directory.
    fn validate_path(&self) -> Result<(), LibraryError> {
        if !self.base_path.exists() {
            return Err(LibraryError::SourceNotFound(
                self.base_path.display().to_string(),
            ));
        }
        if !self.base_path.is_dir() {
            return Err(LibraryError::SourceNotFound(
                self.base_path.display().to_string(),
            ));
        }
        Ok(())
    }

    /// Recursively discovers library artifacts under the base path.
    ///
    /// This differs from `LibraryArtifacts::discover_from` by performing
    /// recursive discovery - it finds all `src/`, `steps/`, and `resources/`
    /// directories anywhere under the base path.
    fn discover_recursive(&self) -> Result<LibraryArtifacts, LibraryError> {
        use walkdir::WalkDir;

        let mut artifacts = LibraryArtifacts::new();

        for entry in WalkDir::new(&self.base_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Check for src/, steps/, resources/ directories
            if let Some(parent) = path.parent() {
                let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if parent_name == "src" && path.is_file() {
                    artifacts.source_files.push(path.to_path_buf());
                } else if parent_name == "steps" && path.is_file() {
                    artifacts.step_files.push(path.to_path_buf());
                } else if parent_name == "resources" && path.is_file() {
                    artifacts.resource_files.push(path.to_path_buf());
                }
            }
        }

        Ok(artifacts)
    }
}

#[async_trait]
impl SourceRetriever for LocalLib {
    async fn retrieve(&self, _config: &LibraryConfig) -> Result<LibraryArtifacts, LibraryError> {
        // Validate the base path exists
        self.validate_path()?;

        // Discover artifacts recursively
        self.discover_recursive()
    }

    fn retriever_type(&self) -> RetrieverType {
        RetrieverType::LocalLib
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::TempDir;

    // ===================================================================
    // D1: LocalLib Struct Tests (RED → GREEN)
    // ===================================================================

    #[test]
    fn test_local_lib_new() {
        let path = PathBuf::from("/path/to/libs");
        let lib = LocalLib::new(path.clone());
        assert_eq!(lib.base_path, path);
    }

    #[test]
    fn test_local_lib_debug() {
        let path = PathBuf::from("/path/to/libs");
        let lib = LocalLib::new(path);
        let debug = format!("{:?}", lib);
        assert!(debug.contains("LocalLib"));
        assert!(debug.contains("/path/to/libs"));
    }

    #[test]
    fn test_local_lib_clone() {
        let path = PathBuf::from("/path/to/libs");
        let lib = LocalLib::new(path.clone());
        let cloned = lib.clone();
        assert_eq!(cloned.base_path, lib.base_path);
    }

    // ===================================================================
    // D2: LocalLib SourceRetriever Implementation Tests (RED → GREEN)
    // ===================================================================

    #[test]
    fn test_local_lib_retriever_type() {
        let lib = LocalLib::new(PathBuf::from("/path/to/libs"));
        assert_eq!(lib.retriever_type(), RetrieverType::LocalLib);
    }

    #[test]
    fn test_local_lib_is_send_sync() {
        // LocalLib must be Send + Sync to satisfy SourceRetriever bounds
        fn _assert_send_sync<T: Send + Sync>() {}
        fn _assert_local_lib_bounds<T: Send + Sync>() {
            _assert_send_sync::<T>();
        }
        _assert_local_lib_bounds::<LocalLib>();
    }

    // ===================================================================
    // D3: LocalLib retrieve Tests (RED → GREEN → TRIANGULATE)
    // ===================================================================

    #[tokio::test]
    async fn test_local_lib_retrieve_valid_directory() {
        // Create a temp directory with nested library structure
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create a nested structure with multiple libraries
        // lib1/src/lib.rs
        fs::create_dir_all(base.join("lib1").join("src"))
            .expect("Should create lib1/src dir");
        fs::write(
            base.join("lib1").join("src").join("lib.rs"),
            "pub mod lib1;",
        )
        .expect("Should create lib1/src/lib.rs");
        fs::create_dir_all(base.join("lib1").join("steps"))
            .expect("Should create lib1/steps dir");
        fs::write(
            base.join("lib1").join("steps").join("deploy.yaml"),
            "name: deploy",
        )
        .expect("Should create lib1/steps/deploy.yaml");

        // lib2/src/main.rs
        fs::create_dir_all(base.join("lib2").join("src"))
            .expect("Should create lib2/src dir");
        fs::write(
            base.join("lib2").join("src").join("main.rs"),
            "fn main() {}",
        )
        .expect("Should create lib2/src/main.rs");

        let lib = LocalLib::new(base.to_path_buf());

        let config = LibraryConfig {
            name: "test-libs".to_string(),
            source_path: ".".to_string(),
            retriever_type: RetrieverType::LocalLib,
            default_version: None,
            modules: vec![],
        };

        let result = lib.retrieve(&config).await;
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        assert!(!artifacts.is_empty());
        // Should find files from both lib1 and lib2
        assert_eq!(artifacts.source_files.len(), 2);
        assert_eq!(artifacts.step_files.len(), 1);
    }

    #[tokio::test]
    async fn test_local_lib_retrieve_empty_directory() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        let lib = LocalLib::new(base.to_path_buf());

        let config = LibraryConfig {
            name: "test-libs".to_string(),
            source_path: ".".to_string(),
            retriever_type: RetrieverType::LocalLib,
            default_version: None,
            modules: vec![],
        };

        let result = lib.retrieve(&config).await;
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        assert!(artifacts.is_empty());
    }

    #[tokio::test]
    async fn test_local_lib_retrieve_nonexistent_path() {
        let lib = LocalLib::new(PathBuf::from("/nonexistent/path/to/libs"));

        let config = LibraryConfig {
            name: "test-libs".to_string(),
            source_path: ".".to_string(),
            retriever_type: RetrieverType::LocalLib,
            default_version: None,
            modules: vec![],
        };

        let result = lib.retrieve(&config).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LibraryError::SourceNotFound(_)));
    }

    #[tokio::test]
    async fn test_local_lib_retrieve_deeply_nested() {
        // Test discovery at multiple nesting levels
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create deeply nested structure
        // level1/level2/level3/libs/my_lib/src/lib.rs
        fs::create_dir_all(
            base.join("level1")
                .join("level2")
                .join("level3")
                .join("libs")
                .join("my_lib")
                .join("src"),
        )
        .expect("Should create deeply nested dirs");
        fs::write(
            base.join("level1")
                .join("level2")
                .join("level3")
                .join("libs")
                .join("my_lib")
                .join("src")
                .join("lib.rs"),
            "pub mod nested;",
        )
        .expect("Should create deeply nested lib.rs");

        let lib = LocalLib::new(base.to_path_buf());

        let config = LibraryConfig {
            name: "test-libs".to_string(),
            source_path: ".".to_string(),
            retriever_type: RetrieverType::LocalLib,
            default_version: None,
            modules: vec![],
        };

        let result = lib.retrieve(&config).await;
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        // Should still find the deeply nested file
        assert_eq!(artifacts.source_files.len(), 1);
        assert!(artifacts.source_files[0]
            .to_str()
            .unwrap()
            .contains("lib.rs"));
    }

    #[tokio::test]
    async fn test_local_lib_retrieve_multiple_libraries() {
        // Test that we can discover from multiple sibling libraries
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create three separate libraries
        for i in 1..=3 {
            let lib_dir = base.join(format!("library{}", i));
            fs::create_dir_all(lib_dir.join("src")).expect("Should create src dir");
            fs::create_dir_all(lib_dir.join("resources")).expect("Should create resources dir");
            fs::write(
                lib_dir.join("src").join(format!("mod{}.rs", i)),
                format!("pub mod library{};", i),
            )
            .expect("Should create mod file");
            fs::write(
                lib_dir.join("resources").join(format!("config{}.json", i)),
                format!("{{\"id\": {}}}", i),
            )
            .expect("Should create resource file");
        }

        let lib = LocalLib::new(base.to_path_buf());

        let config = LibraryConfig {
            name: "test-libs".to_string(),
            source_path: ".".to_string(),
            retriever_type: RetrieverType::LocalLib,
            default_version: None,
            modules: vec![],
        };

        let result = lib.retrieve(&config).await;
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        assert_eq!(artifacts.source_files.len(), 3);
        assert_eq!(artifacts.resource_files.len(), 3);
        assert!(artifacts.step_files.is_empty());
    }

    // ===================================================================
    // D4: LocalLib discover_recursive Tests (TRIANGULATE)
    // ===================================================================

    #[test]
    fn test_local_lib_discover_recursive_finds_nested_src() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create nested src directories
        fs::create_dir_all(base.join("project").join("src"))
            .expect("Should create nested src dir");
        fs::write(
            base.join("project").join("src").join("lib.rs"),
            "pub mod project;",
        )
        .expect("Should create nested lib.rs");

        let local_lib = LocalLib::new(base.to_path_buf());
        let artifacts = local_lib.discover_recursive().expect("Should discover artifacts");

        assert_eq!(artifacts.source_files.len(), 1);
        assert!(artifacts.source_files[0]
            .to_str()
            .unwrap()
            .contains("lib.rs"));
    }

    #[test]
    fn test_local_lib_discover_recursive_empty_when_no_matching_dirs() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create a file at the root, but no src/steps/resources directories
        fs::create_dir_all(base.join("docs")).expect("Should create docs dir");
        fs::write(base.join("README.md"), "# Test").expect("Should create README");

        let local_lib = LocalLib::new(base.to_path_buf());
        let artifacts = local_lib.discover_recursive().expect("Should discover artifacts");

        assert!(artifacts.is_empty());
    }

    #[test]
    fn test_local_lib_discover_recursive_ignores_non_matching_parent_names() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create directories that look similar but aren't the expected structure
        // e.g., "src_backup" should not be matched as "src"
        fs::create_dir_all(base.join("src_backup")).expect("Should create src_backup dir");
        fs::write(base.join("src_backup").join("lib.rs"), "pub mod backup;")
            .expect("Should create backup lib.rs");

        let local_lib = LocalLib::new(base.to_path_buf());
        let artifacts = local_lib.discover_recursive().expect("Should discover artifacts");

        // Should not find src_backup as a source directory
        // because we only match "src", not "src_backup"
        assert_eq!(artifacts.source_files.len(), 0);
    }

    // ===================================================================
    // D5: LocalLib validate_path Tests (TRIANGULATE)
    // ===================================================================

    #[test]
    fn test_local_lib_validate_path_valid_directory() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let lib = LocalLib::new(temp_dir.path().to_path_buf());
        assert!(lib.validate_path().is_ok());
    }

    #[test]
    fn test_local_lib_validate_path_nonexistent() {
        let lib = LocalLib::new(PathBuf::from("/nonexistent/path"));
        let result = lib.validate_path();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LibraryError::SourceNotFound(_)));
    }

    #[test]
    fn test_local_lib_validate_path_is_file_not_directory() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "content").expect("Should create file");

        let lib = LocalLib::new(file_path);
        let result = lib.validate_path();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LibraryError::SourceNotFound(_)));
    }
}
