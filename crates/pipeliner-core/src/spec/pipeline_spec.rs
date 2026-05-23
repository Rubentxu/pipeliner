//! Pipeline specification types for declarative pipeline definitions.
//!
//! This module defines the top-level pipeline specification structure,
//! including schema versioning and post-actions.

use serde::{Deserialize, Serialize};

use super::env_spec::EnvSpec;
use super::stage_spec::StageSpec;
use super::step_spec::StepSpec;

/// The top-level pipeline specification.
///
/// This struct represents a complete pipeline definition that can be
/// serialized to/from JSON for storage and exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    /// Schema version identifier (e.g., "pipeliner.pipeline.v1")
    pub schema_version: String,

    /// Pipeliner version this spec targets (e.g., "0.1.0")
    pub pipeliner_version: String,

    /// Environment variables for the pipeline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<EnvSpec>,

    /// Stages in this pipeline
    pub stages: Vec<StageSpec>,

    /// Pipeline-level post-actions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<PostSpec>,
}

/// Post-actions that can run after pipeline or stage execution.
///
/// Post-actions are organized by the execution outcome:
/// - `always`: Runs regardless of outcome
/// - `success`: Runs only when the pipeline/stage succeeded
/// - `failure`: Runs only when the pipeline/stage failed
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostSpec {
    /// Steps to run always (regardless of success/failure)
    #[serde(default)]
    pub always: Vec<StepSpec>,

    /// Steps to run on success
    #[serde(default)]
    pub success: Vec<StepSpec>,

    /// Steps to run on failure
    #[serde(default)]
    pub failure: Vec<StepSpec>,
}

impl PipelineSpec {
    /// Creates a new pipeline specification.
    ///
    /// # Arguments
    ///
    /// * `schema_version` - The schema version (e.g., "pipeliner.pipeline.v1")
    /// * `pipeliner_version` - The target pipeliner version (e.g., "0.1.0")
    ///
    /// # Example
    ///
    /// ```
    /// use pipeliner_core::spec::{PipelineSpec, StageSpec};
    ///
    /// let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0");
    /// ```
    #[must_use]
    pub fn new(schema_version: &str, pipeliner_version: &str) -> Self {
        Self {
            schema_version: schema_version.to_string(),
            pipeliner_version: pipeliner_version.to_string(),
            env: None,
            stages: Vec::new(),
            post: None,
        }
    }

    /// Sets the environment variables for this pipeline.
    #[must_use]
    pub fn with_env(mut self, env: EnvSpec) -> Self {
        self.env = Some(env);
        self
    }

    /// Adds a stage to the pipeline.
    #[must_use]
    pub fn with_stage(mut self, stage: StageSpec) -> Self {
        self.stages.push(stage);
        self
    }

    /// Sets the pipeline-level post-actions.
    #[must_use]
    pub fn with_post(mut self, post: PostSpec) -> Self {
        self.post = Some(post);
        self
    }

    /// Serializes this pipeline spec to a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserializes a pipeline spec from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl PostSpec {
    /// Creates a new empty post specification.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds steps to run always.
    #[must_use]
    pub fn with_always_steps(mut self, steps: Vec<StepSpec>) -> Self {
        self.always = steps;
        self
    }

    /// Adds steps to run on success.
    #[must_use]
    pub fn with_success_steps(mut self, steps: Vec<StepSpec>) -> Self {
        self.success = steps;
        self
    }

