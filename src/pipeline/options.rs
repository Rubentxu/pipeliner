//! Pipeline options and triggers
//!
//! This module defines configuration options and trigger types for pipelines.

use super::errors::ValidationError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Triggers for pipeline execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trigger {
    /// Cron schedule trigger
    #[serde(rename = "cron")]
    CronSchedule {
        /// Cron expression
        expression: String,
        /// Timezone
        #[serde(skip_serializing_if = "Option::is_none")]
        timezone: Option<String>,
    },

    /// Poll SCM trigger
    PollScm {
        /// Polling interval in minutes
        interval: u64,
    },

    /// Upstream project trigger
    Upstream {
        /// Upstream project name
        project: String,

        /// Threshold status
        #[serde(default)]
        threshold: String,
    },

    /// Manual trigger
    Manual,
}

impl Trigger {
    /// Creates a cron trigger
    pub fn cron(expression: impl Into<String>) -> Self {
        Self::CronSchedule {
            expression: expression.into(),
            timezone: None,
        }
    }

    /// Creates a cron trigger with timezone
    pub fn cron_with_timezone(expression: impl Into<String>, timezone: impl Into<String>) -> Self {
        Self::CronSchedule {
            expression: expression.into(),
            timezone: Some(timezone.into()),
        }
    }

    /// Creates a poll SCM trigger
    #[must_use]
    pub fn poll_scm(interval_minutes: u64) -> Self {
        Self::PollScm {
            interval: interval_minutes,
        }
    }

    /// Creates an upstream trigger
    #[must_use]
    pub fn upstream(project: impl Into<String>) -> Self {
        Self::Upstream {
            project: project.into(),
            threshold: "SUCCESS".to_string(),
        }
    }

    /// Creates a manual trigger
    #[must_use]
    pub fn manual() -> Self {
        Self::Manual
    }
}

impl super::Validate for Trigger {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        match self {
            Self::CronSchedule { expression, .. } => {
                if expression.is_empty() {
                    return Err(ValidationError::InvalidCronExpression(
                        "Cron expression cannot be empty".to_string(),
                    ));
                }
                // Basic validation: 5 parts separated by spaces
                let parts: Vec<&str> = expression.split(' ').collect();
                if parts.len() != 5 && parts.len() != 6 {
                    return Err(ValidationError::InvalidCronExpression(format!(
                        "Invalid cron expression: {expression}"
                    )));
                }
                Ok(())
            }
            Self::PollScm { interval } => {
                if *interval == 0 {
                    return Err(ValidationError::InvalidCronExpression(
                        "Poll interval must be positive".to_string(),
                    ));
                }
                Ok(())
            }
            Self::Upstream { project, .. } => {
                if project.is_empty() {
                    return Err(ValidationError::InvalidAgentType(
                        "Upstream project cannot be empty".to_string(),
                    ));
                }
                Ok(())
            }
            Self::Manual => Ok(()),
        }
    }
}

/// Pipeline configuration options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PipelineOptions {
    /// Global timeout for the entire pipeline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,

    /// Number of retries for the entire pipeline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<usize>,

    /// Skip default checkout
    #[serde(default)]
    pub skip_default_checkout: bool,

    /// Build discarder configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_discarder: Option<BuildDiscarder>,
}

/// Build discarder configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BuildDiscarder {
    /// Number of builds to keep
    pub num_to_keep: usize,

    /// Number of days to keep builds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_to_keep: Option<usize>,
}

impl BuildDiscarder {
    /// Creates new build discarder
    #[must_use]
    pub fn new(num_to_keep: usize) -> Self {
        Self {
            num_to_keep,
            days_to_keep: None,
        }
    }

    /// Sets days to keep
    #[must_use]
    #[allow(clippy::return_self_not_must_use)]
    pub fn with_days_to_keep(mut self, days: usize) -> Self {
        self.days_to_keep = Some(days);
        self
    }
}

/// Helper for validated duration values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidateDuration(Duration);

