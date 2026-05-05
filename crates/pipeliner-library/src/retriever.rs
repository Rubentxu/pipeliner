//! Source retriever trait and registry for library loading.

use async_trait::async_trait;
use pipeliner_core::config::{LibraryConfig, RetrieverType};

use crate::artifacts::LibraryArtifacts;
use crate::error::LibraryError;

/// Trait for retrieving library artifacts from a source.
///
/// Implementors of this trait provide the logic for fetching library
/// content from different sources (git, local filesystem, etc.).
#[async_trait]
pub trait SourceRetriever: Send + Sync {
    /// Retrieve library artifacts from the configured source.
    ///
    /// # Arguments
    ///
    /// * `config` - The library configuration specifying source location and type
    ///
    /// # Returns
    ///
    /// Returns `Ok(LibraryArtifacts)` on success, or `Err(LibraryError)` on failure
    async fn retrieve(&self, config: &LibraryConfig) -> Result<LibraryArtifacts, LibraryError>;

    /// Returns the type of retriever this implementor handles.
    fn retriever_type(&self) -> RetrieverType;
}

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::config::RetrieverType;
    use std::sync::Arc;
    use async_trait::async_trait;

    // ===================================================================
    // A4: SourceRetriever Trait Definition Tests (RED → GREEN)
    // ===================================================================

    /// A mock retriever for testing
    struct MockRetriever {
        retriever_type: RetrieverType,
        artifacts: LibraryArtifacts,
    }

    impl MockRetriever {
        fn new(retriever_type: RetrieverType, artifacts: LibraryArtifacts) -> Self {
            Self { retriever_type, artifacts }
        }
    }

    #[async_trait]
    impl SourceRetriever for MockRetriever {
        async fn retrieve(&self, _config: &LibraryConfig) -> Result<LibraryArtifacts, LibraryError> {
            Ok(self.artifacts.clone())
        }

        fn retriever_type(&self) -> RetrieverType {
            self.retriever_type.clone()
        }
    }

    #[tokio::test]
    async fn test_mock_retriever_retrieve() {
        // A5: Mock retriever returns configured artifacts
        let artifacts = LibraryArtifacts::new();
        let retriever = MockRetriever::new(RetrieverType::GitSource, artifacts);

        let config = LibraryConfig {
            name: "test-lib".to_string(),
            source_path: "https://github.com/example/lib".to_string(),
            retriever_type: RetrieverType::GitSource,
            default_version: Some("main".to_string()),
            modules: vec![],
        };

        let result = retriever.retrieve(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_retriever_retriever_type() {
        // A5: retriever_type() returns correct type
        let artifacts = LibraryArtifacts::new();
        
        let git_retriever = MockRetriever::new(RetrieverType::GitSource, artifacts.clone());
        assert_eq!(git_retriever.retriever_type(), RetrieverType::GitSource);

        let local_retriever = MockRetriever::new(RetrieverType::LocalSource, artifacts.clone());
        assert_eq!(local_retriever.retriever_type(), RetrieverType::LocalSource);

        let local_lib_retriever = MockRetriever::new(RetrieverType::LocalLib, artifacts);
        assert_eq!(local_lib_retriever.retriever_type(), RetrieverType::LocalLib);
    }

    #[test]
    fn test_source_retriever_is_object_safe() {
        // A4: SourceRetriever is Send + Sync (object safety)
        fn _assert_send_sync<T: Send + Sync>() {}
        fn _assert_source_retriever_bounds<T: SourceRetriever>() {
            _assert_send_sync::<T>();
        }
    }

    #[tokio::test]
    async fn test_source_retriever_in_arc() {
        // A5: SourceRetriever can be stored in Arc<dyn SourceRetriever>
        let artifacts = LibraryArtifacts::new();
        let retriever: Arc<dyn SourceRetriever> = Arc::new(MockRetriever::new(
            RetrieverType::GitSource,
            artifacts,
        ));

        let config = LibraryConfig {
            name: "test".to_string(),
            source_path: "https://github.com/example/test".to_string(),
            retriever_type: RetrieverType::GitSource,
            default_version: None,
            modules: vec![],
        };

        let result = retriever.retrieve(&config).await;
        assert!(result.is_ok());
    }
}
