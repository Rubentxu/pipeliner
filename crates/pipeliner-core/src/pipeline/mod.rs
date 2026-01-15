//! Pipeline definition types and builders.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use crate::agent::AgentType;
use crate::environment::Environment;
use crate::matrix::MatrixConfig;
use crate::options::PipelineOptions;
use crate::parameters::Parameters;
use crate::validation::{Validate, ValidationError};

/// A pipeline definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    /// Pipeline name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Pipeline description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Agent configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentType>,

    /// Environment variables
    #[serde(default)]
    pub environment: Environment,

    /// Pipeline parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Parameters>,

    /// Pipeline options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<PipelineOptions>,

    /// Triggers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggers: Option<Triggers>,

    /// Stages
    pub stages: Vec<Stage>,

    /// Matrix configuration (for parallel execution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix: Option<MatrixConfig>,
}

/// Triggers for pipeline execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Triggers {
    /// Cron-based triggers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron: Option<String>,

    /// Poll SCM trigger
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_scm: Option<String>,

    /// Upstream jobs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<UpstreamTrigger>,
}

/// Upstream job trigger configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamTrigger {
    /// Jobs to watch
    pub jobs: Vec<String>,

    /// Threshold for triggering
    #[serde(default = "default_threshold")]
    pub threshold: String,
}

fn default_threshold() -> String {
    "SUCCESS".to_string()
}

/// A single stage in a pipeline
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    /// Stage name
    pub name: String,

    /// Agent configuration (stage-specific)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentType>,

    /// Stage-specific environment
    #[serde(default)]
    pub environment: Environment,

    /// Stage options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<StageOptions>,

    /// When conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<WhenCondition>,

    /// Post-actions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<PostCondition>,

    /// Steps in this stage
    pub steps: Vec<Step>,
}

/// Stage-specific options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StageOptions {
    /// Timeout for this stage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,

    /// Retry count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<usize>,

    /// Skip default checkout
    #[serde(default)]
    pub skip_default_checkout: bool,

    /// Stage-specific fail fast
    #[serde(default)]
    pub fail_fast: bool,
}

/// When condition for conditional stage execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhenCondition {
    /// Branch condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchCondition>,

    /// Environment condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentCondition>,

    /// Tag condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<TagCondition>,

    /// Expression condition (custom expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,

    /// All conditions must match
    #[serde(default)]
    pub all_of: Vec<WhenCondition>,

    /// Any condition must match
    #[serde(default)]
    pub any_of: Vec<WhenCondition>,

    /// Negate condition
    #[serde(default)]
    pub not: Option<Box<WhenCondition>>,
}

/// Branch matching condition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchCondition {
    /// Pattern to match
    pub pattern: String,

    /// Comparator type
    #[serde(default = "default_comparator")]
    pub comparator: String,
}

fn default_comparator() -> String {
    "GLOB".to_string()
}

/// Environment variable condition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCondition {
    /// Environment variable name
    pub name: String,

    /// Expected value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Pattern to match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

/// Tag matching condition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCondition {
    /// Pattern to match
    pub pattern: String,

    /// Comparator type
    #[serde(default = "default_comparator")]
    pub comparator: String,
}

/// Post-condition for stage completion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PostCondition {
    /// Always run
    #[serde(default)]
    pub always: Vec<Step>,

    /// Run on success
    #[serde(default)]
    pub success: Vec<Step>,

    /// Run on failure
    #[serde(default)]
    pub failure: Vec<Step>,

    /// Run on unstable
    #[serde(default)]
    pub unstable: Vec<Step>,

    /// Run when changed
    #[serde(default)]
    pub changed: Vec<Step>,

    /// Cleanup (always runs last)
    #[serde(default)]
    pub cleanup: Vec<Step>,
}

