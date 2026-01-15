//! Shared library support for cross-pipeline code reuse
//!
//! This module provides mechanisms for sharing common pipeline logic
//! across multiple pipelines, similar to Jenkins Shared Libraries.

use crate::pipeline::{Step, StepType};
use std::collections::HashMap;
use std::sync::Arc;

/// A single step from a shared library
#[derive(Debug, Clone)]
pub struct LibraryStep {
    name: String,
    description: String,
    parameters: Vec<String>,
    step_type: StepType,
}

impl LibraryStep {
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>, step: Step) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: Vec::new(),
            step_type: step.step_type.clone(),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    #[must_use]
    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    #[must_use]
    pub fn step_type(&self) -> &StepType {
        &self.step_type
    }

    #[must_use]
    pub fn with_parameters(mut self, params: Vec<String>) -> Self {
        self.parameters = params;
        self
    }
}

#[derive(Debug, Clone)]
pub struct SharedLibrary {
    name: String,
    version: String,
    steps: HashMap<String, Arc<LibraryStep>>,
}

impl SharedLibrary {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: String::from("0.1.0"),
            steps: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn from_crates(_name: &str) -> Result<Self, SharedLibraryError> {
        Err(SharedLibraryError::NotYetImplemented(
            "Loading from crates not yet implemented".to_string(),
        ))
    }

    pub fn from_git(_url: &str, _version: &str) -> Result<Self, SharedLibraryError> {
        Err(SharedLibraryError::NotYetImplemented(
            "Loading from Git not yet implemented".to_string(),
        ))
    }

    #[must_use]
    pub fn register_step(mut self, step: LibraryStep) -> Self {
        self.steps.insert(step.name.clone(), Arc::new(step));
        self
    }

    #[must_use]
    pub fn get_step(&self, name: &str) -> Option<Arc<LibraryStep>> {
        self.steps.get(name).cloned()
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Errors that can occur when working with shared libraries
#[derive(Debug, thiserror::Error)]
pub enum SharedLibraryError {
    /// Feature not yet implemented
    #[error("Shared library not yet implemented: {0}")]
    NotYetImplemented(String),
    /// Step not found in library
    #[error("Step '{0}' not found in library '{1}'")]
    StepNotFound(String, String),
    /// Failed to load library from crates.io
    #[error("Failed to load library from crates: {0}")]
    CratesLoadError(String),
    /// Failed to load library from git repository
    #[error("Failed to load library from git: {0}")]
    GitLoadError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::Environment;
    use crate::pipeline::steps::Step;

    #[test]
    fn test_shared_library_creation() {
        let lib = SharedLibrary::new("my-lib");
        assert_eq!(lib.name(), "my-lib");
        assert_eq!(lib.version(), "0.1.0");
    }

    #[test]
    fn test_shared_library_with_version() {
        let lib = SharedLibrary::new("my-lib").with_version("1.2.3");
        assert_eq!(lib.version(), "1.2.3");
    }

    #[test]
    fn test_shared_library_step_registration() {
        let step = LibraryStep::new("test-step", "A test step", Step::shell("echo test"));
        let lib = SharedLibrary::new("my-lib").register_step(step);

        let retrieved = lib.get_step("test-step");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-step");
    }

    #[test]
    fn test_shared_library_get_nonexistent_step() {
        let lib = SharedLibrary::new("my-lib");
        let retrieved = lib.get_step("nonexistent");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_shared_library_from_crates_returns_error() {
        let result = SharedLibrary::from_crates("test-lib");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SharedLibraryError::NotYetImplemented(_)
        ));
    }

    #[test]
    fn test_shared_library_from_git_returns_error() {
        let result = SharedLibrary::from_git("https://github.com/test/repo", "main");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SharedLibraryError::NotYetImplemented(_)
        ));
    }

    #[test]
    fn test_library_step_with_parameters() {
        let step = LibraryStep::new("deploy", "Deploy step", Step::shell("echo deploy"))
            .with_parameters(vec!["env".to_string(), "version".to_string()]);
        assert_eq!(step.parameters.len(), 2);
        assert!(step.parameters.contains(&"env".to_string()));
    }
}
