//! Library loader with caching, deduplication, and retriever registration.
//!
//! The `LibraryLoader` orchestrates loading libraries from various sources,
//! managing a cache to avoid redundant retrieval operations.

use std::collections::{HashMap, HashSet};

use pipeliner_core::config::{LibraryConfig, RetrieverType};

use crate::artifacts::LibraryArtifacts;
use crate::error::LibraryError;
use crate::retriever::SourceRetriever;

/// Type alias for boxed source retrievers
type BoxedRetriever = Box<dyn SourceRetriever>;

/// Orchestrates library loading with caching and deduplication.
///
/// The loader maintains:
/// - A registry of retrievers for different source types
/// - An in-memory cache of loaded library artifacts
/// - A deduplication set tracking which libraries have been loaded
///
/// # Example
///
/// ```
/// use pipeliner_library::{LibraryLoader, LibraryError};
/// use pipeliner_core::config::{LibraryConfig, RetrieverType};
/// use async_trait::async_trait;
/// use std::path::PathBuf;
///
/// // A simple mock retriever for demonstration
/// struct DummyRetriever;
/// #[async_trait]
/// impl pipeliner_library::SourceRetriever for DummyRetriever {
///     async fn retrieve(&self, _config: &LibraryConfig) -> Result<pipeliner_library::LibraryArtifacts, LibraryError> {
///         Ok(pipeliner_library::LibraryArtifacts::new())
///     }
///     fn retriever_type(&self) -> RetrieverType {
///         RetrieverType::LocalSource
///     }
/// }
///
/// # async {
/// let mut loader = LibraryLoader::new();
/// loader.register_retriever(RetrieverType::LocalSource, Box::new(DummyRetriever));
///
/// let config = LibraryConfig {
///     name: "mylib".to_string(),
///     source_path: "/path/to/lib".to_string(),
///     retriever_type: RetrieverType::LocalSource,
///     default_version: Some("1.0".to_string()),
///     modules: vec![],
/// };
///
/// match loader.load(&config).await {
///     Ok(artifacts) => println!("Loaded {} files", artifacts.total_files()),
///     Err(e) => eprintln!("Failed to load: {}", e),
/// }
/// # };
/// ```
pub struct LibraryLoader {
    /// Registered retrievers by type
    retrievers: HashMap<RetrieverType, BoxedRetriever>,
    /// Cache of loaded library artifacts by key ("name@version")
    cache: HashMap<String, LibraryArtifacts>,
    /// Set of loaded library keys for deduplication
    loaded: HashSet<String>,
}

impl std::fmt::Debug for LibraryLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibraryLoader")
            .field("retrievers", &self.retrievers.len())
            .field("cache", &self.cache.len())
            .field("loaded", &self.loaded.len())
            .finish()
    }
}

impl LibraryLoader {
    /// Creates a new empty LibraryLoader with no registered retrievers.
    #[must_use]
    pub fn new() -> Self {
        Self {
            retrievers: HashMap::new(),
            cache: HashMap::new(),
            loaded: HashSet::new(),
        }
    }

    /// Registers a retriever for a specific type.
    ///
    /// If a retriever for this type already exists, it is replaced.
    ///
    /// # Arguments
    ///
    /// * `retriever_type` - The type of retriever to register
    /// * `retriever` - The retriever instance
    pub fn register_retriever(&mut self, retriever_type: RetrieverType, retriever: BoxedRetriever) {
        self.retrievers.insert(retriever_type, retriever);
    }

    /// Gets a registered retriever for a specific type.
    ///
    /// # Arguments
    ///
    /// * `retriever_type` - The type of retriever to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(&dyn SourceRetriever)` if a retriever is registered for this type,
    /// or `None` if no retriever is registered.
    #[must_use]
    pub fn get_retriever(&self, retriever_type: &RetrieverType) -> Option<&dyn SourceRetriever> {
        self.retrievers.get(retriever_type).map(|r| r.as_ref())
    }

