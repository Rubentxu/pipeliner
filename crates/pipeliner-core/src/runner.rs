//! Pipeline runner for programmatic execution.
//!
//! This module provides the `PipelineRunner` struct which is the primary
//! API for executing pipelines both from the CLI and from library code.
//!
//! ## Example
//!
//! ```rust
//! use pipeliner_core::prelude::*;
//! use pipeliner_core::runner::PipelineRunner;
//!
//! let pipeline = Pipeline::new()
//!     .with_name("My Pipeline")
//!     .with_stage(
//!         Stage::new("Build")
//!             .with_step(Step::shell("echo hello").with_name("greet")),
//!     );
//!
//! let mut runner = PipelineRunner::new();
//! let result = runner.run_blocking(&pipeline);
//! assert!(result.is_ok());
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tracing::{debug, info};

use crate::config::PipelineConfig;
use crate::input::PipelineInput;
use crate::logging::LogLevel;
use crate::registry::StepRegistry;
use crate::validation::Validate;
use crate::Pipeline;
use crate::Stage;
use crate::Step;

/// Configuration for the pipeline runner.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Log level for pipeline execution
    pub log_level: LogLevel,
    /// Cache mode: "full", "deps", or "none"
    pub cache_mode: String,
    /// Whether to clear cache before running
    pub clear_cache: bool,
    /// Whether to force recompilation (script steps)
    pub force_compile: bool,
    /// Environment name (development, staging, production)
    pub environment: Option<String>,
    /// Optional pipeline configuration
    pub pipeline_config: Option<PipelineConfig>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            cache_mode: "full".to_string(),
            clear_cache: false,
            force_compile: false,
            environment: None,
            pipeline_config: None,
        }
    }
}

impl RunnerConfig {
    /// Creates a new default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the log level.
    #[must_use]
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    /// Sets the environment name.
    #[must_use]
    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = Some(env.into());
        self
    }

    /// Sets the pipeline configuration.
    #[must_use]
    pub fn with_pipeline_config(mut self, config: PipelineConfig) -> Self {
        self.pipeline_config = Some(config);
        self
    }
}

/// Event callback type for stage start.
pub type StageStartCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Event callback type for stage completion.
pub type StageCompleteCallback = Arc<dyn Fn(&str, u64) + Send + Sync>;

/// Event callback type for step start.
pub type StepStartCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Pipeline runner for programmatic execution.
///
/// `PipelineRunner` is the primary API for executing pipelines from both
/// CLI and library code. It coordinates the pipeline lifecycle, manages
/// the step registry, and provides event callbacks.
///
/// # Example
///
/// ```rust
/// use pipeliner_core::prelude::*;
/// use pipeliner_core::runner::PipelineRunner;
///
/// let pipeline = Pipeline::new()
///     .with_name("Test Pipeline")
///     .with_stage(Stage::new("Build").with_step(Step::echo("hello")));
///
/// let mut runner = PipelineRunner::new();
/// let result = runner.run_blocking(&pipeline);
/// assert!(result.is_ok());
/// ```
pub struct PipelineRunner {
    /// Step registry for custom step factories
    registry: StepRegistry,
    /// Runner configuration
    config: RunnerConfig,
    /// Callback for stage start events
    on_stage_start: Option<StageStartCallback>,
    /// Callback for stage completion events
    on_stage_complete: Option<StageCompleteCallback>,
    /// Callback for step start events
    on_step_start: Option<StepStartCallback>,
}

impl std::fmt::Debug for PipelineRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineRunner")
            .field("registry", &self.registry)
            .field("config", &self.config)
            .field("has_stage_start_cb", &self.on_stage_start.is_some())
            .field("has_stage_complete_cb", &self.on_stage_complete.is_some())
            .field("has_step_start_cb", &self.on_step_start.is_some())
            .finish()
    }
}

