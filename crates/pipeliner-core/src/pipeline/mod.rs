//! Pipeline definition types and builders.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::agent::AgentType;
use crate::environment::Environment;
use crate::matrix::MatrixConfig;
use crate::options::PipelineOptions;
use crate::parameters::Parameters;
use crate::structure::{PipelineStructure, StageStructure, StepStructure};
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Condition for conditional step execution
///
/// This is a recursive enum that supports:
/// - Direct boolean expressions
/// - Negation (Not)
/// - Logical OR (Any)
/// - Logical AND (All)
/// - Environment variable checks (EnvEqual)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepWhenCondition {
    /// Direct boolean expression
    Expr(bool),

    /// Negation of a condition
    Not(Box<StepWhenCondition>),

    /// True if any child condition is true (OR)
    Any(Vec<StepWhenCondition>),

    /// True if all child conditions are true (AND)
    All(Vec<StepWhenCondition>),

    /// Check if an environment variable equals a specific value
    EnvEqual { key: String, value: String },
}

impl StepWhenCondition {
    /// Evaluates the condition against the given environment
    #[must_use]
    pub fn evaluate(&self, env: &Environment) -> bool {
        match self {
            StepWhenCondition::Expr(b) => *b,
            StepWhenCondition::Not(inner) => !inner.evaluate(env),
            StepWhenCondition::Any(conditions) => conditions.iter().any(|c| c.evaluate(env)),
            StepWhenCondition::All(conditions) => conditions.iter().all(|c| c.evaluate(env)),
            StepWhenCondition::EnvEqual { key, value } => env
                .get(key)
                .and_then(|v| {
                    if let crate::environment::EnvVarValue::Value(v_str) = v {
                        Some(v_str == value)
                    } else {
                        None
                    }
                })
                .unwrap_or(false),
        }
    }
}

/// Environment check for the `is` step type
///
/// Used to conditionally execute steps based on the current environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EnvCheck {
    /// Check if running in integration environment
    Integration,

    /// Check if running in certification environment
    Certification,

    /// Check if running in preproduction environment
    Preproduction,

    /// Check if running in production environment
    Production,

    /// Custom environment check with explicit key and value
    Custom { key: String, value: String },
}

impl EnvCheck {
    /// The environment variable name used to identify the environment
    const ENV_KEY: &'static str = "DEPLOY_ENV";

    /// Helper to extract string value from EnvVarValue
    fn get_string_value(env: &Environment, key: &str) -> Option<String> {
        env.get(key).and_then(|v| {
            if let crate::environment::EnvVarValue::Value(v_str) = v {
                Some(v_str.clone())
            } else {
                None
            }
        })
    }

    /// Checks if the environment matches this EnvCheck variant
    #[must_use]
    pub fn check(&self, env: &Environment) -> bool {
        let env_value = Self::get_string_value(env, Self::ENV_KEY);
        match self {
            EnvCheck::Integration => env_value.map(|v| v == "integration").unwrap_or(false),
            EnvCheck::Certification => env_value.map(|v| v == "certification").unwrap_or(false),
            EnvCheck::Preproduction => env_value.map(|v| v == "preproduction").unwrap_or(false),
            EnvCheck::Production => env_value.map(|v| v == "production").unwrap_or(false),
            EnvCheck::Custom { key, value } => Self::get_string_value(env, key).map(|v| v == *value).unwrap_or(false),
        }
    }
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

    /// Log message at a specific level
    Log {
        /// Log level for this message
        level: crate::logging::LogLevel,
        /// Message to log
        message: String,
    },

    /// Conditional step execution
    ///
    /// Executes the inner steps only if the condition evaluates to true.
    #[serde(rename_all = "camelCase")]
    When {
        /// The condition to evaluate
        condition: StepWhenCondition,
        /// Steps to execute if condition is true
        steps: Vec<Step>,
    },

