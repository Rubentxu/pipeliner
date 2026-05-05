//! Local filesystem source retriever.

use std::path::PathBuf;

use async_trait::async_trait;
use pipeliner_core::config::{LibraryConfig, RetrieverType};

use crate::artifacts::LibraryArtifacts;
use crate::error::LibraryError;
use crate::SourceRetriever;

/// Retriever that loads library artifacts from a local filesystem path.
#[derive(Debug, Clone)]
pub struct LocalSource {
    /// The base path to search from
    pub base_path: PathBuf,
}

impl LocalSource {
    /// Creates a new LocalSource retriever.
    ///
    /// # Arguments
    ///
    /// * `base_path` - The base directory path to search for library artifacts
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Validates that the base path exists and returns an error if not.
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
}

#[async_trait]
impl SourceRetriever for LocalSource {
    async fn retrieve(&self, config: &LibraryConfig) -> Result<LibraryArtifacts, LibraryError> {
        // Validate the base path exists
        self.validate_path()?;

        // The source_path in config is used as a relative path under base_path
        // If source_path is empty or ".", we use base_path directly
        let search_path = if config.source_path.is_empty() || config.source_path == "." {
            self.base_path.clone()
        } else {
            self.base_path.join(&config.source_path)
        };

        // Validate the search path exists
        if !search_path.exists() {
            return Err(LibraryError::SourceNotFound(search_path.display().to_string()));
        }

        // Discover artifacts in the search path
        LibraryArtifacts::discover_from(&search_path)
    }

    fn retriever_type(&self) -> RetrieverType {
        RetrieverType::LocalSource
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::TempDir;

    // ===================================================================
    // C6: LocalSource Struct Tests (RED → GREEN)
    // ===================================================================

    #[test]
    fn test_local_source_new() {
        let path = PathBuf::from("/path/to/lib");
        let source = LocalSource::new(path.clone());
        assert_eq!(source.base_path, path);
    }

    #[test]
    fn test_local_source_debug() {
        let path = PathBuf::from("/path/to/lib");
        let source = LocalSource::new(path);
        let debug = format!("{:?}", source);
        assert!(debug.contains("LocalSource"));
        assert!(debug.contains("/path/to/lib"));
    }

    #[test]
    fn test_local_source_clone() {
        let path = PathBuf::from("/path/to/lib");
        let source = LocalSource::new(path.clone());
        let cloned = source.clone();
        assert_eq!(cloned.base_path, source.base_path);
    }

    // ===================================================================
    // C7: LocalSource SourceRetriever Implementation Tests (RED → GREEN)
    // ===================================================================

    #[test]
    fn test_local_source_retriever_type() {
        let source = LocalSource::new(PathBuf::from("/path/to/lib"));
        assert_eq!(source.retriever_type(), RetrieverType::LocalSource);
    }

    #[test]
    fn test_local_source_is_send_sync() {
        // LocalSource must be Send + Sync to satisfy SourceRetriever bounds
        fn _assert_send_sync<T: Send + Sync>() {}
        fn _assert_local_source_bounds<T: Send + Sync>() {
            _assert_send_sync::<T>();
        }
        _assert_local_source_bounds::<LocalSource>();
    }

    // ===================================================================
    // C8: LocalSource retrieve Tests (RED → GREEN → TRIANGULATE)
    // ===================================================================

    #[tokio::test]
    async fn test_local_source_retrieve_valid_directory() {
        // Create a temp directory with the expected structure
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create directory structure
        fs::create_dir_all(base.join("src")).expect("Should create src dir");
        fs::create_dir_all(base.join("steps")).expect("Should create steps dir");
        fs::write(base.join("src/lib.rs"), "pub mod foo;").expect("Should create src/lib.rs");
        fs::write(base.join("steps/deploy.yaml"), "name: deploy")
            .expect("Should create steps/deploy.yaml");

        let source = LocalSource::new(base.to_path_buf());

        let config = LibraryConfig {
            name: "test-lib".to_string(),
            source_path: ".".to_string(),
            retriever_type: RetrieverType::LocalSource,
            default_version: None,
            modules: vec![],
        };

        let result = source.retrieve(&config).await;
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        assert!(!artifacts.is_empty());
        assert_eq!(artifacts.source_files.len(), 1);
        assert_eq!(artifacts.step_files.len(), 1);
    }

    #[tokio::test]
    async fn test_local_source_retrieve_empty_directory() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        let source = LocalSource::new(base.to_path_buf());

        let config = LibraryConfig {
            name: "test-lib".to_string(),
            source_path: ".".to_string(),
            retriever_type: RetrieverType::LocalSource,
            default_version: None,
            modules: vec![],
        };

        let result = source.retrieve(&config).await;
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        assert!(artifacts.is_empty());
    }

    #[tokio::test]
    async fn test_local_source_retrieve_nonexistent_path() {
        let source = LocalSource::new(PathBuf::from("/nonexistent/path/to/lib"));

        let config = LibraryConfig {
            name: "test-lib".to_string(),
            source_path: ".".to_string(),
            retriever_type: RetrieverType::LocalSource,
            default_version: None,
            modules: vec![],
        };

        let result = source.retrieve(&config).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LibraryError::SourceNotFound(_)));
    }

    #[tokio::test]
    async fn test_local_source_retrieve_with_source_path() {
        // Test that source_path from config is joined with base_path
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create a nested structure: base/lib/src/
        fs::create_dir_all(base.join("lib").join("src"))
            .expect("Should create nested dirs");
        fs::write(base.join("lib").join("src").join("lib.rs"), "pub mod foo;")
            .expect("Should create lib.rs");

        let source = LocalSource::new(base.to_path_buf());

        let config = LibraryConfig {
            name: "test-lib".to_string(),
            source_path: "lib".to_string(), // Relative path under base
            retriever_type: RetrieverType::LocalSource,
            default_version: None,
            modules: vec![],
        };

        let result = source.retrieve(&config).await;
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        assert_eq!(artifacts.source_files.len(), 1);
        assert!(artifacts.source_files[0]
            .to_str()
            .unwrap()
            .contains("lib.rs"));
    }

    #[tokio::test]
    async fn test_local_source_retrieve_nonexistent_source_path() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let base = temp_dir.path();

        // Create a structure but not the "nonexistent" subdir
        fs::create_dir_all(base.join("src")).expect("Should create src dir");

        let source = LocalSource::new(base.to_path_buf());

        let config = LibraryConfig {
            name: "test-lib".to_string(),
            source_path: "nonexistent".to_string(), // This path doesn't exist
            retriever_type: RetrieverType::LocalSource,
            default_version: None,
            modules: vec![],
        };

        let result = source.retrieve(&config).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LibraryError::SourceNotFound(ref path) if path.contains("nonexistent")));
    }

    // ===================================================================
    // C9: LocalSource validate_path Tests (TRIANGULATE)
    // ===================================================================

    #[test]
    fn test_local_source_validate_path_valid_directory() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let source = LocalSource::new(temp_dir.path().to_path_buf());
        assert!(source.validate_path().is_ok());
    }

    #[test]
    fn test_local_source_validate_path_nonexistent() {
        let source = LocalSource::new(PathBuf::from("/nonexistent/path"));
        let result = source.validate_path();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LibraryError::SourceNotFound(_)));
    }

    #[test]
    fn test_local_source_validate_path_is_file_not_directory() {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "content").expect("Should create file");

        let source = LocalSource::new(file_path);
        let result = source.validate_path();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LibraryError::SourceNotFound(_)));
    }
}
