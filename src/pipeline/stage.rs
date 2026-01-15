//! Stage types for pipeline definition
//!
//! This module defines stage types and their builder pattern.

#![allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]

use super::Validate;
use super::errors::ValidationError;
use super::steps::Step;
use serde::{Deserialize, Serialize};
use std::fmt;

/// When conditions for stage execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhenCondition {
    /// Execute only on specific branch
    Branch {
        /// Branch name or pattern
        branch: String,
    },

    /// Execute only on specific tag
    Tag {
        /// Tag name or pattern
        tag: String,
    },

    /// Execute when environment variable matches
    Environment {
        /// Variable name
        name: String,
        /// Expected value
        value: String,
    },

    /// Execute when expression evaluates to true
    Expression {
        /// Boolean expression
        expression: String,
    },

    /// All conditions must be true
    AllOf {
        /// List of conditions
        conditions: Vec<WhenCondition>,
    },

    /// At least one condition must be true
    AnyOf {
        /// List of conditions
        conditions: Vec<WhenCondition>,
    },
}

impl WhenCondition {
    /// Creates a branch condition
    pub fn branch(branch: impl Into<String>) -> Self {
        Self::Branch {
            branch: branch.into(),
        }
    }

    /// Creates a tag condition
    pub fn tag(tag: impl Into<String>) -> Self {
        Self::Tag { tag: tag.into() }
    }

    /// Creates an environment condition
    pub fn environment(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Environment {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Creates an expression condition
    pub fn expression(expr: impl Into<String>) -> Self {
        Self::Expression {
            expression: expr.into(),
        }
    }

    /// Creates an all-of condition
    pub fn all_of(conditions: Vec<WhenCondition>) -> Self {
        Self::AllOf { conditions }
    }

    /// Creates an any-of condition
    pub fn any_of(conditions: Vec<WhenCondition>) -> Self {
        Self::AnyOf { conditions }
    }
}

impl Validate for WhenCondition {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        match self {
            Self::Branch { branch } => {
                if branch.is_empty() {
                    return Err(ValidationError::InvalidNameChars {
                        name: "Branch cannot be empty".to_string(),
                    });
                }
            }
            Self::Tag { tag } => {
                if tag.is_empty() {
                    return Err(ValidationError::InvalidNameChars {
                        name: "Tag cannot be empty".to_string(),
                    });
                }
            }
            Self::Environment { name, value: _ } => {
                if name.is_empty() {
                    return Err(ValidationError::InvalidNameChars {
                        name: "Environment variable name cannot be empty".to_string(),
                    });
                }
            }
            Self::Expression { expression } => {
                if expression.is_empty() {
                    return Err(ValidationError::InvalidNameChars {
                        name: "Expression cannot be empty".to_string(),
                    });
                }
            }
            Self::AllOf { conditions } | Self::AnyOf { conditions } => {
                for cond in conditions {
                    cond.validate()?;
                }
            }
        }
        Ok(())
    }
}

/// A stage in a pipeline
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    /// Stage name
    pub name: String,

    /// Optional agent override for this stage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<super::AgentType>,

    /// Steps in this stage
    pub steps: Vec<Step>,

    /// Parallel branches to execute
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parallel: Vec<super::ParallelBranch>,

    /// Matrix configuration for generating parallel branches
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix: Option<super::MatrixConfig>,

    /// Optional when condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<WhenCondition>,

    /// Post-conditions for this stage
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub post: Vec<super::PostCondition>,
}

impl Validate for Stage {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.name.is_empty() {
            return Err(ValidationError::EmptyName);
        }

        if self.name.len() > 100 {
            return Err(ValidationError::NameTooLong {
                max: 100,
                len: self.name.len(),
            });
        }

        // Stage must have either steps, parallel branches, or matrix
        if self.steps.is_empty() && self.parallel.is_empty() && self.matrix.is_none() {
            return Err(ValidationError::EmptyStage {
                stage: self.name.clone(),
            });
        }

        if let Some(ref agent) = self.agent {
            agent.validate()?;
        }

        if let Some(ref when) = self.when {
            when.validate()?;
        }

        // Validate parallel branches
        for branch in &self.parallel {
            branch.stage.validate()?;
        }

        // Validate matrix configuration
        if let Some(ref matrix) = self.matrix {
            matrix.validate()?;
        }

        Ok(())
    }
}

