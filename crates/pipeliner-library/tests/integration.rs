//! Integration tests for the library system.
//!
//! These tests verify the full flow of loading libraries from various sources.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Tests the full flow: create temp library → LocalSource → LibraryLoader → load
#[tokio::test]
async fn test_full_flow_temp_library_local_source() {
    use pipeliner_library::{LibraryLoader, LocalSource, LibraryArtifacts};
    use pipeliner_core::config::{LibraryConfig, RetrieverType};

    // Create a temp directory with library structure
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let base = temp_dir.path();

    // Create library structure: src/, steps/, resources/
    fs::create_dir_all(base.join("src")).expect("Should create src dir");
    fs::create_dir_all(base.join("steps")).expect("Should create steps dir");
    fs::create_dir_all(base.join("resources")).expect("Should create resources dir");

    // Create some files
    fs::write(base.join("src/lib.rs"), "pub mod foo;").expect("Should create src/lib.rs");
    fs::write(base.join("src/main.rs"), "fn main() {}").expect("Should create src/main.rs");
    fs::write(base.join("steps/deploy.yaml"), "name: deploy").expect("Should create steps/deploy.yaml");
    fs::write(base.join("resources/config.json"), "{}").expect("Should create resources/config.json");

    // Create LibraryLoader with LocalSource using base as the search path
    let mut loader = LibraryLoader::new();
    loader.register_retriever(
        RetrieverType::LocalSource,
        Box::new(LocalSource::new(base.to_path_buf())),
    );

    // Create config with empty source_path to use base directly
    let config = LibraryConfig {
        name: "test-lib".to_string(),
        source_path: ".".to_string(), // Use base directly
        retriever_type: RetrieverType::LocalSource,
        default_version: Some("1.0.0".to_string()),
        modules: vec![],
    };

    // Load the library
    let result = loader.load(&config).await;
    assert!(result.is_ok(), "Should load library successfully: {:?}", result.err());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.source_files.len(), 2, "Should have 2 source files");
    assert_eq!(artifacts.step_files.len(), 1, "Should have 1 step file");
    assert_eq!(artifacts.resource_files.len(), 1, "Should have 1 resource file");

    // Verify is_loaded works
    assert!(loader.is_loaded("test-lib", "1.0.0"));
    assert!(!loader.is_loaded("test-lib", "2.0.0")); // Different version

    // Verify cache
    assert_eq!(loader.cache_size(), 1);
    assert_eq!(loader.loaded_libraries(), vec!["test-lib@1.0.0"]);
}

/// Tests that loading the same library twice returns cached results (dedup)
#[tokio::test]
async fn test_full_flow_deduplication() {
    use pipeliner_library::{LibraryLoader, LocalSource};
    use pipeliner_core::config::{LibraryConfig, RetrieverType};

    // Create a temp directory with library structure
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let base = temp_dir.path();

    fs::create_dir_all(base.join("src")).expect("Should create src dir");
    fs::write(base.join("src/lib.rs"), "pub mod foo;").expect("Should create src/lib.rs");

    // Create LibraryLoader with LocalSource
    let mut loader = LibraryLoader::new();
    loader.register_retriever(
        RetrieverType::LocalSource,
        Box::new(LocalSource::new(base.to_path_buf())),
    );

    let config = LibraryConfig {
        name: "dedup-lib".to_string(),
        source_path: ".".to_string(),
        retriever_type: RetrieverType::LocalSource,
        default_version: Some("1.0.0".to_string()),
        modules: vec![],
    };

    // Load twice
    let result1 = loader.load(&config).await;
    let result2 = loader.load(&config).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    // Should be deduplicated
    assert_eq!(loader.cache_size(), 1);
    assert_eq!(loader.loaded_libraries().len(), 1);
}

/// Tests clearing the cache
#[tokio::test]
async fn test_full_flow_clear_cache() {
    use pipeliner_library::{LibraryLoader, LocalSource};
    use pipeliner_core::config::{LibraryConfig, RetrieverType};

    let temp_dir = TempDir::new().expect("Should create temp dir");
    let base = temp_dir.path();

    fs::create_dir_all(base.join("src")).expect("Should create src dir");
    fs::write(base.join("src/lib.rs"), "pub mod foo;").expect("Should create src/lib.rs");

    let mut loader = LibraryLoader::new();
    loader.register_retriever(
        RetrieverType::LocalSource,
        Box::new(LocalSource::new(base.to_path_buf())),
    );

    let config = LibraryConfig {
        name: "cache-test".to_string(),
        source_path: ".".to_string(),
        retriever_type: RetrieverType::LocalSource,
        default_version: Some("1.0.0".to_string()),
        modules: vec![],
    };

    // Load library
    loader.load(&config).await.expect("Should load");
    assert_eq!(loader.cache_size(), 1);

    // Clear cache
    loader.clear_cache();
    assert_eq!(loader.cache_size(), 0);
    assert!(loader.loaded_libraries().is_empty());
}

/// Tests multiple libraries with different retrievers
#[tokio::test]
async fn test_full_flow_multiple_libraries() {
    use pipeliner_library::{LibraryLoader, LocalSource, LocalLib};
    use pipeliner_core::config::{LibraryConfig, RetrieverType};

    // Create first temp dir for LocalSource
    let temp_dir1 = TempDir::new().expect("Should create temp dir");
    let base1 = temp_dir1.path();
    fs::create_dir_all(base1.join("src")).expect("Should create src dir");
    fs::write(base1.join("src/lib.rs"), "pub mod lib1;").expect("Should create lib1.rs");

    // Create second temp dir for LocalLib
    let temp_dir2 = TempDir::new().expect("Should create temp dir");
    let base2 = temp_dir2.path();
    fs::create_dir_all(base2.join("mylib")).expect("Should create mylib dir");
    fs::write(base2.join("mylib/step.rs"), "pub struct MyStep;").expect("Should create step.rs");

    // Create LibraryLoader with both retriever types
    let mut loader = LibraryLoader::new();
    loader.register_retriever(RetrieverType::LocalSource, Box::new(LocalSource::new(base1.to_path_buf())));
    loader.register_retriever(RetrieverType::LocalLib, Box::new(LocalLib::new(base2.to_path_buf())));

    // Load first library (LocalSource)
    let config1 = LibraryConfig {
        name: "lib1".to_string(),
        source_path: ".".to_string(),
        retriever_type: RetrieverType::LocalSource,
        default_version: Some("1.0.0".to_string()),
        modules: vec![],
    };

    // Load second library (LocalLib)
    let config2 = LibraryConfig {
        name: "lib2".to_string(),
        source_path: "mylib".to_string(), // Relative path under base2
        retriever_type: RetrieverType::LocalLib,
        default_version: Some("2.0.0".to_string()),
        modules: vec![],
    };

    let result1 = loader.load(&config1).await;
    let result2 = loader.load(&config2).await;

    assert!(result1.is_ok(), "lib1 should load: {:?}", result1.err());
    assert!(result2.is_ok(), "lib2 should load: {:?}", result2.err());
    assert_eq!(loader.cache_size(), 2);
    assert_eq!(loader.loaded_libraries().len(), 2);
}
