//! Pipeline options for execution control.
//!
//! This module provides types for configuring pipeline execution
//! options like timeouts, retries, and triggers.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Pipeline execution options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PipelineOptions {
    /// Timeout for the entire pipeline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Timeout>,
    /// Retry configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<Retry>,
    /// Build discarder configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discarder: Option<BuildDiscarder>,
    /// Quiet period
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet_period: Option<Duration>,
    /// Block downstream jobs
    #[serde(default)]
    pub block_downstream: bool,
    /// Block upstream jobs
    #[serde(default)]
    pub block_upstream: bool,
    /// Skip default checkout
    #[serde(default)]
    pub skip_default_checkout: bool,
    /// Checkout to subdirectory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_dir: Option<String>,
}

/// Timeout configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Timeout {
    /// Fixed duration timeout
    Duration {
        /// Maximum duration
        duration: Duration,
    },
    /// Timeout with activity check
    Activity {
        /// Maximum duration
        duration: Duration,
    },
}

/// Retry configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Retry {
    /// Fixed number of retries
    Count(usize),
    /// Retry with delay
    CountWithDelay {
        /// Number of attempts
        count: usize,
        /// Delay between retries
        delay: Duration,
    },
}

/// Build discarder configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildDiscarder {
    /// Maximum number of builds to keep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_builds: Option<u32>,
    /// Days to keep builds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_to_keep: Option<u32>,
    /// Days to keep successful builds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_to_keep_success: Option<u32>,
    /// Days to keep failed builds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_to_keep_failure: Option<u32>,
    /// Builds to keep per branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_branch: Option<u32>,
}

/// Pipeline trigger types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Trigger {
    /// Cron-based scheduling
    Cron {
        /// Cron expression
        expression: String,
        /// Accept on days
        #[serde(default)]
        accept_on_days: Vec<String>,
        /// Filter expression
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<TriggerFilter>,
    },
    /// Poll SCM for changes
    PollScm {
        /// Cron expression for polling
        expression: String,
        /// Filter expression
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<TriggerFilter>,
    },
    /// Upstream job trigger
    Upstream {
        /// Jobs to watch
        jobs: Vec<String>,
        /// Threshold for triggering
        #[serde(default = "default_threshold")]
        threshold: String,
    },
    /// Remote trigger via API
    Remote {
        /// Token for authentication
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
    /// GitHub hook trigger
    GithubPush {
        /// Repository filter
        #[serde(skip_serializing_if = "Option::is_none")]
        repository: Option<String>,
    },
    /// GitLab merge request trigger
    GitlabMergeRequest {
        /// Trigger on open
        #[serde(default)]
        on_open: bool,
        /// Trigger on update
        #[serde(default)]
        on_update: bool,
        /// Trigger on close
        #[serde(default)]
        on_close: bool,
        /// Filter by target branch
        #[serde(skip_serializing_if = "Option::is_none")]
        target_branch: Option<String>,
    },
    /// Bitbucket webhook trigger
    BitbucketPush {
        /// Repository filter
        #[serde(skip_serializing_if = "Option::is_none")]
        repository: Option<String>,
    },
}

fn default_threshold() -> String {
    "SUCCESS".to_string()
}

/// Trigger filter for SCM polling
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerFilter {
    /// Include patterns
    #[serde(default)]
    pub include: Vec<String>,
    /// Exclude patterns
    #[serde(default)]
    pub exclude: Vec<String>,
    /// File filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_paths: Option<FileFilter>,
}

/// File filter for triggers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FileFilter {
    /// Include specific paths
    Include(Vec<String>),
    /// Exclude specific paths
    Exclude(Vec<String>),
}

/// Stage-specific options
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StageOptions {
    /// Timeout for this stage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Timeout>,
    /// Retry count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<Retry>,
    /// Skip default checkout
    #[serde(default)]
    pub skip_default_checkout: bool,
    /// Fail fast
    #[serde(default)]
    pub fail_fast: bool,
    /// Continue on failure
    #[serde(default)]
    pub continue_on_failure: bool,
    /// Return current build status on failure
    #[serde(default)]
    pub return_current_build_status: bool,
}