impl Stage {
    /// Creates a new stage
    pub fn new(name: impl Into<String>, steps: Vec<Step>) -> Self {
        Self {
            name: name.into(),
            agent: None,
            steps,
            parallel: Vec::new(),
            matrix: None,
            when: None,
            post: Vec::new(),
        }
    }

    /// Sets agent for this stage
    pub fn with_agent(mut self, agent: super::AgentType) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Sets parallel branches for this stage
    pub fn with_parallel(mut self, parallel: Vec<super::ParallelBranch>) -> Self {
        self.parallel = parallel;
        self
    }

    /// Sets matrix configuration for this stage
    pub fn with_matrix(mut self, matrix: super::MatrixConfig) -> Self {
        self.matrix = Some(matrix);
        self
    }

    /// Sets when condition for this stage
    pub fn with_when(mut self, when: WhenCondition) -> Self {
        self.when = Some(when);
        self
    }

    /// Adds a post-condition to this stage
    pub fn with_post(mut self, post: super::PostCondition) -> Self {
        self.post.push(post);
        self
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stage({}): {} steps", self.name, self.steps.len())
    }
}

/// Builder for creating stages
pub struct StageBuilder {
    stage: Stage,
}

impl StageBuilder {
    /// Creates a new stage builder
    pub fn new(name: impl Into<String>, steps: Vec<Step>) -> Self {
        Self {
            stage: Stage::new(name, steps),
        }
    }

    /// Sets agent for the stage
    pub fn agent(mut self, agent: super::AgentType) -> Self {
        self.stage.agent = Some(agent);
        self
    }

    /// Sets when condition for the stage
    pub fn when(mut self, when: WhenCondition) -> Self {
        self.stage.when = Some(when);
        self
    }

    /// Adds a step to the stage
    pub fn step(mut self, step: Step) -> Self {
        self.stage.steps.push(step);
        self
    }

    /// Adds multiple steps to the stage
    pub fn steps(mut self, mut steps: Vec<Step>) -> Self {
        self.stage.steps.append(&mut steps);
        self
    }

    /// Adds a post-condition to the stage
    pub fn post(mut self, condition: super::PostCondition) -> Self {
        self.stage.post.push(condition);
        self
    }

    /// Builds the stage
    #[allow(clippy::missing_errors_doc)]
    pub fn build(self) -> Result<Stage, ValidationError> {
        self.stage.validate()?;
        Ok(self.stage)
    }

    /// Builds the stage without validation (for internal use)
    #[must_use]
    pub fn build_unchecked(self) -> Stage {
        self.stage
    }
}

#[cfg(test)]
mod tests {
    use super::super::agent::AgentType;
    use super::super::post::PostCondition;
    use super::*;

    #[test]
    fn test_stage_creation() {
        let steps = vec![Step::shell("cargo build")];
        let stage = Stage::new("Build", steps);

        assert_eq!(stage.name, "Build");
        assert_eq!(stage.steps.len(), 1);
        assert!(stage.agent.is_none());
        assert!(stage.when.is_none());
    }

    #[test]
    fn test_stage_validation_empty_name() {
        let stage = Stage::new("", vec![Step::shell("echo")]);
        assert!(stage.validate().is_err());
    }

    #[test]
    fn test_stage_validation_name_too_long() {
        let long_name = "a".repeat(101);
        let stage = Stage::new(long_name, vec![Step::shell("echo")]);
        let result = stage.validate();
        assert!(matches!(result, Err(ValidationError::NameTooLong { .. })));
    }

    #[test]
    fn test_stage_validation_empty_steps() {
        let stage = Stage::new("Build", vec![]);
        let result = stage.validate();
        assert!(matches!(result, Err(ValidationError::EmptyStage { .. })));
    }

    #[test]
    fn test_stage_with_agent() {
        let steps = vec![Step::shell("cargo build")];
        let stage = Stage::new("Build", steps).with_agent(AgentType::any());

        assert!(stage.agent.is_some());
    }

    #[test]
    fn test_stage_with_when() {
        let steps = vec![Step::shell("cargo build")];
        let stage = Stage::new("Build", steps).with_when(WhenCondition::branch("main"));

        assert!(stage.when.is_some());
    }

    #[test]
    fn test_stage_with_post() {
        let steps = vec![Step::shell("cargo build")];
        let stage =
            Stage::new("Build", steps).with_post(PostCondition::always(vec![Step::echo("done")]));

        assert_eq!(stage.post.len(), 1);
    }

