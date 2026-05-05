//! # Pipeliner Library System
//!
//! Runtime library loading from git repos, local paths, or local library dirs.
//!
//! This crate provides the extensibility mechanism for Pipeliner pipelines,
//! allowing custom steps and resources to be loaded from external sources.
//!
//! ## Architecture
//!
//! - `SourceRetriever` trait: Pluggable strategies for retrieving library artifacts
//! - `GitSource`: Retrieves libraries from git repositories
//! - `LocalSource`: Retrieves libraries from local filesystem paths
//! - `LocalLib`: Retrieves Rust source files recursively from local directories
//! - `LibraryArtifacts`: Discovered files from a library source
//! - `LibraryLoader`: Orchestrates retrieval, caching, and step registration

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

pub mod artifacts;
pub mod error;
pub mod loader;
pub mod retriever;
pub mod retrievers;

// Re-exports for common use
pub use artifacts::LibraryArtifacts;
pub use error::LibraryError;
pub use loader::LibraryLoader;
pub use retriever::SourceRetriever;
pub use retrievers::{GitSource, LocalLib, LocalSource};
