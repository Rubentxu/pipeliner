//! Core types for pipeline domain
//!
//! This module contains fundamental types that represent
//! structure of CI/CD pipelines.

#![allow(clippy::must_use_candidate)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// Result type for pipeline execution
pub type PipelineResult = std::result::Result<StageResult, super::errors::PipelineError>;

/// Possible outcomes of a pipeline or stage execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageResult {
    /// Execution completed successfully
    Success,
    /// Execution failed
    Failure,
    /// Execution completed with unstable state
    Unstable,
    /// Execution was skipped
    Skipped,
}

impl StageResult {
    /// Returns true if result is successful
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Returns true if result is a failure
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure)
    }

    /// Returns true if result is unstable
    #[must_use]
    pub fn is_unstable(&self) -> bool {
        matches!(self, Self::Unstable)
    }

    /// Returns true if result is skipped
    #[must_use]
    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped)
    }
}

impl fmt::Display for StageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "SUCCESS"),
            Self::Failure => write!(f, "FAILURE"),
            Self::Unstable => write!(f, "UNSTABLE"),
            Self::Skipped => write!(f, "SKIPPED"),
        }
    }
}

/// Trait for types that can be validated
#[allow(clippy::missing_errors_doc)]
pub trait Validate {
    /// Type of validation error
    type Error;

    /// Validates this type
    fn validate(&self) -> std::result::Result<(), Self::Error>;
}
