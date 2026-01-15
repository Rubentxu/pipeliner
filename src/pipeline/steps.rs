//! Step types for pipeline execution
//!
//! This module defines step types that represent atomic units of work.

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// Types of steps available in pipelines
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepType {
    /// Shell command execution
    Shell {
        /// Command to execute
        command: String,
    },

    /// Echo message
    Echo {
        /// Message to output
        message: String,
    },

    /// Retry a step multiple times
    Retry {
        /// Number of retry attempts
        count: usize,
        /// Step to retry
        step: Box<Step>,
    },

    /// Timeout for a step
    Timeout {
        /// Maximum duration
        duration: Duration,
        /// Step to execute with timeout
        step: Box<Step>,
    },

    /// Stash files for later use
    Stash {
        /// Name of the stash
        name: String,
        /// File pattern to include
        includes: String,
    },

    /// Unstash previously stashed files
    Unstash {
        /// Name of the stash
        name: String,
    },

    /// Input prompt
    Input {
        /// Message to display
        message: String,
        /// Default value
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },

    /// Change directory
    Dir {
        /// Directory path
        path: String,
        /// Steps to execute in directory
        steps: Vec<Step>,
    },
}

impl StepType {
    /// Creates a shell command step
    pub fn shell(command: impl Into<String>) -> Self {
        Self::Shell {
            command: command.into(),
        }
    }

    /// Creates an echo step
    pub fn echo(message: impl Into<String>) -> Self {
        Self::Echo {
            message: message.into(),
        }
    }

    /// Creates a retry step
    pub fn retry(count: usize, step: Step) -> Self {
        Self::Retry {
            count,
            step: Box::new(step),
        }
    }

    /// Creates a timeout step
    pub fn timeout(duration: Duration, step: Step) -> Self {
        Self::Timeout {
            duration,
            step: Box::new(step),
        }
    }

    /// Creates a stash step
    pub fn stash(name: impl Into<String>, includes: impl Into<String>) -> Self {
        Self::Stash {
            name: name.into(),
            includes: includes.into(),
        }
    }

    /// Creates an unstash step
    pub fn unstash(name: impl Into<String>) -> Self {
        Self::Unstash { name: name.into() }
    }

    /// Creates an input step
    pub fn input(message: impl Into<String>) -> Self {
        Self::Input {
            message: message.into(),
            default: None,
        }
    }

    /// Creates an input step with default value
    pub fn input_with_default(message: impl Into<String>, default: impl Into<String>) -> Self {
        Self::Input {
            message: message.into(),
            default: Some(default.into()),
        }
    }

    /// Creates a directory change step
    pub fn dir(path: impl Into<String>, steps: Vec<Step>) -> Self {
        Self::Dir {
            path: path.into(),
            steps,
        }
    }
}

impl fmt::Display for StepType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shell { command } => write!(f, "sh({command})"),
            Self::Echo { message } => write!(f, "echo({message})"),
            Self::Retry { count, .. } => write!(f, "retry({count})"),
            Self::Timeout { duration, .. } => write!(f, "timeout({duration:?})"),
            Self::Stash { name, includes } => write!(f, "stash({name}, {includes})"),
            Self::Unstash { name } => write!(f, "unstash({name})"),
            Self::Input { message, .. } => write!(f, "input({message})"),
            Self::Dir { path, steps } => {
                write!(f, "dir({path}, {} steps)", steps.len())
            }
        }
    }
}

/// A single step in a pipeline
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// Type of step
    #[serde(flatten)]
    pub step_type: StepType,

    /// Optional name for the step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional timeout override for this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,
}

impl Step {
    /// Creates a new step
    pub fn new(step_type: StepType) -> Self {
        Self {
            step_type,
            name: None,
            timeout: None,
        }
    }

    /// Sets the name of the step
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the timeout for the step
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Creates a shell command step
    pub fn shell(command: impl Into<String>) -> Self {
        Self::new(StepType::shell(command))
    }

    /// Creates an echo step
    pub fn echo(message: impl Into<String>) -> Self {
        Self::new(StepType::echo(message))
    }