/// A single step in a stage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
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

    /// Retry a step
    Retry {
        /// Number of attempts
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

    /// Stash files
    Stash {
        /// Stash name
        name: String,
        /// Files to include
        #[serde(default)]
        includes: Vec<String>,
        /// Files to exclude
        #[serde(default)]
        excludes: Vec<String>,
    },

    /// Unstash files
    Unstash {
        /// Stash name
        name: String,
    },

    /// Input prompt
    Input {
        /// Message to display
        message: String,
        /// Default value
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
        /// Parameters to request
        #[serde(default)]
        parameters: Vec<StepParameter>,
    },

    /// Change directory
    Dir {
        /// Directory path
        path: PathBuf,
        /// Steps to execute in directory
        steps: Vec<Step>,
    },

    /// Script block
    Script {
        /// Script content
        content: String,
    },

    /// Archive artifacts
    Archive {
        /// Files to archive
        artifacts: Vec<String>,
        /// Exclude patterns
        #[serde(default)]
        excludes: Vec<String>,
        /// Fingerprint files
        #[serde(default)]
        fingerprint: bool,
    },

    /// Custom step (from plugin)
    Custom {
        /// Step name
        name: String,
        /// Configuration
        config: serde_json::Value,
    },
}

/// Step parameter definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StepParameter {
    /// String parameter
    String {
        name: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        default_value: Option<String>,
    },
    /// Boolean parameter
    Boolean {
        name: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        default_value: bool,
    },
    /// Choice parameter
    Choice {
        name: String,
        #[serde(default)]
        description: String,
        choices: Vec<String>,
    },
}

/// A step with metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// Step type
    #[serde(flatten)]
    pub step_type: StepType,

    /// Optional name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional timeout override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,

    /// Optional retry override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<usize>,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            step_type: StepType::Echo {
                message: String::new(),
            },
            name: None,
            timeout: None,
            retry: None,
        }
    }
}

impl Pipeline {
    /// Creates a new empty pipeline
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the pipeline name
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the pipeline description
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the agent
    #[must_use]
    pub fn with_agent(mut self, agent: AgentType) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Sets the environment
    #[must_use]
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    /// Sets the parameters
    #[must_use]
    pub fn with_parameters(mut self, parameters: Parameters) -> Self {
        self.parameters = Some(parameters);
        self
    }

    /// Sets the options
    #[must_use]
    pub fn with_options(mut self, options: PipelineOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Adds a stage
    #[must_use]
    pub fn with_stage(mut self, stage: Stage) -> Self {
        self.stages.push(stage);
        self
    }

    /// Sets the matrix configuration
    #[must_use]
    pub fn with_matrix(mut self, matrix: MatrixConfig) -> Self {
        self.matrix = Some(matrix);
        self
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            name: None,
            description: None,
            agent: None,
            environment: Environment::new(),
            parameters: None,
            options: None,
            triggers: None,
            stages: Vec::new(),
            matrix: None,
        }
    }
}

impl Validate for Pipeline {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.stages.is_empty() {
            return Err(ValidationError::EmptyStages);
        }

        for stage in &self.stages {
            stage.validate()?;
        }

        if let Some(matrix) = &self.matrix {
            matrix.validate()?;
        }

        if let Some(params) = &self.parameters {
            params.validate()?;
        }

        Ok(())
    }
}

impl Validate for Stage {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.name.is_empty() {
            return Err(ValidationError::EmptyName);
        }

        if self.steps.is_empty() {
            return Err(ValidationError::EmptySteps {
                stage: self.name.clone(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentType;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = Pipeline::new()
            .with_name("Test Pipeline")
            .with_agent(AgentType::any());

        assert_eq!(pipeline.name, Some("Test Pipeline".to_string()));
        assert!(matches!(pipeline.agent, Some(AgentType::Any)));
    }

    #[test]
    fn test_pipeline_validation_empty_stages() {
        let pipeline = Pipeline::new();
        assert!(pipeline.validate().is_err());
    }

    #[test]
    fn test_stage_validation() {
        let stage = Stage {
            name: "".to_string(),
            ..Default::default()
        };
        assert!(stage.validate().is_err());
    }

    #[test]
    fn test_step_types() {
        let shell = StepType::Shell {
            command: "echo hello".to_string(),
        };
        assert!(matches!(shell, StepType::Shell { .. }));

        let echo = StepType::Echo {
            message: "Hello".to_string(),
        };
        assert!(matches!(echo, StepType::Echo { .. }));
    }
}