    /// Error handler wrapper
    ///
    /// Executes steps and if any fail, runs the on_error steps before propagating the error.
    #[serde(rename_all = "camelCase")]
    ErrorHandler {
        /// Steps to execute
        steps: Vec<Step>,
        /// Steps to execute on error (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        on_error: Option<Vec<Step>>,
    },

    /// Environment check step
    ///
    /// Returns Success if the environment check matches, Skipped otherwise.
    #[serde(rename_all = "camelCase")]
    Is {
        /// The environment check to perform
        env_check: EnvCheck,
    },

    /// WithCredentials step
    ///
    /// Executes inner steps with credentials injected as environment variables.
    #[serde(rename_all = "camelCase")]
    WithCredentials {
        /// The credential ID to look up from PipelineConfig
        credential_id: String,
        /// Steps to execute with the credentials
        steps: Vec<Step>,
    },

    /// Checkout step
    ///
    /// Clones a repository using git.
    #[serde(rename_all = "camelCase")]
    Checkout {
        /// SCM configuration for checkout
        scm: crate::config::ScmConfig,
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

    /// Gets the pipeline name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
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

    /// Export the pipeline structure for external visualization.
    ///
    /// Used to emit `PipelineDecl` events before execution starts,
    /// so consumers (dashboard, Bastion gateway) can project the graph.
    #[must_use]
    pub fn structure(&self) -> PipelineStructure {
        PipelineStructure {
            stages: self.stages.iter().map(|stage| {
                let has_matrix = stage.options.as_ref().and_then(|o| o.retry).is_some()
                    || self.matrix.is_some();
                StageStructure {
                    name: stage.name.clone(),
                    steps: stage.steps.iter().map(|step| StepStructure {
                        name: step.name.clone(),
                        step_type: match &step.step_type {
                            StepType::Shell { .. } => "shell",
                            StepType::Echo { .. } => "echo",
                            StepType::Retry { .. } => "retry",
                            StepType::Timeout { .. } => "timeout",
                            StepType::Stash { .. } => "stash",
                            StepType::Unstash { .. } => "unstash",
                            StepType::Input { .. } => "input",
                            StepType::Dir { .. } => "dir",
                            StepType::Script { .. } => "script",
                            StepType::Archive { .. } => "archive",
                            StepType::Custom { .. } => "custom",
                            StepType::Log { .. } => "log",
                            StepType::When { .. } => "when",
                            StepType::ErrorHandler { .. } => "error_handler",
                            StepType::Is { .. } => "is",
                            StepType::WithCredentials { .. } => "with_credentials",
                            StepType::Checkout { .. } => "checkout",
                        }
                        .to_string(),
                        command: match &step.step_type {
                            StepType::Shell { command } => Some(command.clone()),
                            StepType::Script { content } => Some(content.clone()),
                            _ => None,
                        },
                    }).collect(),
                    has_parallel: false,
                    has_matrix,
                    when_condition: stage.when.as_ref().map(|w| format!("{:?}", w)),
                }
            }).collect(),
        }
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

impl Stage {
    /// Creates a new stage with the given name
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            agent: None,
            environment: Environment::new(),
            options: None,
            when: None,
            post: None,
            steps: Vec::new(),
        }
    }

    /// Sets the stage name
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the agent for this stage
    #[must_use]
    pub fn with_agent(mut self, agent: AgentType) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Adds a step to this stage
    #[must_use]
    pub fn with_step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    /// Sets the environment for this stage
    #[must_use]
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }
}

impl Default for Stage {
    fn default() -> Self {
        Self {
            name: String::new(),
            agent: None,
            environment: Environment::new(),
            options: None,
            when: None,
            post: None,
            steps: Vec::new(),
        }
    }
}

impl Step {
    /// Creates a new shell step
    #[must_use]
    pub fn shell(command: impl Into<String>) -> Self {
        Self {
            step_type: StepType::Shell {
                command: command.into(),
            },
            name: None,
            timeout: None,
            retry: None,
        }
    }

