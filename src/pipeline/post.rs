//! Post-condition types for pipeline execution
//!
//! This module defines conditions that execute after pipeline or stage completion.

#![allow(clippy::must_use_candidate)]

use super::steps::Step;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Conditions that execute after pipeline or stage completion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostCondition {
    /// Always execute regardless of result
    Always {
        /// Steps to execute
        steps: Vec<Step>,
    },

    /// Execute only on success
    Success {
        /// Steps to execute
        steps: Vec<Step>,
    },

    /// Execute only on failure
    Failure {
        /// Steps to execute
        steps: Vec<Step>,
    },

    /// Execute on unstable or failure
    Unstable {
        /// Steps to execute
        steps: Vec<Step>,
    },

    /// Execute when result differs from previous run
    Changed {
        /// Steps to execute
        steps: Vec<Step>,
    },
}

impl PostCondition {
    /// Creates an "always" condition
    pub fn always(steps: Vec<Step>) -> Self {
        Self::Always { steps }
    }

    /// Creates a "success" condition
    pub fn success(steps: Vec<Step>) -> Self {
        Self::Success { steps }
    }

    /// Creates a "failure" condition
    pub fn failure(steps: Vec<Step>) -> Self {
        Self::Failure { steps }
    }

    /// Creates an "unstable" condition
    pub fn unstable(steps: Vec<Step>) -> Self {
        Self::Unstable { steps }
    }

    /// Creates a "changed" condition
    pub fn changed(steps: Vec<Step>) -> Self {
        Self::Changed { steps }
    }

    /// Returns the steps for this condition
    pub fn steps(&self) -> &[Step] {
        match self {
            Self::Always { steps }
            | Self::Success { steps }
            | Self::Failure { steps }
            | Self::Unstable { steps }
            | Self::Changed { steps } => steps,
        }
    }

    /// Returns true if this condition should execute given the result
    pub fn should_execute(
        &self,
        result: super::StageResult,
        previous: Option<super::StageResult>,
    ) -> bool {
        match self {
            Self::Always { .. } => true,
            Self::Success { .. } => result.is_success(),
            Self::Failure { .. } => result.is_failure(),
            Self::Unstable { .. } => result.is_unstable() || result.is_failure(),
            Self::Changed { .. } => {
                if let Some(prev) = previous {
                    prev != result
                } else {
                    true // First run always counts as changed
                }
            }
        }
    }
}

impl fmt::Display for PostCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Always { steps } => write!(f, "always({} steps)", steps.len()),
            Self::Success { steps } => write!(f, "success({} steps)", steps.len()),
            Self::Failure { steps } => write!(f, "failure({} steps)", steps.len()),
            Self::Unstable { steps } => write!(f, "unstable({} steps)", steps.len()),
            Self::Changed { steps } => write!(f, "changed({} steps)", steps.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::StageResult;
    use super::*;

    #[test]
    fn test_post_condition_always() {
        let steps = vec![Step::echo("test")];
        let cond = PostCondition::always(steps.clone());
        assert_eq!(cond.steps().len(), 1);
        assert!(cond.should_execute(StageResult::Success, None));
        assert!(cond.should_execute(StageResult::Failure, None));
    }

    #[test]
    fn test_post_condition_success() {
        let steps = vec![Step::echo("test")];
        let cond = PostCondition::success(steps);

        assert!(cond.should_execute(StageResult::Success, None));
        assert!(!cond.should_execute(StageResult::Failure, None));
        assert!(!cond.should_execute(StageResult::Unstable, None));
    }

    #[test]
    fn test_post_condition_failure() {
        let steps = vec![Step::echo("test")];
        let cond = PostCondition::failure(steps);

        assert!(!cond.should_execute(StageResult::Success, None));
        assert!(cond.should_execute(StageResult::Failure, None));
    }

    #[test]
    fn test_post_condition_unstable() {
        let steps = vec![Step::echo("test")];
        let cond = PostCondition::unstable(steps);

        assert!(!cond.should_execute(StageResult::Success, None));
        assert!(cond.should_execute(StageResult::Unstable, None));
        assert!(cond.should_execute(StageResult::Failure, None));
    }

    #[test]
    fn test_post_condition_changed() {
        let steps = vec![Step::echo("test")];
        let cond = PostCondition::changed(steps);

        // First run always executes
        assert!(cond.should_execute(StageResult::Success, None));

        // Same result doesn't execute
        assert!(!cond.should_execute(StageResult::Success, Some(StageResult::Success)));

        // Different result executes
        assert!(cond.should_execute(StageResult::Failure, Some(StageResult::Success)));
    }

    #[test]
    fn test_post_condition_display() {
        let cond = PostCondition::always(vec![Step::echo("test")]);
        assert_eq!(cond.to_string(), "always(1 steps)");

        let cond = PostCondition::success(vec![Step::echo("test")]);
        assert_eq!(cond.to_string(), "success(1 steps)");
    }

    // Épica 2 Tests - Evaluación completa de post-conditions

    #[test]
    fn test_post_always_executes_on_success() {
        let cond = PostCondition::always(vec![Step::echo("cleanup")]);
        assert!(cond.should_execute(StageResult::Success, None));
    }

    #[test]
    fn test_post_always_executes_on_failure() {
        let cond = PostCondition::always(vec![Step::echo("cleanup")]);
        assert!(cond.should_execute(StageResult::Failure, None));
    }

    #[test]
    fn test_post_success_only_on_success() {
        let cond = PostCondition::success(vec![Step::echo("notify")]);
        assert!(cond.should_execute(StageResult::Success, None));
        assert!(!cond.should_execute(StageResult::Failure, None));
        assert!(!cond.should_execute(StageResult::Unstable, None));
    }

    #[test]
    fn test_post_failure_only_on_failure() {
        let cond = PostCondition::failure(vec![Step::echo("alert")]);
        assert!(!cond.should_execute(StageResult::Success, None));
        assert!(cond.should_execute(StageResult::Failure, None));
    }

    #[test]
    fn test_post_unstable_on_unstable_or_failure() {
        let cond = PostCondition::unstable(vec![Step::echo("warning")]);
        assert!(!cond.should_execute(StageResult::Success, None));
        assert!(cond.should_execute(StageResult::Unstable, None));
        assert!(cond.should_execute(StageResult::Failure, None));
    }

    #[test]
    fn test_post_changed_detects_result_change() {
        let cond = PostCondition::changed(vec![Step::echo("changed")]);

        // First run - should execute
        assert!(cond.should_execute(StageResult::Success, None));

        // Same result - should not execute
        assert!(!cond.should_execute(StageResult::Success, Some(StageResult::Success)));

        // Different result (Failure after Success) - should execute
        assert!(cond.should_execute(StageResult::Failure, Some(StageResult::Success)));

        // Different result (Success after Failure) - should execute
        assert!(cond.should_execute(StageResult::Success, Some(StageResult::Failure)));

        // Same result again - should not execute
        assert!(!cond.should_execute(StageResult::Success, Some(StageResult::Success)));
    }
}