    /// Adds steps to run on failure.
    #[must_use]
    pub fn with_failure_steps(mut self, steps: Vec<StepSpec>) -> Self {
        self.failure = steps;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::stage_spec::{StageExecution, StageSpec};
    use crate::spec::step_spec::{EchoStepSpec, StepSpec};

    #[test]
    fn test_pipeline_spec_creation() {
        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0");
        assert_eq!(spec.schema_version, "pipeliner.pipeline.v1");
        assert_eq!(spec.pipeliner_version, "0.1.0");
        assert!(spec.env.is_none());
        assert!(spec.stages.is_empty());
        assert!(spec.post.is_none());
    }

    #[test]
    fn test_pipeline_spec_with_stages() {
        let stage = StageSpec::new("build", "Build");
        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
            .with_stage(stage);

        assert_eq!(spec.stages.len(), 1);
    }

    #[test]
    fn test_pipeline_spec_to_json() {
        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0");
        let json = spec.to_json().unwrap();

        assert!(json.contains("pipeliner.pipeline.v1"));
        assert!(json.contains("0.1.0"));
    }

    #[test]
    fn test_pipeline_spec_from_json() {
        let json = r#"{
            "schema_version": "pipeliner.pipeline.v1",
            "pipeliner_version": "0.1.0",
            "stages": []
        }"#;

        let spec = PipelineSpec::from_json(json).unwrap();
        assert_eq!(spec.schema_version, "pipeliner.pipeline.v1");
        assert_eq!(spec.pipeliner_version, "0.1.0");
    }

    #[test]
    fn test_pipeline_spec_json_roundtrip() {
        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
            .with_stage(
                StageSpec::new("build", "Build")
                    .with_steps(vec![
                        StepSpec::Echo(EchoStepSpec {
                            message: "hello".to_string(),
                        }),
                    ]),
            );

        let json = serde_json::to_string(&spec).unwrap();
        let parsed: PipelineSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.schema_version, spec.schema_version);
        assert_eq!(parsed.pipeliner_version, spec.pipeliner_version);
        assert_eq!(parsed.stages.len(), spec.stages.len());
    }

    #[test]
    fn test_post_spec_creation() {
        let post = PostSpec::new();
        assert!(post.always.is_empty());
        assert!(post.success.is_empty());
        assert!(post.failure.is_empty());
    }

    #[test]
    fn test_post_spec_with_steps() {
        let always_step = StepSpec::Echo(EchoStepSpec {
            message: "always".to_string(),
        });
        let success_step = StepSpec::Echo(EchoStepSpec {
            message: "success".to_string(),
        });

        let post = PostSpec::new()
            .with_always_steps(vec![always_step])
            .with_success_steps(vec![success_step]);

        assert_eq!(post.always.len(), 1);
        assert_eq!(post.success.len(), 1);
        assert!(post.failure.is_empty());
    }

    #[test]
    fn test_pipeline_spec_with_post() {
        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
            .with_post(PostSpec::new());

        assert!(spec.post.is_some());
    }

    #[test]
    fn test_pipeline_spec_with_nested_stages() {
        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
            .with_stage(
                StageSpec::new("build", "Build")
                    .with_steps(vec![
                        StepSpec::Echo(EchoStepSpec {
                            message: "building".to_string(),
                        }),
                    ]),
            )
            .with_stage(
                StageSpec::new("test", "Test")
                    .with_steps(vec![
                        StepSpec::Echo(EchoStepSpec {
                            message: "testing".to_string(),
                        }),
                    ]),
            );

        assert_eq!(spec.stages.len(), 2);
    }

    #[test]
    fn test_pipeline_spec_with_env() {
        use crate::spec::EnvSpec;

        let env = EnvSpec::new()
            .with_var("RUST_BACKTRACE", "1")
            .with_var("LOG_LEVEL", "debug");

        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
            .with_env(env);

        assert!(spec.env.is_some());
        assert_eq!(spec.env.as_ref().unwrap().get("RUST_BACKTRACE"), Some("1"));
        assert_eq!(spec.env.as_ref().unwrap().get("LOG_LEVEL"), Some("debug"));
    }

    #[test]
    fn test_pipeline_spec_env_json_roundtrip() {
        use crate::spec::EnvSpec;

        let env = EnvSpec::new().with_var("FOO", "bar");
        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
            .with_env(env);

        let json = serde_json::to_string(&spec).unwrap();
        let parsed: PipelineSpec = serde_json::from_str(&json).unwrap();

        assert!(parsed.env.is_some());
        assert_eq!(parsed.env.unwrap().get("FOO"), Some("bar"));
    }
}