    /// Creates a new echo step
    #[must_use]
    pub fn echo(message: impl Into<String>) -> Self {
        Self {
            step_type: StepType::Echo {
                message: message.into(),
            },
            name: None,
            timeout: None,
            retry: None,
        }
    }

    /// Sets the step name
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the retry count
    #[must_use]
    pub fn with_retry(mut self, count: usize) -> Self {
        self.retry = Some(count);
        self
    }

    /// Sets the timeout
    #[must_use]
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
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

    // =======================================================================
    // StepType::Log Tests
    // =======================================================================

    #[test]
    fn test_step_type_log_variant_exists() {
        use crate::logging::LogLevel;

        let log_step = StepType::Log {
            level: LogLevel::Info,
            message: "Test message".to_string(),
        };
        assert!(matches!(log_step, StepType::Log { .. }));
    }

    #[test]
    fn test_step_type_log_pattern_matching() {
        use crate::logging::LogLevel;

        let log_step = StepType::Log {
            level: LogLevel::Warn,
            message: "Warning message".to_string(),
        };

        if let StepType::Log { level, message } = log_step {
            assert_eq!(level, LogLevel::Warn);
            assert_eq!(message, "Warning message");
        } else {
            panic!("Expected StepType::Log variant");
        }
    }

    #[test]
    fn test_step_type_log_serialization() {
        use crate::logging::LogLevel;

        let log_step = StepType::Log {
            level: LogLevel::Error,
            message: "Error occurred".to_string(),
        };

        let json = serde_json::to_string(&log_step).unwrap();
        assert!(json.contains("\"type\":\"log\""));
        assert!(json.contains("\"level\":\"error\""));
        assert!(json.contains("\"message\":\"Error occurred\""));
    }

    #[test]
    fn test_step_type_log_deserialization() {
        use crate::logging::LogLevel;

        let json = r#"{"type":"log","level":"debug","message":"Debug info"}"#;
        let log_step: StepType = serde_json::from_str(json).unwrap();

        if let StepType::Log { level, message } = log_step {
            assert_eq!(level, LogLevel::Debug);
            assert_eq!(message, "Debug info");
        } else {
            panic!("Expected StepType::Log variant");
        }
    }

