//! Execution context for pipeline execution.
//!
//! This module provides the execution context that tracks state
//! during pipeline execution.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use pipeliner_core::{Environment, VariableResolver};

/// Execution configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Working directory for execution
    pub working_dir: PathBuf,
    /// Environment variables to set
    pub environment: Environment,
    /// Timeout for the entire execution
    pub global_timeout: Option<std::time::Duration>,
    /// Retry configuration
    pub retry_on_failure: bool,
    /// Maximum retries
    pub max_retries: usize,
    /// Retry delay
    pub retry_delay: std::time::Duration,
    /// Cleanup on exit
    pub cleanup: bool,
    /// ANSI colors in output
    pub colors: bool,
    /// Quiet mode (less output)
    pub quiet: bool,
    /// Write results to file
    pub output_file: Option<PathBuf>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            environment: Environment::new(),
            global_timeout: None,
            retry_on_failure: false,
            max_retries: 0,
            retry_delay: std::time::Duration::from_secs(1),
            cleanup: true,
            colors: std::io::IsTerminal::is_terminal(&std::io::stdout()),
            quiet: false,
            output_file: None,
        }
    }
}

/// Execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Unique execution ID
    pub execution_id: Uuid,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// Current working directory
    pub working_dir: PathBuf,
    /// Environment variables
    pub environment: Environment,
    /// Stashed files
    pub stashes: Arc<Mutex<HashMap<String, PathBuf>>>,
    /// Current stage
    pub current_stage: Option<String>,
    /// Current step
    pub current_step: Option<String>,
    /// Stage results
    pub stage_results: Arc<Mutex<Vec<StageResult>>>,
    /// Step results
    pub step_results: Arc<Mutex<Vec<StepResult>>>,
    /// Nested directory stack
    pub dir_stack: Vec<PathBuf>,
    /// Build parameters
    pub parameters: HashMap<String, String>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionContext {
    /// Creates a new execution context
    #[must_use]
    pub fn new() -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            start_time: Utc::now(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            environment: Environment::new(),
            stashes: Arc::new(Mutex::new(HashMap::new())),
            current_stage: None,
            current_step: None,
            stage_results: Arc::new(Mutex::new(Vec::new())),
            step_results: Arc::new(Mutex::new(Vec::new())),
            dir_stack: Vec::new(),
            parameters: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Creates a new context with a specific working directory
    #[must_use]
    pub fn with_working_dir(path: PathBuf) -> Self {
        let mut ctx = Self::new();
        ctx.working_dir = path;
        ctx
    }

    /// Pushes a directory onto the stack
    pub fn push_dir(&mut self, path: PathBuf) {
        self.dir_stack.push(self.working_dir.clone());
        self.working_dir = path;
    }

    /// Pops a directory from the stack
    pub fn pop_dir(&mut self) -> Option<PathBuf> {
        if let Some(prev) = self.dir_stack.pop() {
            self.working_dir = prev;
            Some(self.working_dir.clone())
        } else {
            None
        }
    }

    /// Gets the current directory
    #[must_use]
    pub fn cwd(&self) -> &PathBuf {
        &self.working_dir
    }

    /// Stashes files for later use
    pub async fn stash(&self, name: impl Into<String>, path: PathBuf) {
        let mut stashes = self.stashes.lock().await;
        stashes.insert(name.into(), path);
    }

    /// Unstashes files
    pub async fn unstash(&self, name: impl Into<String>) -> Option<PathBuf> {
        let mut stashes = self.stashes.lock().await;
        stashes.remove(&name.into())
    }

    /// Checks if a stash exists
    #[must_use]
    pub async fn has_stash(&self, name: impl Into<String>) -> bool {
        let stashes = self.stashes.lock().await;
        stashes.contains_key(&name.into())
    }

    /// Records a stage result
    pub async fn record_stage_result(&self, result: StageResult) {
        let mut results = self.stage_results.lock().await;
        results.push(result);
    }

    /// Records a step result
    pub async fn record_step_result(&self, result: StepResult) {
        let mut results = self.step_results.lock().await;
        results.push(result);
    }

    /// Sets a build parameter
    pub fn set_parameter(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.parameters.insert(name.into(), value.into());
    }

    /// Gets a build parameter
    #[must_use]
    pub fn get_parameter(&self, name: &str) -> Option<&str> {
        self.parameters.get(name).map(|s| s.as_str())
    }

    /// Sets custom metadata
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Gets custom metadata
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Sets the current stage
    pub fn set_current_stage(&mut self, name: impl Into<String>) {
        self.current_stage = Some(name.into());
    }

    /// Sets the current step
    pub fn set_current_step(&mut self, name: impl Into<String>) {
        self.current_step = Some(name.into());
    }

    /// Clears the current stage
    pub fn clear_current_stage(&mut self) {
        self.current_stage = None;
    }

    /// Clears the current step
    pub fn clear_current_step(&mut self) {
        self.current_step = None;
    }
}

impl VariableResolver for ExecutionContext {
    fn resolve(&self, name: &str) -> Option<String> {
        // Check parameters first
        if let Some(value) = self.parameters.get(name) {
            return Some(value.clone());
        }
        // Then check environment
        self.environment.resolve(name)
    }
}

/// Stage execution result
#[derive(Debug, Clone)]
pub struct StageResult {
    pub name: String,
    pub status: crate::ExecutionStatus,
    pub duration: chrono::Duration,
    pub error: Option<String>,
}

/// Step execution result
#[derive(Debug, Clone)]
pub struct StepResult {
    pub stage: String,
    pub name: Option<String>,
    pub status: crate::ExecutionStatus,
    pub duration: chrono::Duration,
    pub error: Option<String>,
    pub output: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_execution_context_creation() {
        let ctx = ExecutionContext::new();
        assert!(!ctx.execution_id.to_string().is_empty());
        assert!(ctx.current_stage.is_none());
        assert!(ctx.current_step.is_none());
    }

    #[test]
    fn test_execution_context_push_pop_dir() {
        let mut ctx = ExecutionContext::new();
        let original = ctx.working_dir.clone();
        let new_dir = PathBuf::from("/tmp/test");

        ctx.push_dir(new_dir.clone());
        assert_eq!(ctx.working_dir, new_dir);

        ctx.pop_dir();
        assert_eq!(ctx.working_dir, original);
    }

    #[test]
    fn test_execution_context_parameters() {
        let mut ctx = ExecutionContext::new();
        ctx.set_parameter("VERSION", "1.0.0");
        assert_eq!(ctx.get_parameter("VERSION"), Some("1.0.0"));
        assert_eq!(ctx.get_parameter("UNKNOWN"), None);
    }

    #[tokio::test]
    async fn test_execution_context_stash() {
        let ctx = ExecutionContext::new();
        ctx.stash("test-stash", PathBuf::from("/tmp/test")).await;
        assert!(ctx.has_stash("test-stash").await);

        let path = ctx.unstash("test-stash").await;
        assert!(path.is_some());
        assert!(ctx.has_stash("test-stash").await == false);
    }

    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();
        assert!(config.cleanup);
        assert!(!config.quiet);
        assert_eq!(config.max_retries, 0);
    }
}
