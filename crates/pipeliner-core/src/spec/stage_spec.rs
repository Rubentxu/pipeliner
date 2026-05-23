//! Stage specification types for declarative pipeline definitions.
//!
//! This module defines the stage execution types, including sequential steps
//! and parallel stage execution.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::env_spec::EnvSpec;
use super::step_spec::StepSpec;
use super::pipeline_spec::PostSpec;

/// Creates a Duration from minutes.
///
/// # Arguments
///
/// * `n` - Number of minutes
///
/// # Example
///
/// ```
/// use pipeliner_core::spec::stage_spec::minutes;
/// use std::time::Duration;
///
/// let timeout = minutes(5);
/// assert_eq!(timeout, Duration::from_secs(300));
/// ```
#[must_use]
pub fn minutes(n: u64) -> Duration {
    Duration::from_secs(n * 60)
}

/// Creates a Duration from seconds.
///
/// # Arguments
///
/// * `n` - Number of seconds
///
/// # Example
///
/// ```
/// use pipeliner_core::spec::stage_spec::seconds;
/// use std::time::Duration;
///
/// let timeout = seconds(30);
/// assert_eq!(timeout, Duration::from_secs(30));
/// ```
#[must_use]
pub fn seconds(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// Specification for stage execution options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OptionsSpec {
    /// Timeout for stage execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,

    /// Number of retries on failure (default 0)
    #[serde(default)]
    pub retry: u32,

    /// Fail fast - stop all parallel stages when any stage fails (default true)
    #[serde(default)]
    pub fail_fast: bool,
}

impl OptionsSpec {
    /// Creates a new empty options specification.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the timeout for the options.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the retry count for the options.
    #[must_use]
    pub fn with_retry(mut self, retry: u32) -> Self {
        self.retry = retry;
        self
    }

    /// Sets the fail_fast option.
    #[must_use]
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }
}

/// A stage specification within a pipeline.
///
/// A stage has an ID for referencing, a display name for UI purposes,
/// and an execution model that defines how steps are run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    /// Unique identifier for the stage
    pub id: String,

    /// Human-readable display name
    pub display_name: String,

    /// Environment variables for this stage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<EnvSpec>,

    /// Execution options (timeout, retry, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OptionsSpec>,

    /// How this stage executes its steps
    pub execution: StageExecution,

    /// Post-actions for this stage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<PostSpec>,
}

/// Defines how a stage executes its content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StageExecution {
    /// Sequential steps execution
    Steps {
        /// Steps to execute in order
        steps: Vec<StepSpec>,
    },
    /// Parallel stages execution
    Parallel {
        /// Child stages to execute in parallel
        stages: Vec<StageSpec>,
    },
}