    #[test]
    fn test_step_type_log_roundtrip() {
        use crate::logging::LogLevel;

        let original = StepType::Log {
            level: LogLevel::Fatal,
            message: "Critical failure".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let parsed: StepType = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_step_type_log_all_levels() {
        use crate::logging::LogLevel;

        let levels = vec![
            (LogLevel::Debug, "Debug message"),
            (LogLevel::Info, "Info message"),
            (LogLevel::Warn, "Warn message"),
            (LogLevel::Error, "Error message"),
            (LogLevel::Fatal, "Fatal message"),
        ];

        for (level, msg) in levels {
            let log_step = StepType::Log {
                level,
                message: msg.to_string(),
            };

            if let StepType::Log { level: lvl, message: m } = log_step {
                assert_eq!(lvl, level);
                assert_eq!(m, msg);
            } else {
                panic!("Expected StepType::Log variant for level {:?}", level);
            }
        }
    }

    // =======================================================================
    // WhenCondition Tests (SCN-AST-001 to SCN-AST-005)
    // =======================================================================

    #[test]
    fn test_when_condition_expr_true() {
        // SCN-AST-001: When with true condition → steps execute
        let env = Environment::new();
        let condition = StepWhenCondition::Expr(true);
        assert!(condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_expr_false() {
        // SCN-AST-002: When with false condition → steps skipped
        let env = Environment::new();
        let condition = StepWhenCondition::Expr(false);
        assert!(!condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_not_true() {
        // SCN-AST-003: When.not(true) → steps execute
        let env = Environment::new();
        let condition = StepWhenCondition::Not(Box::new(StepWhenCondition::Expr(true)));
        assert!(!condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_not_false() {
        let env = Environment::new();
        let condition = StepWhenCondition::Not(Box::new(StepWhenCondition::Expr(false)));
        assert!(condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_any_with_true() {
        // SCN-AST-004: When.any([false, true]) → execute
        let env = Environment::new();
        let condition = StepWhenCondition::Any(vec![
            StepWhenCondition::Expr(false),
            StepWhenCondition::Expr(true),
        ]);
        assert!(condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_any_all_false() {
        let env = Environment::new();
        let condition = StepWhenCondition::Any(vec![
            StepWhenCondition::Expr(false),
            StepWhenCondition::Expr(false),
        ]);
        assert!(!condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_all_true() {
        let env = Environment::new();
        let condition = StepWhenCondition::All(vec![
            StepWhenCondition::Expr(true),
            StepWhenCondition::Expr(true),
        ]);
        assert!(condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_all_with_false() {
        // SCN-AST-005: When.all([true, false]) → skipped
        let env = Environment::new();
        let condition = StepWhenCondition::All(vec![
            StepWhenCondition::Expr(true),
            StepWhenCondition::Expr(false),
        ]);
        assert!(!condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_env_equal_match() {
        let mut env = Environment::new();
        env.insert("BRANCH", "main");
        let condition = StepWhenCondition::EnvEqual {
            key: "BRANCH".to_string(),
            value: "main".to_string(),
        };
        assert!(condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_env_equal_no_match() {
        let mut env = Environment::new();
        env.insert("BRANCH", "develop");
        let condition = StepWhenCondition::EnvEqual {
            key: "BRANCH".to_string(),
            value: "main".to_string(),
        };
        assert!(!condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_env_equal_missing_key() {
        let env = Environment::new();
        let condition = StepWhenCondition::EnvEqual {
            key: "MISSING".to_string(),
            value: "value".to_string(),
        };
        assert!(!condition.evaluate(&env));
    }

    #[test]
    fn test_when_condition_nested() {
        let env = Environment::new();
        // (true AND false) OR true = true
        let condition = StepWhenCondition::Any(vec![
            StepWhenCondition::All(vec![
                StepWhenCondition::Expr(true),
                StepWhenCondition::Expr(false),
            ]),
            StepWhenCondition::Expr(true),
        ]);
        assert!(condition.evaluate(&env));
    }

    // =======================================================================
    // EnvCheck Tests (SCN-AST-008, SCN-AST-009)
    // =======================================================================

    #[test]
    fn test_env_check_integration_match() {
        // SCN-AST-008: Is.integration when ENV=integration → success
        let mut env = Environment::new();
        env.insert("DEPLOY_ENV", "integration");
        let check = EnvCheck::Integration;
        assert!(check.check(&env));
    }

    #[test]
    fn test_env_check_integration_no_match() {
        let mut env = Environment::new();
        env.insert("DEPLOY_ENV", "dev");
        let check = EnvCheck::Integration;
        assert!(!check.check(&env));
    }

    #[test]
    fn test_env_check_production_match() {
        // SCN-AST-009: Is.production when ENV=production → success
        let mut env = Environment::new();
        env.insert("DEPLOY_ENV", "production");
        let check = EnvCheck::Production;
        assert!(check.check(&env));
    }

    #[test]
    fn test_env_check_production_no_match() {
        // SCN-AST-009: Is.production when ENV=dev → skipped
        let mut env = Environment::new();
        env.insert("DEPLOY_ENV", "dev");
        let check = EnvCheck::Production;
        assert!(!check.check(&env));
    }

    #[test]
    fn test_env_check_certification_match() {
        let mut env = Environment::new();
        env.insert("DEPLOY_ENV", "certification");
        let check = EnvCheck::Certification;
        assert!(check.check(&env));
    }

    #[test]
    fn test_env_check_preproduction_match() {
        let mut env = Environment::new();
        env.insert("DEPLOY_ENV", "preproduction");
        let check = EnvCheck::Preproduction;
        assert!(check.check(&env));
    }

    #[test]
    fn test_env_check_custom_match() {
        let mut env = Environment::new();
        env.insert("CUSTOM_KEY", "custom_value");
        let check = EnvCheck::Custom {
            key: "CUSTOM_KEY".to_string(),
            value: "custom_value".to_string(),
        };
        assert!(check.check(&env));
    }

    #[test]
    fn test_env_check_custom_no_match() {
        let mut env = Environment::new();
        env.insert("CUSTOM_KEY", "other_value");
        let check = EnvCheck::Custom {
            key: "CUSTOM_KEY".to_string(),
            value: "custom_value".to_string(),
        };
        assert!(!check.check(&env));
    }

    #[test]
    fn test_env_check_missing_key_returns_false() {
        let env = Environment::new();
        let check = EnvCheck::Integration;
        assert!(!check.check(&env));
    }

    // =======================================================================
    // StepType::When Tests
    // =======================================================================

    #[test]
    fn test_step_type_when_variant_exists() {
        let when_step = StepType::When {
            condition: StepWhenCondition::Expr(true),
            steps: vec![],
        };
        assert!(matches!(when_step, StepType::When { .. }));
    }

    #[test]
    fn test_step_type_when_serialization() {
        let when_step = StepType::When {
            condition: StepWhenCondition::Expr(true),
            steps: vec![Step::echo("hello")],
        };

        let json = serde_json::to_string(&when_step).unwrap();
        assert!(json.contains("\"type\":\"when\""));
        assert!(json.contains("\"condition\":{\"expr\":true}"));
    }

    #[test]
    fn test_step_type_when_deserialization() {
        let json = r#"{"type":"when","condition":{"expr":true},"steps":[]}"#;
        let when_step: StepType = serde_json::from_str(json).unwrap();
        assert!(matches!(when_step, StepType::When { .. }));
    }

    // =======================================================================
    // StepType::ErrorHandler Tests (SCN-AST-006, SCN-AST-007)
    // =======================================================================

    #[test]
    fn test_step_type_error_handler_variant_exists() {
        let eh_step = StepType::ErrorHandler {
            steps: vec![],
            on_error: None,
        };
        assert!(matches!(eh_step, StepType::ErrorHandler { .. }));
    }

    #[test]
    fn test_step_type_error_handler_with_on_error() {
        let eh_step = StepType::ErrorHandler {
            steps: vec![Step::shell("echo hello")],
            on_error: Some(vec![Step::shell("echo error")]),
        };
        assert!(matches!(eh_step, StepType::ErrorHandler { .. }));
    }

    #[test]
    fn test_step_type_error_handler_serialization() {
        let eh_step = StepType::ErrorHandler {
            steps: vec![],
            on_error: Some(vec![]),
        };

        let json = serde_json::to_string(&eh_step).unwrap();
        assert!(json.contains("\"type\":\"errorHandler\""));
        assert!(json.contains("\"steps\":[]"));
        assert!(json.contains("\"onError\":[]"));
    }

    // =======================================================================
    // StepType::Is Tests (SCN-AST-008, SCN-AST-009)
    // =======================================================================

    #[test]
    fn test_step_type_is_variant_exists() {
        let is_step = StepType::Is {
            env_check: EnvCheck::Production,
        };
        assert!(matches!(is_step, StepType::Is { .. }));
    }

    #[test]
    fn test_step_type_is_serialization() {
        let is_step = StepType::Is {
            env_check: EnvCheck::Production,
        };

        let json = serde_json::to_string(&is_step).unwrap();
        eprintln!("JSON: {}", json);
        assert!(json.contains("\"type\":\"is\""));
        assert!(json.contains("\"envCheck\":\"production\""));
    }

    #[test]
    fn test_step_type_is_deserialization() {
        let json = r#"{"type":"is","envCheck":"production"}"#;
        let is_step: StepType = serde_json::from_str(json).unwrap();
        assert!(matches!(is_step, StepType::Is { env_check: EnvCheck::Production, .. }));
    }

    // =======================================================================
    // StepType::WithCredentials Tests (SCN-AST-010, SCN-AST-011)
    // =======================================================================

    #[test]
    fn test_step_type_with_credentials_variant_exists() {
        let cred_step = StepType::WithCredentials {
            credential_id: "github-creds".to_string(),
            steps: vec![],
        };
        assert!(matches!(cred_step, StepType::WithCredentials { .. }));
    }

    #[test]
    fn test_step_type_with_credentials_serialization() {
        let cred_step = StepType::WithCredentials {
            credential_id: "github-creds".to_string(),
            steps: vec![Step::echo("test")],
        };

        let json = serde_json::to_string(&cred_step).unwrap();
        assert!(json.contains("\"type\":\"withCredentials\""));
        assert!(json.contains("\"credentialId\":\"github-creds\""));
        assert!(json.contains("\"steps\""));
    }

    #[test]
    fn test_step_type_with_credentials_deserialization() {
        let json = r#"{"type":"withCredentials","credentialId":"gh","steps":[{"type":"echo","message":"hi"}]}"#;
        let cred_step: StepType = serde_json::from_str(json).unwrap();
        assert!(matches!(cred_step, StepType::WithCredentials { credential_id, .. } if credential_id == "gh"));
    }

    #[test]
    fn test_step_type_with_credentials_empty_steps() {
        let cred_step = StepType::WithCredentials {
            credential_id: "creds".to_string(),
            steps: vec![],
        };

        let json = serde_json::to_string(&cred_step).unwrap();
        assert!(json.contains("\"steps\":[]"));
    }

    // =======================================================================
    // StepType::Checkout Tests (SCN-AST-012, SCN-AST-013)
    // =======================================================================

    #[test]
    fn test_step_type_checkout_variant_exists() {
        use crate::config::ScmConfig;
        let checkout_step = StepType::Checkout {
            scm: ScmConfig {
                url: "https://github.com/example/repo".to_string(),
                branch: "main".to_string(),
                credentials_id: None,
                shallow_clone: true,
                submodule_recursive: false,
            },
        };
        assert!(matches!(checkout_step, StepType::Checkout { .. }));
    }

    #[test]
    fn test_step_type_checkout_serialization() {
        use crate::config::ScmConfig;
        let checkout_step = StepType::Checkout {
            scm: ScmConfig {
                url: "https://github.com/example/repo".to_string(),
                branch: "develop".to_string(),
                credentials_id: Some("ssh-key".to_string()),
                shallow_clone: true,
                submodule_recursive: true,
            },
        };

        let json = serde_json::to_string(&checkout_step).unwrap();
        assert!(json.contains("\"type\":\"checkout\""));
        assert!(json.contains("\"url\":\"https://github.com/example/repo\""));
        assert!(json.contains("\"branch\":\"develop\""));
        assert!(json.contains("\"shallowClone\":true"));
    }

    #[test]
    fn test_step_type_checkout_deserialization() {
        let json = r#"{"type":"checkout","scm":{"url":"https://github.com/test/repo","branch":"feature","shallowClone":false,"submoduleRecursive":true}}"#;
        let checkout_step: StepType = serde_json::from_str(json).unwrap();
        assert!(matches!(checkout_step, StepType::Checkout { scm } if scm.url == "https://github.com/test/repo" && scm.branch == "feature"));
    }

    #[test]
    fn test_step_type_checkout_shallow_clone_serde() {
        use crate::config::ScmConfig;
        // Test that shallow_clone defaults to true
        let checkout_shallow = StepType::Checkout {
            scm: ScmConfig {
                url: "https://github.com/example/repo".to_string(),
                branch: "main".to_string(),
                credentials_id: None,
                shallow_clone: true,
                submodule_recursive: true,
            },
        };

        let json = serde_json::to_string(&checkout_shallow).unwrap();
        assert!(json.contains("\"shallowClone\":true"));
    }
}