impl ValidateDuration {
    /// Creates a validated duration from minutes
    #[must_use]
    pub fn minutes(minutes: u64) -> Self {
        Self(Duration::from_secs(minutes * 60))
    }

    /// Creates a validated duration from seconds
    #[must_use]
    pub fn seconds(seconds: u64) -> Self {
        Self(Duration::from_secs(seconds))
    }

    /// Returns the underlying duration
    #[must_use]
    pub fn as_duration(&self) -> Duration {
        self.0
    }
}

impl From<ValidateDuration> for Duration {
    fn from(val: ValidateDuration) -> Self {
        val.0
    }
}

impl super::Validate for PipelineOptions {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        if let Some(timeout) = self.timeout
            && timeout.as_secs() == 0
        {
            return Err(ValidationError::InvalidTimeout { value: 0 });
        }

        if let Some(retry) = self.retry
            && retry == 0
        {
            return Err(ValidationError::InvalidRetryCount { value: retry });
        }

        Ok(())
    }
}

impl PipelineOptions {
    /// Creates new pipeline options
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets global timeout
    #[must_use]
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Sets retry count
    #[must_use]
    pub fn with_retry(mut self, count: usize) -> Self {
        self.retry = Some(count);
        self
    }

    /// Sets skip default checkout
    #[must_use]
    pub fn with_skip_default_checkout(mut self, skip: bool) -> Self {
        self.skip_default_checkout = skip;
        self
    }

    /// Sets build discarder
    #[must_use]
    pub fn with_build_discarder(mut self, discarder: BuildDiscarder) -> Self {
        self.build_discarder = Some(discarder);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::Validate;

    #[test]
    fn test_trigger_cron() {
        let trigger = Trigger::cron("H/15 * * * *");
        assert!(matches!(trigger, Trigger::CronSchedule { .. }));
        assert!(trigger.validate().is_ok());
    }

    #[test]
    fn test_trigger_cron_invalid() {
        let trigger = Trigger::cron("");
        assert!(trigger.validate().is_err());
    }

    #[test]
    fn test_trigger_poll_scm() {
        let trigger = Trigger::poll_scm(15);
        assert!(matches!(trigger, Trigger::PollScm { interval: 15 }));
        assert!(trigger.validate().is_ok());
    }

    #[test]
    fn test_trigger_upstream() {
        let trigger = Trigger::upstream("upstream-project");
        assert!(matches!(trigger, Trigger::Upstream { .. }));
        assert!(trigger.validate().is_ok());
    }

    #[test]
    fn test_trigger_manual() {
        let trigger = Trigger::manual();
        assert!(matches!(trigger, Trigger::Manual));
        assert!(trigger.validate().is_ok());
    }

    #[test]
    fn test_pipeline_options_default() {
        let options = PipelineOptions::default();
        assert!(options.timeout.is_none());
        assert!(options.retry.is_none());
        assert!(!options.skip_default_checkout);
        assert!(options.build_discarder.is_none());
    }

    #[test]
    fn test_pipeline_options_with_timeout() {
        let options = PipelineOptions::new().with_timeout(Duration::from_secs(600));
        assert_eq!(options.timeout, Some(Duration::from_secs(600)));
        assert!(options.validate().is_ok());
    }

    #[test]
    fn test_pipeline_options_invalid_timeout() {
        let options = PipelineOptions::new().with_timeout(Duration::from_secs(0));
        assert!(options.validate().is_err());
    }

    #[test]
    fn test_pipeline_options_with_retry() {
        let options = PipelineOptions::new().with_retry(3);
        assert_eq!(options.retry, Some(3));
        assert!(options.validate().is_ok());
    }

    #[test]
    fn test_pipeline_options_invalid_retry() {
        let options = PipelineOptions::new().with_retry(0);
        assert!(options.validate().is_err());
    }

    #[test]
    fn test_build_discarder() {
        let discarder = BuildDiscarder::new(10).with_days_to_keep(30);
        assert_eq!(discarder.num_to_keep, 10);
        assert_eq!(discarder.days_to_keep, Some(30));
    }
}
