//! Pipeline validation types and rules.
//!
//! This module provides the validation framework for pipelines,
//! including error types and validation trait implementations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Validation result type
pub type ValidationResult<T = ()> = Result<T, ValidationError>;

/// Validation error types
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ValidationError {
    /// Pipeline has no stages
    #[error("pipeline must have at least one stage")]
    EmptyStages,

    /// Stage has no name
    #[error("stage name cannot be empty")]
    EmptyName,

    /// Stage has no steps
    #[error("stage '{stage}' must have at least one step")]
    EmptySteps {
        /// Stage name
        stage: String,
    },

    /// Invalid agent configuration
    #[error("invalid agent configuration: {reason}")]
    InvalidAgent {
        /// Reason for validation failure
        reason: String,
    },

    /// Invalid environment variable
    #[error("invalid environment variable '{name}': {reason}")]
    InvalidEnvironment {
        /// Variable name
        name: String,
        /// Reason for validation failure
        reason: String,
    },

    /// Invalid parameter
    #[error("invalid parameter '{name}': {reason}")]
    InvalidParameter {
        /// Parameter name
        name: String,
        /// Reason for validation failure
        reason: String,
    },

    /// Invalid matrix configuration
    #[error("invalid matrix configuration: {reason}")]
    InvalidMatrix {
        /// Reason for validation failure
        reason: String,
    },

    /// Circular dependency detected
    #[error("circular dependency detected involving '{stage}'")]
    CircularDependency {
        /// Stage involved in circular dependency
        stage: String,
    },

    /// Duplicate stage name
    #[error("duplicate stage name '{name}'")]
    DuplicateStage {
        /// Duplicate stage name
        name: String,
    },

    /// Invalid step configuration
    #[error("invalid step configuration: {reason}")]
    InvalidStep {
        /// Reason for validation failure
        reason: String,
    },

    /// Missing required option
    #[error("missing required option: {option}")]
    MissingOption {
        /// Missing option name
        option: String,
    },

    /// Invalid timeout configuration
    #[error("invalid timeout configuration: {reason}")]
    InvalidTimeout {
        /// Reason for validation failure
        reason: String,
    },

    /// Invalid retry configuration
    #[error("invalid retry configuration: {reason}")]
    InvalidRetry {
        /// Reason for validation failure
        reason: String,
    },

    /// Validation error with path context
    #[error("validation error at {path}: {error}")]
    WithPath {
        /// Path to the error location
        path: String,
        /// Underlying error
        error: Box<ValidationError>,
    },
}

/// Trait for validatable types
pub trait Validate {
    /// The error type returned by validation
    type Error;

    /// Validates this instance
    fn validate(&self) -> Result<(), Self::Error>;
}

impl<T, E> Validate for Result<T, E>
where
    T: Validate,
    E: Clone,
{
    type Error = T::Error;

    fn validate(&self) -> Result<(), Self::Error> {
        match self {
            Ok(value) => value.validate(),
            Err(_) => Ok(()),
        }
    }
}

impl<T: Validate> Validate for Option<T> {
    type Error = T::Error;

    fn validate(&self) -> Result<(), Self::Error> {
        match self {
            Some(value) => value.validate(),
            None => Ok(()),
        }
    }
}

impl<T: Validate> Validate for Vec<T> {
    type Error = T::Error;

    fn validate(&self) -> Result<(), Self::Error> {
        for item in self {
            item.validate()?;
        }
        Ok(())
    }
}

/// Validation context for complex validations
#[derive(Debug, Default)]
pub struct ValidationContext {
    /// Current path in the structure
    path: Vec<String>,
}

impl ValidationContext {
    /// Creates a new validation context
    #[must_use]
    pub fn new() -> Self {
        Self { path: Vec::new() }
    }

    /// Pushes a path component
    pub fn push(&mut self, component: impl Into<String>) {
        self.path.push(component.into());
    }

    /// Pops a path component
    pub fn pop(&mut self) {
        self.path.pop();
    }

    /// Gets the current path as a string
    #[must_use]
    pub fn path(&self) -> String {
        self.path.join(".")
    }

    /// Wraps an error with the current path
    pub fn wrap<E>(&self, error: E) -> ValidationError
    where
        E: Into<ValidationError>,
    {
        if self.path.is_empty() {
            error.into()
        } else {
            ValidationError::WithPath {
                path: self.path(),
                error: Box::new(error.into()),
            }
        }
    }
}

/// Validation rules for common patterns
pub mod rules {
    use super::*;

    /// Validates that a name is not empty
    pub fn validate_name(name: &str, _field: &str) -> Result<(), ValidationError> {
        if name.trim().is_empty() {
            Err(ValidationError::EmptyName)
        } else {
            Ok(())
        }
    }

    /// Validates that a timeout is within acceptable bounds
    pub fn validate_timeout(seconds: u64) -> Result<(), ValidationError> {
        if seconds == 0 {
            return Err(ValidationError::InvalidTimeout {
                reason: "timeout must be greater than zero".to_string(),
            });
        }
        if seconds > 86400 * 7 {
            return Err(ValidationError::InvalidTimeout {
                reason: "timeout exceeds maximum of 7 days".to_string(),
            });
        }
        Ok(())
    }

    /// Validates that a retry count is within acceptable bounds
    pub fn validate_retry(count: usize) -> Result<(), ValidationError> {
        if count > 10 {
            return Err(ValidationError::InvalidRetry {
                reason: "retry count exceeds maximum of 10".to_string(),
            });
        }
        Ok(())
    }

    /// Validates that a stage name is unique within a list
    pub fn validate_unique_stages(stages: &[String]) -> Result<(), ValidationError> {
        let mut seen = std::collections::HashSet::new();
        for stage in stages {
            if !seen.insert(stage) {
                return Err(ValidationError::DuplicateStage {
                    name: stage.clone(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_stages_error() {
        let error = ValidationError::EmptyStages;
        assert_eq!(error.to_string(), "pipeline must have at least one stage");
    }

    #[test]
    fn test_empty_steps_error() {
        let error = ValidationError::EmptySteps {
            stage: "Build".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "stage 'Build' must have at least one step"
        );
    }

    #[test]
    fn test_validation_context() {
        let ctx = ValidationContext::new();
        assert_eq!(ctx.path(), "");
    }

    #[test]
    fn test_validation_context_with_path() {
        let mut ctx = ValidationContext::new();
        ctx.push("stages");
        ctx.push("0");
        assert_eq!(ctx.path(), "stages.0");
    }

    #[test]
    fn test_validate_timeout_valid() {
        assert!(rules::validate_timeout(3600).is_ok());
    }

    #[test]
    fn test_validate_timeout_zero() {
        assert!(rules::validate_timeout(0).is_err());
    }

    #[test]
    fn test_validate_retry_valid() {
        assert!(rules::validate_retry(3).is_ok());
    }

    #[test]
    fn test_validate_retry_too_high() {
        assert!(rules::validate_retry(15).is_err());
    }
}