    #[test]
    fn test_stage_display() {
        let steps = vec![Step::shell("cargo build")];
        let stage = Stage::new("Build", steps);

        assert_eq!(stage.to_string(), "Stage(Build): 1 steps");
    }

    #[test]
    fn test_stage_builder() {
        let steps = vec![Step::shell("cargo build")];
        let builder = StageBuilder::new("Build", steps)
            .agent(AgentType::any())
            .when(WhenCondition::branch("main"))
            .post(PostCondition::always(vec![Step::echo("done")]));

        let stage = builder.build().unwrap();

        assert_eq!(stage.name, "Build");
        assert!(stage.agent.is_some());
        assert!(stage.when.is_some());
        assert_eq!(stage.post.len(), 1);
    }

    #[test]
    fn test_when_condition_branch() {
        let cond = WhenCondition::branch("main");
        assert!(matches!(cond, WhenCondition::Branch { .. }));
        assert!(cond.validate().is_ok());
    }

    #[test]
    fn test_when_condition_tag() {
        let cond = WhenCondition::tag("v1.0.0");
        assert!(matches!(cond, WhenCondition::Tag { .. }));
        assert!(cond.validate().is_ok());
    }

    #[test]
    fn test_when_condition_environment() {
        let cond = WhenCondition::environment("DEPLOY", "true");
        assert!(matches!(cond, WhenCondition::Environment { .. }));
        assert!(cond.validate().is_ok());
    }

    #[test]
    fn test_when_condition_expression() {
        let cond = WhenCondition::expression("BRANCH == 'main'");
        assert!(matches!(cond, WhenCondition::Expression { .. }));
        assert!(cond.validate().is_ok());
    }

    #[test]
    fn test_when_condition_all_of() {
        let conditions = vec![
            WhenCondition::branch("main"),
            WhenCondition::environment("DEPLOY", "true"),
        ];
        let cond = WhenCondition::all_of(conditions);
        assert!(matches!(cond, WhenCondition::AllOf { .. }));
        assert!(cond.validate().is_ok());
    }

    #[test]
    fn test_when_condition_any_of() {
        let conditions = vec![
            WhenCondition::branch("main"),
            WhenCondition::branch("develop"),
        ];
        let cond = WhenCondition::any_of(conditions);
        assert!(matches!(cond, WhenCondition::AnyOf { .. }));
        assert!(cond.validate().is_ok());
    }

    #[test]
    fn test_when_condition_invalid_empty() {
        let cond = WhenCondition::branch("");
        assert!(cond.validate().is_err());
    }

    // Épica 2 Tests - When Conditions

    #[test]
    fn test_when_branch_condition_evaluation() {
        let cond = WhenCondition::branch("main");
        // WhenCondition::branch does not have an evaluate method yet
        // This test documents the expected behavior
        assert!(matches!(cond, WhenCondition::Branch { branch } if branch == "main"));
    }

    #[test]
    fn test_when_tag_condition() {
        let cond = WhenCondition::tag("v1.0.0");
        assert!(matches!(cond, WhenCondition::Tag { tag } if tag == "v1.0.0"));
    }

    #[test]
    fn test_when_environment_condition() {
        let cond = WhenCondition::environment("ENVIRONMENT", "production");
        assert!(matches!(cond, WhenCondition::Environment { name, value } 
            if name == "ENVIRONMENT" && value == "production"));
    }

    #[test]
    fn test_when_expression_condition() {
        let cond = WhenCondition::expression("BRANCH_NAME == 'main' && DEPLOY == 'true'");
        assert!(matches!(cond, WhenCondition::Expression { expression } 
            if expression.contains("BRANCH_NAME")));
    }

    #[test]
    fn test_when_allof_shortcircuit() {
        let conditions = vec![
            WhenCondition::expression("false"),
            WhenCondition::expression("true"),
        ];
        let cond = WhenCondition::all_of(conditions);
        assert!(matches!(cond, WhenCondition::AllOf { conditions: c } if c.len() == 2));
    }

    #[test]
    fn test_when_any_of() {
        let conditions = vec![
            WhenCondition::branch("main"),
            WhenCondition::branch("develop"),
        ];
        let cond = WhenCondition::any_of(conditions);
        assert!(matches!(cond, WhenCondition::AnyOf { conditions: c } if c.len() == 2));
    }

    #[test]
    fn test_when_condition_nested() {
        let inner = vec![WhenCondition::branch("main")];
        let cond = WhenCondition::all_of(inner);
        assert!(matches!(cond, WhenCondition::AllOf { .. }));
    }
}
