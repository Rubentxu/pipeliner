//! Pipeline definition and builder

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

use crate::pipeline::agent::AgentType;
use crate::pipeline::errors::ValidationError;
use crate::pipeline::options::{PipelineOptions, Trigger};
use crate::pipeline::post::PostCondition;
use crate::pipeline::stage::Stage;
use crate::pipeline::types::Validate;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Main pipeline structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pipeline {
    /// Pipeline name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Agent for pipeline execution
    pub agent: AgentType,

    /// Stages in pipeline
    pub stages: Vec<Stage>,

    /// Environment variables
    #[serde(default)]
    pub environment: crate::pipeline::Environment,

    /// Pipeline parameters
    #[serde(default)]
    pub parameters: crate::pipeline::Parameters,

    /// Pipeline triggers
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub triggers: Vec<Trigger>,

    /// Pipeline options
    #[serde(default)]
    pub options: PipelineOptions,

    /// Post-conditions for pipeline
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub post: Vec<PostCondition>,
}

impl Validate for Pipeline {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        // Validate agent
        self.agent.validate()?;

        // Validate stages
        if self.stages.is_empty() {
            return Err(ValidationError::EmptyPipeline);
        }

        for stage in &self.stages {
            stage.validate()?;
        }

        // Validate triggers
        for trigger in &self.triggers {
            trigger.validate()?;
        }

        // Validate options
        self.options.validate()?;

        Ok(())
    }
}

impl Pipeline {
    /// Creates a new pipeline builder
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder::new()
    }

    /// Returns pipeline name
    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    /// Returns number of stages
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }
}

impl fmt::Display for Pipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Pipeline({}): {} stages",
            self.name.as_deref().unwrap_or("unnamed"),
            self.stages.len()
        )
    }
}

/// Builder for creating pipelines
#[derive(Debug, Clone)]
pub struct PipelineBuilder {
    pipeline: Pipeline,
}

impl PipelineBuilder {
    /// Creates a new pipeline builder
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline {
                name: None,
                agent: AgentType::Any,
                stages: Vec::new(),
                environment: crate::pipeline::Environment::new(),
                parameters: crate::pipeline::Parameters::new(),
                triggers: Vec::new(),
                options: PipelineOptions::default(),
                post: Vec::new(),
            },
        }
    }

    /// Sets pipeline name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.pipeline.name = Some(name.into());
        self
    }

    /// Sets agent for pipeline
    pub fn agent(mut self, agent: AgentType) -> Self {
        self.pipeline.agent = agent;
        self
    }

    /// Adds a stage to pipeline
    pub fn stage(mut self, stage: Stage) -> Self {
        self.pipeline.stages.push(stage);
        self
    }

    /// Adds multiple stages to pipeline
    pub fn stages(mut self, mut stages: Vec<Stage>) -> Self {
        self.pipeline.stages.append(&mut stages);
        self
    }

    /// Configures environment with a closure or directly
    pub fn environment<F>(mut self, f: F) -> Self
    where
        F: FnOnce(crate::pipeline::Environment) -> crate::pipeline::Environment,
    {
        self.pipeline.environment = f(self.pipeline.environment);
        self
    }

    /// Sets environment directly (convenience method)
    #[must_use]
    pub fn with_environment(mut self, environment: crate::pipeline::Environment) -> Self {
        self.pipeline.environment = environment;
        self
    }

    /// Configures parameters with a closure or directly
    pub fn parameters<F>(mut self, f: F) -> Self
    where
        F: FnOnce(crate::pipeline::Parameters) -> crate::pipeline::Parameters,
    {
        self.pipeline.parameters = f(self.pipeline.parameters);
        self
    }

    /// Sets parameters directly (convenience method)
    #[must_use]
    pub fn with_parameters(mut self, parameters: crate::pipeline::Parameters) -> Self {
        self.pipeline.parameters = parameters;
        self
    }

    /// Adds a trigger to pipeline
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.pipeline.triggers.push(trigger);
        self
    }

    /// Sets pipeline options
    pub fn options(mut self, options: PipelineOptions) -> Self {
        self.pipeline.options = options;
        self
    }

    /// Adds a post-condition to pipeline
    pub fn post(mut self, condition: PostCondition) -> Self {
        self.pipeline.post.push(condition);
        self
    }

    /// Adds multiple post-conditions to pipeline
    #[must_use]
    pub fn posts(mut self, conditions: Vec<PostCondition>) -> Self {
        self.pipeline.post.extend(conditions);
        self
    }

    /// Builds pipeline
    #[allow(clippy::missing_errors_doc)]
    pub fn build(self) -> Result<Pipeline, ValidationError> {
        self.pipeline.validate()?;
        Ok(self.pipeline)
    }

    /// Builds pipeline without validation (for internal use)
    #[must_use]
    pub fn build_unchecked(self) -> Pipeline {
        self.pipeline
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