    /// Loads a library, returning cached artifacts if already loaded.
    ///
    /// This method implements deduplication: if a library with the same name and version
    /// has already been loaded, it returns the cached artifacts without calling the retriever.
    ///
    /// # Arguments
    ///
    /// * `config` - The library configuration
    ///
    /// # Returns
    ///
    /// Returns a reference to the cached `LibraryArtifacts` on success,
    /// or `Err(LibraryError)` on failure.
    ///
    /// # Errors
    ///
    /// Returns `LibraryError::InvalidConfig` if no retriever is registered for the
    /// configured retriever type.
    pub async fn load(&mut self, config: &LibraryConfig) -> Result<LibraryArtifacts, LibraryError> {
        // Build the cache key: "name@version" or "name@latest"
        let version = config.default_version.as_deref().unwrap_or("latest");
        let key = format!("{}@{}", config.name, version);

        // Deduplication check - return cached if already loaded
        if let Some(artifacts) = self.cache.get(&key) {
            return Ok(artifacts.clone());
        }

        // Get the appropriate retriever
        let retriever = self.retrievers.get(&config.retriever_type).ok_or_else(|| {
            LibraryError::InvalidConfig(format!(
                "No retriever registered for {:?}",
                config.retriever_type
            ))
        })?;

        // Retrieve artifacts from the source
        let artifacts = retriever.retrieve(config).await?;

        // Cache the artifacts
        self.cache.insert(key.clone(), artifacts.clone());
        self.loaded.insert(key);

        // Return the owned artifacts
        Ok(artifacts)
    }

    /// Checks if a library has been loaded.
    ///
    /// # Arguments
    ///
    /// * `name` - The library name
    /// * `version` - The library version (or "latest")
    ///
    /// # Returns
    ///
    /// Returns `true` if the library has been loaded, `false` otherwise.
    #[must_use]
    pub fn is_loaded(&self, name: &str, version: &str) -> bool {
        let key = format!("{}@{}", name, version);
        self.loaded.contains(&key)
    }

    /// Returns a list of all loaded library keys.
    ///
    /// # Returns
    ///
    /// A vector of strings in the format "name@version"
    #[must_use]
    pub fn loaded_libraries(&self) -> Vec<String> {
        self.loaded.iter().cloned().collect()
    }

    /// Returns the number of cached library artifacts.
    #[must_use]
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Clears all cached artifacts and the loaded set.
    ///
    /// This does not unregister retrievers.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.loaded.clear();
    }

    /// Returns true if no retrievers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.retrievers.is_empty()
    }
}