impl Default for PipelineRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineRunner {
    /// Creates a new pipeline runner with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: StepRegistry::new(),
            config: RunnerConfig::default(),
            on_stage_start: None,
            on_stage_complete: None,
            on_step_start: None,
        }
    }

    /// Creates a runner with a custom step registry.
    #[must_use]
    pub fn with_registry(registry: StepRegistry) -> Self {
        Self {
            registry,
            config: RunnerConfig::default(),
            on_stage_start: None,
            on_stage_complete: None,
            on_step_start: None,
        }
    }

    /// Sets the runner configuration.
    #[must_use]
    pub fn with_config(mut self, config: RunnerConfig) -> Self {
        self.config = config;
        self
    }

    /// Registers a callback for stage start events.
    #[must_use]
    pub fn on_stage_start<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_stage_start = Some(Arc::new(f));
        self
    }

    /// Registers a callback for stage completion events.
    #[must_use]
    pub fn on_stage_complete<F>(mut self, f: F) -> Self
    where
        F: Fn(&str, u64) + Send + Sync + 'static,
    {
        self.on_stage_complete = Some(Arc::new(f));
        self
    }

    /// Registers a callback for step start events.
    #[must_use]
    pub fn on_step_start<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_step_start = Some(Arc::new(f));
        self
    }

    /// Sets the log level.
    #[must_use]
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.config.log_level = level;
        self
    }

    /// Runs a pipeline synchronously (blocking).
    ///
    /// This method blocks the current thread until the pipeline completes.
    /// For async execution, use [`run_async`](Self::run_async).
    ///
    /// # Errors
    ///
    /// Returns a `RuntimeError` if the pipeline fails validation or
    /// if a phase execution fails.
    pub fn run_blocking(&mut self, pipeline: &Pipeline) -> Result<crate::runtime::PipelineRunResult, crate::runtime::RuntimeError> {
        // Validate the pipeline first
        pipeline.validate().map_err(|e| crate::runtime::RuntimeError::ConfigError(e.to_string()))?;

        let start = Instant::now();
        let min_level = self.config.log_level;

        info!("========================================");
        info!("   Pipeliner - Pipeline Execution");
        info!("========================================");
        info!("Pipeline: {:?}", pipeline.name());
        info!("Stages: {}", pipeline.stages.len());

        let mut stages_executed = 0;
        let mut steps_executed = 0;
        let mut all_success = true;

        for stage_or_parallel in &pipeline.stages {
            match stage_or_parallel {
                crate::pipeline::StageOrParallel::Stage(stage) => {
                    let stage_start = Instant::now();
                    let stage_name = &stage.name;

                    // Fire stage start callback
                    if let Some(ref callback) = self.on_stage_start {
                        callback(stage_name);
                    }

                    info!("[Stage] {}", stage_name);
                    info!("----------------------------------------");

                    let mut stage_success = true;
                    for step in &stage.steps {
                        // Fire step start callback
                        let step_name = step.name.as_deref().unwrap_or("unnamed");
                        if let Some(ref callback) = self.on_step_start {
                            callback(step_name);
                        }

                        // TODO: Execute step via executor (currently just tracking)
                        debug!("[Step] {} (type: {:?})", step_name, step.step_type);
                        steps_executed += 1;
                    }

                    if stage_success {
                        let duration_ms = stage_start.elapsed().as_millis() as u64;
                        // Fire stage complete callback
                        if let Some(ref callback) = self.on_stage_complete {
                            callback(stage_name, duration_ms);
                        }
                    } else {
                        all_success = false;
                    }

                    stages_executed += 1;
                }
                crate::pipeline::StageOrParallel::Parallel(group) => {
                    let group_name = group.name.as_deref().unwrap_or("parallel");
                    info!("[Parallel] {}", group_name);
                    info!("----------------------------------------");
                    
                    // Execute stages in parallel (sequential for now, true parallelism needs executor)
                    for stage in &group.stages {
                        let stage_start = Instant::now();
                        let stage_name = &stage.name;

                        if let Some(ref callback) = self.on_stage_start {
                            callback(stage_name);
                        }

                        info!("[Parallel Stage] {}", stage_name);

                        for step in &stage.steps {
                            let step_name = step.name.as_deref().unwrap_or("unnamed");
                            if let Some(ref callback) = self.on_step_start {
                                callback(step_name);
                            }
                            debug!("[Step] {} (type: {:?})", step_name, step.step_type);
                            steps_executed += 1;
                        }

                        let duration_ms = stage_start.elapsed().as_millis() as u64;
                        if let Some(ref callback) = self.on_stage_complete {
                            callback(stage_name, duration_ms);
                        }

                        stages_executed += 1;
                    }
                }
            }
        }

        let total_duration = start.elapsed().as_millis() as u64;

        info!("========================================");
        info!("   Execution Complete");
        info!("========================================");
        info!("Stages: {}/{} completed", stages_executed, pipeline.stages.len());
        info!("Total time: {}ms", total_duration);

        Ok(crate::runtime::PipelineRunResult::success(
            stages_executed,
            steps_executed,
            total_duration,
        ))
    }

    /// Runs a pipeline asynchronously.
    ///
    /// This method executes the pipeline without blocking the current thread.
    /// For synchronous execution, use [`run_blocking`](Self::run_blocking).
    ///
    /// # Errors
    ///
    /// Returns a `RuntimeError` if the pipeline fails validation or
    /// if a phase execution fails.
    pub async fn run_async(&mut self, pipeline: &Pipeline) -> Result<crate::runtime::PipelineRunResult, crate::runtime::RuntimeError> {
        // Delegate to blocking implementation for now
        // TODO: Implement true async execution with tokio tasks
        self.run_blocking(pipeline)
    }

    /// Runs a pipeline from a file (blocking).
    ///
    /// Detects the input format from the file extension (.rs, .json, .toml)
    /// and parses accordingly. For `.rs` files, returns an error indicating
    /// that the `pipeliner-script` crate is required for Rust script execution.
    ///
    /// # Errors
    ///
    /// Returns `RuntimeError::ConfigError` if the file cannot be read, parsed,
    /// validation fails, or if the file format requires the script engine.
    pub fn run_file_blocking(&mut self, path: &std::path::Path) -> Result<crate::runtime::PipelineRunResult, crate::runtime::RuntimeError> {
        let input = PipelineInput::detect(path)
            .map_err(|e| crate::runtime::RuntimeError::ConfigError(e.to_string()))?;

        // Check if this input requires the script engine
        if input.requires_script_engine() {
            return Err(crate::runtime::RuntimeError::ConfigError(
                "Rust script execution requires the pipeliner-script crate".to_string(),
            ));
        }

        let pipeline = input
            .parse()
            .map_err(|e| crate::runtime::RuntimeError::ConfigError(e.to_string()))?;

        self.run_blocking(&pipeline)
    }

    /// Runs a pipeline from a file (async).
    ///
    /// # Errors
    ///
    /// Returns a `RuntimeError` if the file cannot be read, parsed,
    /// or if the pipeline execution fails.
    pub async fn run_file_async(&mut self, path: &std::path::Path) -> Result<crate::runtime::PipelineRunResult, crate::runtime::RuntimeError> {
        self.run_file_blocking(path)
    }

    /// Returns a reference to the step registry.
    #[must_use]
    pub fn registry(&self) -> &StepRegistry {
        &self.registry
    }

    /// Returns a mutable reference to the step registry.
    pub fn registry_mut(&mut self) -> &mut StepRegistry {
        &mut self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{Step, StepType};

    fn create_test_pipeline() -> Pipeline {
        Pipeline::new()
            .with_name("Test Pipeline")
            .with_stage(
                Stage::new("Build")
                    .with_step(Step::echo("Building...").with_name("build-echo")),
            )
    }

    fn create_multi_stage_pipeline() -> Pipeline {
        Pipeline::new()
            .with_name("Multi-Stage Pipeline")
            .with_stage(
                Stage::new("Build")
                    .with_step(Step::echo("Building...").with_name("build")),
            )
            .with_stage(
                Stage::new("Test")
                    .with_step(Step::echo("Testing...").with_name("test")),
            )
    }

    // =======================================================================
    // RunnerConfig Tests
    // =======================================================================

    #[test]
    fn test_runner_config_default() {
        let config = RunnerConfig::default();
        assert_eq!(config.cache_mode, "full");
        assert!(!config.clear_cache);
        assert!(!config.force_compile);
        assert!(config.environment.is_none());
    }

    #[test]
    fn test_runner_config_builder() {
        let config = RunnerConfig::new()
            .with_log_level(LogLevel::Debug)
            .with_environment("production");

        assert_eq!(config.log_level, LogLevel::Debug);
        assert_eq!(config.environment.as_deref(), Some("production"));
    }

    // =======================================================================
    // PipelineRunner::new() Tests
    // =======================================================================

    #[test]
    fn test_pipeline_runner_new() {
        let runner = PipelineRunner::new();
        assert!(runner.registry().is_empty());
    }

    #[test]
    fn test_pipeline_runner_default() {
        let runner = PipelineRunner::default();
        assert!(runner.registry().is_empty());
    }

    #[test]
    fn test_pipeline_runner_with_config() {
        let config = RunnerConfig::new()
            .with_log_level(LogLevel::Warn)
            .with_environment("staging");

        let runner = PipelineRunner::new().with_config(config);
        assert_eq!(runner.config.log_level, LogLevel::Warn);
        assert_eq!(runner.config.environment.as_deref(), Some("staging"));
    }

    // =======================================================================
    // PipelineRunner::run_blocking() Tests
    // =======================================================================

    #[test]
    fn test_pipeline_runner_run_blocking_simple() {
        let pipeline = create_test_pipeline();
        let mut runner = PipelineRunner::new();
        let result = runner.run_blocking(&pipeline);

        assert!(result.is_ok());
        let run_result = result.unwrap();
        assert!(run_result.success);
        assert_eq!(run_result.stages_executed, 1);
        assert_eq!(run_result.steps_executed, 1);
        // Duration may be 0 for very fast execution
        assert!(run_result.error.is_none());
    }

    #[test]
    fn test_pipeline_runner_run_blocking_multi_stage() {
        let pipeline = create_multi_stage_pipeline();
        let mut runner = PipelineRunner::new();
        let result = runner.run_blocking(&pipeline);

        assert!(result.is_ok());
        let run_result = result.unwrap();
        assert!(run_result.success);
        assert_eq!(run_result.stages_executed, 2);
        assert_eq!(run_result.steps_executed, 2);
    }

    #[test]
    fn test_pipeline_runner_run_blocking_empty_pipeline() {
        let pipeline = Pipeline::new(); // Empty - should fail validation
        let mut runner = PipelineRunner::new();
        let result = runner.run_blocking(&pipeline);

        assert!(result.is_err());
    }

    // =======================================================================
    // Event Callbacks Tests
    // =======================================================================

    #[test]
    fn test_pipeline_runner_on_stage_start() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let pipeline = create_test_pipeline();
        let stages_seen = Arc::new(AtomicUsize::new(0));

        let counter = stages_seen.clone();
        let mut runner = PipelineRunner::new()
            .on_stage_start(move |name| {
                let _ = name; // Use the name
                counter.fetch_add(1, Ordering::SeqCst);
            });

        let result = runner.run_blocking(&pipeline);
        assert!(result.is_ok());
        assert_eq!(stages_seen.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_pipeline_runner_on_stage_complete() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let pipeline = create_test_pipeline();
        let completions = Arc::new(AtomicUsize::new(0));

        let counter = completions.clone();
        let mut runner = PipelineRunner::new()
            .on_stage_complete(move |_name, _duration| {
                counter.fetch_add(1, Ordering::SeqCst);
            });

        let result = runner.run_blocking(&pipeline);
        assert!(result.is_ok());
        assert_eq!(completions.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_pipeline_runner_on_step_start() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let pipeline = create_multi_stage_pipeline();
        let steps_seen = Arc::new(AtomicUsize::new(0));

        let counter = steps_seen.clone();
        let mut runner = PipelineRunner::new()
            .on_step_start(move |_name| {
                counter.fetch_add(1, Ordering::SeqCst);
            });

        let result = runner.run_blocking(&pipeline);
        assert!(result.is_ok());
        assert_eq!(steps_seen.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_pipeline_runner_multiple_callbacks() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let pipeline = create_multi_stage_pipeline();
        let stages = Arc::new(AtomicUsize::new(0));
        let steps = Arc::new(AtomicUsize::new(0));

        let stages_counter = stages.clone();
        let steps_counter = steps.clone();
        let mut runner = PipelineRunner::new()
            .on_stage_start(move |_name| {
                stages_counter.fetch_add(1, Ordering::SeqCst);
            })
            .on_step_start(move |_name| {
                steps_counter.fetch_add(1, Ordering::SeqCst);
            });

        let result = runner.run_blocking(&pipeline);
        assert!(result.is_ok());
        assert_eq!(stages.load(Ordering::SeqCst), 2);
        assert_eq!(steps.load(Ordering::SeqCst), 2);
    }

    // =======================================================================
    // PipelineRunner::run_file_blocking() Tests
    // =======================================================================

    #[test]
    fn test_pipeline_runner_run_file_requires_script_engine() {
        let mut runner = PipelineRunner::new();
        let path = std::path::Path::new("test-pipeline.rs");
        let result = runner.run_file_blocking(path);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::runtime::RuntimeError::ConfigError(msg) => {
                assert!(msg.contains("pipeliner-script"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    // =======================================================================
    // Registry Tests
    // =======================================================================

    #[test]
    fn test_pipeline_runner_with_registry() {
        let mut registry = StepRegistry::new();
        // Registry starts empty
        assert!(registry.is_empty());

        let runner = PipelineRunner::with_registry(registry);
        assert!(runner.registry().is_empty());
    }

    #[test]
    fn test_pipeline_runner_registry_mut() {
        let mut runner = PipelineRunner::new();
        assert!(runner.registry().is_empty());

        // Can access mutable registry
        let registry = runner.registry_mut();
        assert!(registry.is_empty());
    }

    // =======================================================================
    // PipelineRunner::run_file_blocking() Integration Tests
    // =======================================================================
}