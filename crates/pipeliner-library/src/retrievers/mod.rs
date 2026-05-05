//! Source retrievers for different library types.

pub mod git_source;
pub mod local_lib;
pub mod local_source;

// Re-exports for convenience
pub use git_source::GitSource;
pub use local_lib::LocalLib;
pub use local_source::LocalSource;