impl Default for LibraryLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use pipeliner_core::config::RetrieverType;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // ===================================================================
    // Test retrievers for testing LibraryLoader
    // ===================================================================

    /// A mock retriever that returns configurable artifacts
    #[derive(Debug)]
    struct MockRetriever {
        retriever_type: RetrieverType,
        artifacts: LibraryArtifacts,
        delay_ms: u64,
    }

    impl MockRetriever {
        fn new(retriever_type: RetrieverType, artifacts: LibraryArtifacts) -> Self {
            Self {
                retriever_type,
                artifacts,
                delay_ms: 0,
            }
        }

        fn with_delay(mut self, delay_ms: u64) -> Self {
            self.delay_ms = delay_ms;
            self
        }
    }

    #[async_trait]
    impl SourceRetriever for MockRetriever {
        async fn retrieve(&self, _config: &LibraryConfig) -> Result<LibraryArtifacts, LibraryError> {
            if self.delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
            }
            Ok(self.artifacts.clone())
        }

        fn retriever_type(&self) -> RetrieverType {
            self.retriever_type.clone()
        }
    }

    fn create_test_config(name: &str, version: &str, retriever_type: RetrieverType) -> LibraryConfig {
        LibraryConfig {
            name: name.to_string(),
            source_path: format!("https://example.com/{}", name),
            retriever_type,
            default_version: Some(version.to_string()),
            modules: vec![],
        }
    }

    fn create_artifacts_with_files(files: Vec<&str>) -> LibraryArtifacts {
        let mut artifacts = LibraryArtifacts::new();
        for f in files {
            artifacts.source_files.push(PathBuf::from(f));
        }
        artifacts
    }

    // ===================================================================
    // E1: LibraryLoader Basic Tests
    // ===================================================================

    #[test]
    fn test_library_loader_new() {
        let loader = LibraryLoader::new();
        assert!(loader.is_empty());
        assert_eq!(loader.cache_size(), 0);
        assert!(loader.loaded_libraries().is_empty());
    }

    #[test]
    fn test_library_loader_default() {
        let loader = LibraryLoader::default();
        assert!(loader.is_empty());
    }

    // ===================================================================
    // E2: Retriever Registration Tests
    // ===================================================================

    #[test]
    fn test_register_retriever() {
        let mut loader = LibraryLoader::new();
        assert!(loader.is_empty());

        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            LibraryArtifacts::new(),
        ));
        loader.register_retriever(RetrieverType::GitSource, retriever);

        assert!(!loader.is_empty());
        assert!(loader.get_retriever(&RetrieverType::GitSource).is_some());
        assert!(loader.get_retriever(&RetrieverType::LocalSource).is_none());
    }

    #[test]
    fn test_register_multiple_retrievers() {
        let mut loader = LibraryLoader::new();

        let git_retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            LibraryArtifacts::new(),
        ));
        let local_retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::LocalSource,
            LibraryArtifacts::new(),
        ));
        let local_lib_retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::LocalLib,
            LibraryArtifacts::new(),
        ));

        loader.register_retriever(RetrieverType::GitSource, git_retriever);
        loader.register_retriever(RetrieverType::LocalSource, local_retriever);
        loader.register_retriever(RetrieverType::LocalLib, local_lib_retriever);

        assert!(loader.get_retriever(&RetrieverType::GitSource).is_some());
        assert!(loader.get_retriever(&RetrieverType::LocalSource).is_some());
        assert!(loader.get_retriever(&RetrieverType::LocalLib).is_some());
    }

    #[test]
    fn test_replace_retriever() {
        let mut loader = LibraryLoader::new();

        let retriever1: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            create_artifacts_with_files(vec!["file1.rs"]),
        ));
        loader.register_retriever(RetrieverType::GitSource, retriever1);

        let retriever2: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            create_artifacts_with_files(vec!["file2.rs"]),
        ));
        loader.register_retriever(RetrieverType::GitSource, retriever2);

        // The retriever is replaced but loader is not empty
        assert!(!loader.is_empty());
        assert!(loader.get_retriever(&RetrieverType::GitSource).is_some());
    }

    // ===================================================================
    // E3: Cache Integration Tests
    // ===================================================================

    #[tokio::test]
    async fn test_load_returns_artifacts() {
        let mut loader = LibraryLoader::new();
        let artifacts = create_artifacts_with_files(vec!["src/lib.rs", "src/main.rs"]);
        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            artifacts,
        ));
        loader.register_retriever(RetrieverType::GitSource, retriever);

        let config = create_test_config("mylib", "1.0", RetrieverType::GitSource);
        let result = loader.load(&config).await;

        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.source_files.len(), 2);
        assert!(loader.is_loaded("mylib", "1.0"));
    }

    #[tokio::test]
    async fn test_load_caches_artifacts() {
        let mut loader = LibraryLoader::new();
        let artifacts = create_artifacts_with_files(vec!["src/lib.rs"]);
        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            artifacts,
        ));
        loader.register_retriever(RetrieverType::GitSource, retriever);

        let config = create_test_config("mylib", "1.0", RetrieverType::GitSource);

        // First load
        let result1 = loader.load(&config).await;
        assert!(result1.is_ok());
        assert_eq!(loader.cache_size(), 1);

        // Second load - should return cached
        let result2 = loader.load(&config).await;
        assert!(result2.is_ok());
        assert_eq!(loader.cache_size(), 1); // Still 1, not 2
    }

    #[tokio::test]
    async fn test_load_different_libraries() {
        let mut loader = LibraryLoader::new();

        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::LocalSource,
            LibraryArtifacts::new(),
        ));
        loader.register_retriever(RetrieverType::LocalSource, retriever);

        let config1 = create_test_config("lib1", "1.0", RetrieverType::LocalSource);
        let config2 = create_test_config("lib2", "2.0", RetrieverType::LocalSource);

        // Load first library
        let result1 = loader.load(&config1).await;
        assert!(result1.is_ok());
        assert_eq!(loader.cache_size(), 1);

        // Load second library
        let result2 = loader.load(&config2).await;
        assert!(result2.is_ok());
        assert_eq!(loader.cache_size(), 2);
        assert_eq!(loader.loaded_libraries().len(), 2);
    }

    #[tokio::test]
    async fn test_load_with_different_versions() {
        let mut loader = LibraryLoader::new();
        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::LocalSource,
            LibraryArtifacts::new(),
        ));
        loader.register_retriever(RetrieverType::LocalSource, retriever);

        let config1 = create_test_config("mylib", "1.0", RetrieverType::LocalSource);
        let config2 = create_test_config("mylib", "2.0", RetrieverType::LocalSource);

        // Load first version
        let result1 = loader.load(&config1).await;
        assert!(result1.is_ok());
        assert_eq!(loader.cache_size(), 1);

        // Load second version
        let result2 = loader.load(&config2).await;
        assert!(result2.is_ok());
        assert_eq!(loader.cache_size(), 2);
        assert!(loader.is_loaded("mylib", "1.0"));
        assert!(loader.is_loaded("mylib", "2.0"));
    }

    #[tokio::test]
    async fn test_load_same_library_different_types() {
        let mut loader = LibraryLoader::new();

        let git_retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            create_artifacts_with_files(vec!["git_file.rs"]),
        ));
        let local_retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::LocalSource,
            create_artifacts_with_files(vec!["local_file.rs"]),
        ));

        loader.register_retriever(RetrieverType::GitSource, git_retriever);
        loader.register_retriever(RetrieverType::LocalSource, local_retriever);

        let config_git = create_test_config("gitlib", "1.0", RetrieverType::GitSource);
        let config_local = create_test_config("localib", "1.0", RetrieverType::LocalSource);

        // Load via different retrievers
        let result1 = loader.load(&config_git).await;
        assert!(result1.is_ok());

        let result2 = loader.load(&config_local).await;
        assert!(result2.is_ok());

        assert_eq!(loader.cache_size(), 2);
    }

    // ===================================================================
    // E4: Deduplication Tests (SCN-LS-006)
    // ===================================================================

    #[tokio::test]
    async fn test_deduplication_by_name_and_version() {
        let mut loader = LibraryLoader::new();
        let artifacts = create_artifacts_with_files(vec!["src/lib.rs"]);

        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            artifacts,
        ));
        loader.register_retriever(RetrieverType::GitSource, retriever);

        let config = create_test_config("mylib", "1.0", RetrieverType::GitSource);

        // First load
        let result1 = loader.load(&config).await;
        assert!(result1.is_ok());
        assert_eq!(loader.cache_size(), 1);

        // Second load - should deduplicate (same key, returns cached)
        let result2 = loader.load(&config).await;
        assert!(result2.is_ok());

        // Should still have only 1 cached entry
        assert_eq!(loader.cache_size(), 1);
        assert_eq!(loader.loaded_libraries().len(), 1);
    }

    // ===================================================================
    // E5: Error Handling Tests
    // ===================================================================

    #[tokio::test]
    async fn test_load_no_retriever_registered() {
        let mut loader = LibraryLoader::new();
        // No retrievers registered

        let config = create_test_config("mylib", "1.0", RetrieverType::GitSource);
        let result = loader.load(&config).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LibraryError::InvalidConfig(_)));
    }

    #[tokio::test]
    async fn test_load_invalid_config_error_message() {
        let mut loader = LibraryLoader::new();

        let config = create_test_config("mylib", "1.0", RetrieverType::LocalSource);
        let result = loader.load(&config).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("Invalid library configuration"));
        assert!(err_str.contains("LocalSource"));
    }

    // ===================================================================
    // E6: Cache Management Tests
    // ===================================================================

    #[tokio::test]
    async fn test_clear_cache() {
        let mut loader = LibraryLoader::new();
        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            create_artifacts_with_files(vec!["file.rs"]),
        ));
        loader.register_retriever(RetrieverType::GitSource, retriever);

        let config = create_test_config("mylib", "1.0", RetrieverType::GitSource);
        loader.load(&config).await.unwrap();

        assert_eq!(loader.cache_size(), 1);
        assert!(!loader.loaded_libraries().is_empty());

        loader.clear_cache();

        assert_eq!(loader.cache_size(), 0);
        assert!(loader.loaded_libraries().is_empty());
        // But retrievers should still be registered
        assert!(!loader.is_empty());
    }

    #[test]
    fn test_is_loaded_not_loaded() {
        let loader = LibraryLoader::new();
        assert!(!loader.is_loaded("mylib", "1.0"));
        assert!(!loader.is_loaded("other", "latest"));
    }

    #[tokio::test]
    async fn test_is_loaded_after_load() {
        let mut loader = LibraryLoader::new();
        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            create_artifacts_with_files(vec!["file.rs"]),
        ));
        loader.register_retriever(RetrieverType::GitSource, retriever);

        assert!(!loader.is_loaded("mylib", "1.0"));

        let config = create_test_config("mylib", "1.0", RetrieverType::GitSource);
        loader.load(&config).await.unwrap();

        assert!(loader.is_loaded("mylib", "1.0"));
        assert!(!loader.is_loaded("mylib", "2.0")); // Different version
        assert!(!loader.is_loaded("other", "1.0")); // Different name
    }

    #[tokio::test]
    async fn test_loaded_libraries() {
        let mut loader = LibraryLoader::new();
        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::LocalSource,
            LibraryArtifacts::new(),
        ));
        loader.register_retriever(RetrieverType::LocalSource, retriever);

        let config1 = create_test_config("lib1", "1.0", RetrieverType::LocalSource);
        let config2 = create_test_config("lib2", "2.0", RetrieverType::LocalSource);

        loader.load(&config1).await.unwrap();
        loader.load(&config2).await.unwrap();

        let loaded = loader.loaded_libraries();
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains(&"lib1@1.0".to_string()));
        assert!(loaded.contains(&"lib2@2.0".to_string()));
    }

    // ===================================================================
    // E7: Multiple Retrievers Tests
    // ===================================================================

    #[tokio::test]
    async fn test_multiple_retriever_types() {
        let mut loader = LibraryLoader::new();

        let git_retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            create_artifacts_with_files(vec!["git_step.yaml"]),
        ));
        let local_retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::LocalSource,
            create_artifacts_with_files(vec!["local_step.yaml"]),
        ));
        let local_lib_retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::LocalLib,
            create_artifacts_with_files(vec!["lib_step.rs"]),
        ));

        loader.register_retriever(RetrieverType::GitSource, git_retriever);
        loader.register_retriever(RetrieverType::LocalSource, local_retriever);
        loader.register_retriever(RetrieverType::LocalLib, local_lib_retriever);

        let git_config = create_test_config("gitlib", "main", RetrieverType::GitSource);
        let local_config = create_test_config("localib", "1.0", RetrieverType::LocalSource);
        let local_lib_config = create_test_config("localrs", "0.1", RetrieverType::LocalLib);

        // Load via git retriever
        let git_result = loader.load(&git_config).await;
        assert!(git_result.is_ok());

        // Load via local retriever
        let local_result = loader.load(&local_config).await;
        assert!(local_result.is_ok());

        // Load via local_lib retriever
        let local_lib_result = loader.load(&local_lib_config).await;
        assert!(local_lib_result.is_ok());

        assert_eq!(loader.cache_size(), 3);
    }

    // ===================================================================
    // E8: is_loaded with different version formats
    // ===================================================================

    #[tokio::test]
    async fn test_is_loaded_with_none_version() {
        let mut loader = LibraryLoader::new();
        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::GitSource,
            create_artifacts_with_files(vec!["file.rs"]),
        ));
        loader.register_retriever(RetrieverType::GitSource, retriever);

        // Config with default_version = None (should use "latest")
        let config = LibraryConfig {
            name: "mylib".to_string(),
            source_path: "https://example.com/mylib".to_string(),
            retriever_type: RetrieverType::GitSource,
            default_version: None, // This means "latest"
            modules: vec![],
        };

        loader.load(&config).await.unwrap();

        assert!(loader.is_loaded("mylib", "latest"));
        assert!(!loader.is_loaded("mylib", "1.0")); // Explicit version not loaded
    }

    #[test]
    fn test_loaded_libraries_empty() {
        let loader = LibraryLoader::new();
        assert!(loader.loaded_libraries().is_empty());
    }

    #[test]
    fn test_cache_size_empty() {
        let loader = LibraryLoader::new();
        assert_eq!(loader.cache_size(), 0);
    }

    #[tokio::test]
    async fn test_cache_size_after_loads() {
        let mut loader = LibraryLoader::new();
        let retriever: Box<dyn SourceRetriever> = Box::new(MockRetriever::new(
            RetrieverType::LocalSource,
            LibraryArtifacts::new(),
        ));
        loader.register_retriever(RetrieverType::LocalSource, retriever);

        assert_eq!(loader.cache_size(), 0);

        let config = create_test_config("lib1", "1.0", RetrieverType::LocalSource);
        loader.load(&config).await.unwrap();
        assert_eq!(loader.cache_size(), 1);

        let config2 = create_test_config("lib2", "1.0", RetrieverType::LocalSource);
        loader.load(&config2).await.unwrap();
        assert_eq!(loader.cache_size(), 2);

        // Same library again - should not increase
        loader.load(&config).await.unwrap();
        assert_eq!(loader.cache_size(), 2);
    }
}