impl PipelineOptions {
    /// Creates a new empty options
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the timeout
    #[must_use]
    pub fn with_timeout(mut self, timeout: Timeout) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the retry configuration
    #[must_use]
    pub fn with_retry(mut self, retry: Retry) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Sets the build discarder
    #[must_use]
    pub fn with_discarder(mut self, discarder: BuildDiscarder) -> Self {
        self.discarder = Some(discarder);
        self
    }

    /// Blocks downstream jobs
    #[must_use]
    pub fn block_downstream(mut self) -> Self {
        self.block_downstream = true;
        self
    }

    /// Blocks upstream jobs
    #[must_use]
    pub fn block_upstream(mut self) -> Self {
        self.block_upstream = true;
        self
    }
}

impl Timeout {
    /// Creates a duration timeout
    #[must_use]
    pub fn duration(seconds: u64) -> Self {
        Self::Duration {
            duration: Duration::from_secs(seconds),
        }
    }

    /// Creates an activity timeout
    #[must_use]
    pub fn activity(seconds: u64) -> Self {
        Self::Activity {
            duration: Duration::from_secs(seconds),
        }
    }

    /// Returns the duration
    #[must_use]
    pub fn duration_value(&self) -> Duration {
        match self {
            Timeout::Duration { duration } => *duration,
            Timeout::Activity { duration } => *duration,
        }
    }
}

impl Retry {
    /// Creates a retry with count
    #[must_use]
    pub fn count(count: usize) -> Self {
        Self::Count(count)
    }

    /// Creates a retry with count and delay
    #[must_use]
    pub fn count_with_delay(count: usize, seconds: u64) -> Self {
        Self::CountWithDelay {
            count,
            delay: Duration::from_secs(seconds),
        }
    }

    /// Returns the number of attempts
    #[must_use]
    pub fn attempts(&self) -> usize {
        match self {
            Retry::Count(c) => *c + 1, // Original + retries
            Retry::CountWithDelay { count, .. } => *count + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_options_new() {
        let options = PipelineOptions::new();
        assert!(options.timeout.is_none());
        assert!(options.retry.is_none());
    }

    #[test]
    fn test_pipeline_options_with_timeout() {
        let options = PipelineOptions::new().with_timeout(Timeout::duration(3600));
        assert!(options.timeout.is_some());
    }

    #[test]
    fn test_pipeline_options_with_retry() {
        let options = PipelineOptions::new().with_retry(Retry::count(3));
        assert!(options.retry.is_some());
    }

    #[test]
    fn test_timeout_duration() {
        let timeout = Timeout::duration(1800);
        assert_eq!(timeout.duration_value().as_secs(), 1800);
    }

    #[test]
    fn test_retry_count() {
        let retry = Retry::count(3);
        assert_eq!(retry.attempts(), 4); // 3 retries + 1 original
    }

    #[test]
    fn test_retry_with_delay() {
        let retry = Retry::count_with_delay(3, 60);
        if let Retry::CountWithDelay { count, delay } = retry {
            assert_eq!(count, 3);
            assert_eq!(delay.as_secs(), 60);
        } else {
            panic!("Expected CountWithDelay variant");
        }
    }

    #[test]
    fn test_build_discarder() {
        let discarder = BuildDiscarder {
            max_builds: Some(100),
            days_to_keep: Some(30),
            days_to_keep_success: None,
            days_to_keep_failure: None,
            per_branch: None,
        };
        assert_eq!(discarder.max_builds, Some(100));
    }

    #[test]
    fn test_trigger_cron() {
        let trigger = Trigger::Cron {
            expression: "H * * * *".to_string(),
            accept_on_days: vec!["MON".to_string()],
            filter: None,
        };
        if let Trigger::Cron { expression, .. } = trigger {
            assert_eq!(expression, "H * * * *");
        } else {
            panic!("Expected Cron variant");
        }
    }
}