impl StageSpec {
    /// Creates a new stage specification with the given ID and display name.
    #[must_use]
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            env: None,
            options: None,
            execution: StageExecution::Steps { steps: Vec::new() },
            post: None,
        }
    }

    /// Sets the environment variables for this stage.
    #[must_use]
    pub fn with_env(mut self, env: EnvSpec) -> Self {
        self.env = Some(env);
        self
    }

    /// Sets the execution options for this stage.
    #[must_use]
    pub fn with_options(mut self, options: OptionsSpec) -> Self {
        self.options = Some(options);
        self
    }

    /// Sets the execution to sequential steps.
    #[must_use]
    pub fn with_steps(mut self, steps: Vec<StepSpec>) -> Self {
        self.execution = StageExecution::Steps { steps };
        self
    }

    /// Sets the execution to parallel stages.
    #[must_use]
    pub fn with_parallel_stages(mut self, stages: Vec<StageSpec>) -> Self {
        self.execution = StageExecution::Parallel { stages };
        self
    }

    /// Sets the post-actions for this stage.
    #[must_use]
    pub fn with_post(mut self, post: PostSpec) -> Self {
        self.post = Some(post);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::step_spec::{EchoStepSpec, ShellStepSpec, StepSpec};

    #[test]
    fn test_stage_spec_creation() {
        let stage = StageSpec::new("build", "Build Stage");
        assert_eq!(stage.id, "build");
        assert_eq!(stage.display_name, "Build Stage");
    }

    #[test]
    fn test_stage_spec_with_steps() {
        let stage = StageSpec::new("test", "Test Stage")
            .with_steps(vec![
                StepSpec::Echo(EchoStepSpec {
                    message: "Running tests".to_string(),
                }),
            ]);

        match stage.execution {
            StageExecution::Steps { steps } => {
                assert_eq!(steps.len(), 1);
            }
            StageExecution::Parallel { .. } => {
                panic!("Expected Steps execution");
            }
        }
    }

    #[test]
    fn test_stage_spec_with_parallel_stages() {
        let child_stage = StageSpec::new("child1", "Child 1");
        let stage = StageSpec::new("parent", "Parent Stage")
            .with_parallel_stages(vec![child_stage]);

        match stage.execution {
            StageExecution::Parallel { stages } => {
                assert_eq!(stages.len(), 1);
            }
            StageExecution::Steps { .. } => {
                panic!("Expected Parallel execution");
            }
        }
    }

    #[test]
    fn test_stage_execution_serialization() {
        let steps = vec![StepSpec::Shell(ShellStepSpec::new("echo hello"))];
        let execution = StageExecution::Steps { steps };

        let json = serde_json::to_string(&execution).unwrap();
        assert!(json.contains("\"type\":\"steps\""));
    }

    #[test]
    fn test_stage_spec_json_roundtrip() {
        let original = StageSpec::new("build", "Build")
            .with_steps(vec![
                StepSpec::Echo(EchoStepSpec {
                    message: "Building...".to_string(),
                }),
            ]);

        let json = serde_json::to_string(&original).unwrap();
        let parsed: StageSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, original.id);
        assert_eq!(parsed.display_name, original.display_name);
    }

    #[test]
    fn test_minutes_helper() {
        let duration = minutes(5);
        assert_eq!(duration, std::time::Duration::from_secs(300));
    }

    #[test]
    fn test_seconds_helper() {
        let duration = seconds(30);
        assert_eq!(duration, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_options_spec_default() {
        let options = OptionsSpec::new();
        assert!(options.timeout.is_none());
        assert_eq!(options.retry, 0);
    }

    #[test]
    fn test_options_spec_with_timeout() {
        let options = OptionsSpec::new()
            .with_timeout(std::time::Duration::from_secs(300))
            .with_retry(3);

        assert!(options.timeout.is_some());
        assert_eq!(options.retry, 3);
    }

    #[test]
    fn test_options_spec_serialization() {
        let options = OptionsSpec::new()
            .with_timeout(minutes(5))
            .with_retry(2);

        let json = serde_json::to_string(&options).unwrap();
        let parsed: OptionsSpec = serde_json::from_str(&json).unwrap();

        assert!(parsed.timeout.is_some());
        assert_eq!(parsed.retry, 2);
    }

    #[test]
    fn test_stage_spec_with_env() {
        let env = EnvSpec::new()
            .with_var("FOO", "bar")
            .with_var("BAZ", "qux");

        let stage = StageSpec::new("build", "Build")
            .with_env(env);

        assert!(stage.env.is_some());
        let env = stage.env.unwrap();
        assert_eq!(env.get("FOO"), Some("bar"));
        assert_eq!(env.get("BAZ"), Some("qux"));
    }

    #[test]
    fn test_stage_spec_with_options() {
        let options = OptionsSpec::new()
            .with_timeout(minutes(5))
            .with_retry(3);

        let stage = StageSpec::new("build", "Build")
            .with_options(options);

        assert!(stage.options.is_some());
        let opts = stage.options.unwrap();
        assert!(opts.timeout.is_some());
        assert_eq!(opts.retry, 3);
    }

    #[test]
    fn test_stage_spec_env_json_roundtrip() {
        let env = EnvSpec::new().with_var("KEY", "value");
        let stage = StageSpec::new("build", "Build")
            .with_env(env);

        let json = serde_json::to_string(&stage).unwrap();
        let parsed: StageSpec = serde_json::from_str(&json).unwrap();

        assert!(parsed.env.is_some());
        assert_eq!(parsed.env.unwrap().get("KEY"), Some("value"));
    }

    #[test]
    fn test_stage_spec_options_json_roundtrip() {
        let options = OptionsSpec::new()
            .with_timeout(seconds(60))
            .with_retry(2);

        let stage = StageSpec::new("build", "Build")
            .with_options(options);

        let json = serde_json::to_string(&stage).unwrap();
        let parsed: StageSpec = serde_json::from_str(&json).unwrap();

        assert!(parsed.options.is_some());
        assert_eq!(parsed.options.unwrap().retry, 2);
    }
}