    /// Creates a retry step
    pub fn retry(count: usize, step: Self) -> Self {
        Self::new(StepType::retry(count, step))
    }

    /// Creates a timeout step
    pub fn timeout(duration: Duration, step: Self) -> Self {
        Self::new(StepType::timeout(duration, step))
    }

    /// Creates a stash step
    pub fn stash(name: impl Into<String>, includes: impl Into<String>) -> Self {
        Self::new(StepType::stash(name, includes))
    }

    /// Creates an unstash step
    pub fn unstash(name: impl Into<String>) -> Self {
        Self::new(StepType::unstash(name))
    }

    /// Creates an input step
    pub fn input(message: impl Into<String>) -> Self {
        Self::new(StepType::input(message))
    }

    /// Creates a directory change step
    pub fn dir(path: impl Into<String>, steps: Vec<Step>) -> Self {
        Self::new(StepType::dir(path, steps))
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "Step({}): {}", name, self.step_type),
            None => write!(f, "Step: {}", self.step_type),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_type_shell() {
        let step_type = StepType::shell("echo test");
        assert!(matches!(step_type, StepType::Shell { .. }));
        assert_eq!(step_type.to_string(), "sh(echo test)");
    }

    #[test]
    fn test_step_type_echo() {
        let step_type = StepType::echo("Hello");
        assert!(matches!(step_type, StepType::Echo { .. }));
        assert_eq!(step_type.to_string(), "echo(Hello)");
    }

    #[test]
    fn test_step_type_retry() {
        let inner = Step::shell("echo test");
        let step_type = StepType::retry(3, inner.clone());
        assert!(matches!(step_type, StepType::Retry { count: 3, .. }));
        assert_eq!(step_type.to_string(), "retry(3)");
    }

    #[test]
    fn test_step_type_timeout() {
        let inner = Step::shell("echo test");
        let duration = Duration::from_secs(60);
        let step_type = StepType::timeout(duration, inner.clone());
        assert!(matches!(step_type, StepType::Timeout { .. }));
        assert!(step_type.to_string().contains("60"));
    }

    #[test]
    fn test_step_creation() {
        let step = Step::shell("cargo build");
        assert!(matches!(step.step_type, StepType::Shell { .. }));
        assert!(step.name.is_none());
        assert!(step.timeout.is_none());
    }

    #[test]
    fn test_step_with_name() {
        let step = Step::shell("cargo build").with_name("Build");
        assert_eq!(step.name, Some("Build".to_string()));
        assert_eq!(step.to_string(), "Step(Build): sh(cargo build)");
    }

    #[test]
    fn test_step_with_timeout() {
        let step = Step::shell("cargo build").with_timeout(Duration::from_secs(120));
        assert_eq!(step.timeout, Some(Duration::from_secs(120)));
    }

    #[test]
    fn test_step_builder_methods() {
        let step = Step::shell("echo test");
        let retry = Step::retry(3, step.clone());
        let timeout = Step::timeout(Duration::from_secs(60), step);

        assert!(matches!(retry.step_type, StepType::Retry { .. }));
        assert!(matches!(timeout.step_type, StepType::Timeout { .. }));
    }

    #[test]
    fn test_step_stash_unstash() {
        let stash = Step::stash("my-stash", "*.rs");
        let unstash = Step::unstash("my-stash");

        assert!(matches!(stash.step_type, StepType::Stash { .. }));
        assert!(matches!(unstash.step_type, StepType::Unstash { .. }));
        assert_eq!(stash.to_string(), "Step: stash(my-stash, *.rs)");
    }

    #[test]
    fn test_input_step() {
        let input = Step::input("Continue?");
        assert!(matches!(input.step_type, StepType::Input { .. }));
    }

    #[test]
    fn test_dir_step_with_multiple_steps() {
        let steps = vec![Step::shell("cd subdir"), Step::shell("cargo build")];
        let dir_step = Step::dir("subdir", steps);
        assert!(
            matches!(dir_step.step_type, StepType::Dir { path, steps: s } if path == "subdir" && s.len() == 2)
        );
    }
}
