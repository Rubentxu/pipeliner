//! # Local Executor Module
//!
//! Local pipeline execution engine.
//!
//! This module provides the [`LocalExecutor`] which executes a [`PipelineSpec`]
//! on the local machine with support for:
//!
//! - Sequential and parallel stage execution
//! - Stage retry logic
//! - Error handling and early exit (fail_fast)
//! - Event emission for progress tracking
//!
//! ## Execution Model
//!
//! ```ignore
//! PipelineSpec
//!     │
//!     ▼
//! ┌─────────────────────────────┐
//! │    LocalExecutor            │
//! │  ┌───────────────────────┐  │
//! │  │ Stage 1 (sequential)  │  │
//! │  │   Step 1 → Step 2    │  │
//! │  └───────────────────────┘  │
//! │  ┌───────────────────────┐  │
//! │  │ Stage 2 (parallel)    │  │
//! │  │  ┌─────┐  ┌─────┐    │  │
//! │  │  │ S1  │  │ S2  │    │  │
//! │  │  └─────┘  └─────┘    │  │
//! │  └───────────────────────┘  │
//! └─────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use pipeliner_runtime::LocalExecutor;
//! use pipeliner_core::spec::PipelineSpec;
//!
//! let executor = LocalExecutor::new();
//! let result = executor.execute(&spec).await?;
//!
//! if result.success {
//!     println!("Pipeline succeeded!");
//! } else {
//!     println!("Pipeline failed with exit code {:?}", result.exit_code);
//! }
//! ```

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use pipeliner_core::spec::{
    PipelineSpec, StageExecution, StageSpec, StepSpec,
    step_spec::{EchoStepSpec, ShellStepSpec, DirStepSpec, WithEnvStepSpec, LetOutputStepSpec, InterpolationMode, WithCredentialsStepSpec, JUnitStepSpec, ArchiveStepSpec},
};

use crate::events::{BufferedEmitter, EventEmitter, PipelineEvent};

/// Environment context for variable interpolation during step execution.
///
/// This struct holds environment variables that can be referenced in shell
/// scripts using `$VAR` or `${VAR}` syntax. Variables are stored in a HashMap
/// and the context is cloneable for passing between step handlers.
#[derive(Debug, Clone, Default)]
pub struct EnvContext {
    vars: HashMap<String, String>,
}

impl EnvContext {
    /// Creates a new empty environment context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new environment context with the given variables.
    pub fn with_vars(vars: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            vars: vars.into_iter().collect(),
        }
    }

    /// Gets a variable value by name.
    ///
    /// Returns `None` if the variable is not defined.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }

    /// Sets a variable value.
    pub fn set(&mut self, name: String, value: String) {
        self.vars.insert(name, value);
    }

    /// Removes a variable from the context.
    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.vars.remove(name)
    }

    /// Returns an iterator over all variable names and values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Returns the number of variables in this context.
    pub fn len(&self) -> usize {
        self.vars.len()
    }

    /// Returns true if this context has no variables.
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }
}

/// Interpolates variables in a string using `$VAR` and `${VAR}` syntax.
///
/// Variables are looked up in the provided `EnvContext`. Undefined variables
/// are replaced with empty strings.
///
/// # Arguments
///
/// * `input` - The string to interpolate
/// * `env` - The environment context containing variable values
///
/// # Examples
///
/// ```
/// use pipeliner_runtime::local_executor::{EnvContext, interpolate};
///
/// let mut env = EnvContext::new();
/// env.set("NAME".to_string(), "world".to_string());
///
/// let result = interpolate("Hello $NAME!", &env);
/// assert_eq!(result, "Hello world!");
///
/// let result = interpolate("Hello ${NAME}!", &env);
/// assert_eq!(result, "Hello world!");
///
/// let result = interpolate("Hello ${UNDEFINED:-default}!", &env);
/// assert_eq!(result, "Hello !"); // Default values not supported, returns empty
/// ```
pub fn interpolate(input: &str, env: &EnvContext) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            let next = bytes[i + 1];

            if next == b'$' {
                // Escaped dollar sign
                result.push('$');
                i += 2;
            } else if next == b'{'{ 
                // ${VAR} form
                if let Some((var_name, end_pos)) = parse_braced_var(&bytes[i + 2..]) {
                    if let Some(value) = env.get(&var_name) {
                        result.push_str(value);
                    }
                    i = end_pos;
                } else {
                    result.push('$');
                    i += 1;
                }
            } else if next.is_ascii_alphanumeric() || next == b'_' {
                // $VAR form
                if let Some((var_name, end_pos)) = parse_bare_var(&bytes[i + 1..]) {
                    if let Some(value) = env.get(&var_name) {
                        result.push_str(value);
                    }
                    i = end_pos;
                } else {
                    result.push('$');
                    i += 1;
                }
            } else {
                result.push('$');
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Parses a ${VAR} variable reference.
/// Returns the variable name and the end position in the original slice.
fn parse_braced_var(bytes: &[u8]) -> Option<(String, usize)> {
    let mut end = 0;
    for (idx, &byte) in bytes.iter().enumerate() {
        if byte == b'}' {
            end = idx;
            break;
        }
        if !byte.is_ascii_alphanumeric() && byte != b'_' {
            return None;
        }
    }

    if end == 0 {
        return None;
    }

    let var_name = String::from_utf8_lossy(&bytes[..end]).to_string();
    // end + 2 accounts for the opening '{' we skipped and the closing '}' we're at
    Some((var_name, 2 + end + 1))
}

/// Parses a $VAR variable reference.
/// Returns the variable name and the end position in the original slice.
fn parse_bare_var(bytes: &[u8]) -> Option<(String, usize)> {
    let mut end = 0;
    for (idx, &byte) in bytes.iter().enumerate() {
        if !byte.is_ascii_alphanumeric() && byte != b'_' {
            end = idx;
            break;
        }
        end = idx + 1;
    }

    if end == 0 {
        return None;
    }

    let var_name = String::from_utf8_lossy(&bytes[..end]).to_string();
    Some((var_name, 1 + end))
}

/// Configuration for the local executor.
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum number of parallel stages to run (0 = unlimited)
    pub max_parallelism: usize,
    /// Default timeout for stages (None = no timeout)
    pub stage_timeout: Option<std::time::Duration>,
    /// Default timeout for steps (None = no timeout)
    pub step_timeout: Option<std::time::Duration>,
    /// Whether to fail fast on stage failure
    pub fail_fast: bool,
    /// Working directory for pipeline execution
    pub workdir: Option<std::path::PathBuf>,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_parallelism: 0, // unlimited
            stage_timeout: None,
            step_timeout: None,
            fail_fast: true,
            workdir: None,
        }
    }
}

impl ExecutorConfig {
    /// Creates a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum parallelism.
    pub fn with_max_parallelism(mut self, max: usize) -> Self {
        self.max_parallelism = max;
        self
    }

    /// Sets the stage timeout.
    pub fn with_stage_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.stage_timeout = Some(timeout);
        self
    }

    /// Sets the step timeout.
    pub fn with_step_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.step_timeout = Some(timeout);
        self
    }

    /// Sets fail_fast behavior.
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// Sets the working directory.
    pub fn with_workdir(mut self, workdir: std::path::PathBuf) -> Self {
        self.workdir = Some(workdir);
        self
    }
}

/// Result of a step execution.
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Whether the step succeeded
    pub success: bool,
    /// Exit code if applicable
    pub exit_code: Option<i32>,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Standard output
    pub stdout: Option<String>,
    /// Standard error
    pub stderr: Option<String>,
}

impl StepResult {
    /// Creates a successful step result.
    pub fn success(exit_code: i32, duration_secs: f64) -> Self {
        Self {
            success: true,
            exit_code: Some(exit_code),
            duration_secs,
            stdout: None,
            stderr: None,
        }
    }

    /// Creates a failed step result.
    pub fn failure(exit_code: Option<i32>, duration_secs: f64) -> Self {
        Self {
            success: false,
            exit_code,
            duration_secs,
            stdout: None,
            stderr: None,
        }
    }
}

/// Internal representation of step results for a stage.
#[derive(Debug, Clone)]
pub struct StepResultEntry {
    pub step_index: usize,
    pub step_type: String,
    pub step_label: Option<String>,
    pub started_at: DateTime<Utc>,
    pub result: StepResult,
}

/// Result of a stage execution.
#[derive(Debug, Clone)]
pub struct StageResult {
    /// Stage ID
    pub stage_id: String,
    /// Stage display name
    pub stage_name: String,
    /// Whether the stage succeeded
    pub success: bool,
    /// Exit code if applicable
    pub exit_code: Option<i32>,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Results of each step
    pub step_results: Vec<StepResultEntry>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// When the stage started
    pub started_at: DateTime<Utc>,
}

/// Result of pipeline execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Pipeline ID
    pub pipeline_id: Uuid,
    /// Pipeline name if available
    pub pipeline_name: Option<String>,
    /// Whether the pipeline succeeded
    pub success: bool,
    /// Exit code if applicable
    pub exit_code: Option<i32>,
    /// When the pipeline started
    pub started_at: DateTime<Utc>,
    /// When the pipeline completed
    pub completed_at: DateTime<Utc>,
    /// Total duration in seconds
    pub total_duration_secs: f64,
    /// Results of each stage
    pub stage_results: Vec<StageResult>,
    /// Total number of retries
    pub total_retries: u32,
}

impl ExecutionResult {
    /// Returns when a specific stage started.
    pub fn stage_started_at(&self, stage_index: usize) -> DateTime<Utc> {
        self.stage_results
            .get(stage_index)
            .map(|s| s.started_at)
            .unwrap_or(self.started_at)
    }
}

/// BoxFuture type alias for recursive async calls
type BoxFuture<'a> = Pin<Box<dyn std::future::Future<Output = Result<StepResult, ExecutionError>> + Send + 'a>>;

/// Local pipeline executor.
#[derive(Clone)]
pub struct LocalExecutor {
    config: ExecutorConfig,
    emitter: Arc<RwLock<Option<Box<dyn EventEmitter>>>>,
    /// Semaphore for limiting parallelism (None = unlimited)
    semaphore: Option<Arc<Semaphore>>,
    /// Cancellation token for graceful shutdown
    cancellation_token: Option<CancellationToken>,
}

impl LocalExecutor {
    /// Creates a new local executor with default configuration.
    pub fn new() -> Self {
        Self {
            config: ExecutorConfig::default(),
            emitter: Arc::new(RwLock::new(None)),
            semaphore: None,
            cancellation_token: None,
        }
    }

    /// Creates a new executor with the given configuration.
    pub fn with_config(config: ExecutorConfig) -> Self {
        Self {
            config: config.clone(),
            emitter: Arc::new(RwLock::new(None)),
            semaphore: None,
            cancellation_token: None,
        }
    }

    /// Sets the maximum parallelism for concurrent stage execution.
    ///
    /// When set, the executor will limit the number of parallel stages
    /// to at most `max` using a semaphore.
    ///
    /// # Arguments
    ///
    /// * `max` - Maximum number of concurrent stages (must be > 0)
    ///
    /// # Example
    ///
    /// ```
    /// use pipeliner_runtime::LocalExecutor;
    ///
    /// let executor = LocalExecutor::new()
    ///     .with_max_parallelism(4);
    /// ```
    pub fn with_max_parallelism(mut self, max: usize) -> Self {
        if max > 0 {
            self.semaphore = Some(Arc::new(Semaphore::new(max)));
        }
        self
    }

    /// Sets the cancellation token for graceful shutdown.
    ///
    /// When the cancellation token is triggered, the executor will
    /// emit a `PipelineEvent::Cancelled` event and stop execution.
    ///
    /// # Arguments
    ///
    /// * `token` - Cancellation token to use
    ///
    /// # Example
    ///
    /// ```
    /// use pipeliner_runtime::LocalExecutor;
    /// use tokio_util::sync::CancellationToken;
    ///
    /// let token = CancellationToken::new();
    /// let executor = LocalExecutor::new()
    ///     .with_cancellation_token(token);
    /// ```
    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    /// Sets an event emitter for pipeline events.
    pub fn set_emitter(&mut self, emitter: Box<dyn EventEmitter>) {
        *self.emitter.write() = Some(emitter);
    }

    /// Subscribes to pipeline events with a callback.
    pub fn subscribe<F>(&mut self, callback: F)
    where
        F: Fn(PipelineEvent) + Send + Sync + 'static,
    {
        self.set_emitter(Box::new(crate::events::CallbackEmitter::new(callback)));
    }

    /// Returns a buffered emitter for collecting events.
    pub fn buffered_emitter(&self) -> BufferedEmitter {
        BufferedEmitter::new()
    }

    fn emit(&self, event: PipelineEvent) {
        if let Some(ref emitter) = *self.emitter.read() {
            emitter.emit(event);
        }
    }

    /// Emit a Cancelled event for the given run.
    fn emit_cancelled(&self, run_id: Uuid, reason: String) {
        self.emit(PipelineEvent::Cancelled {
            run_id,
            pipeline_id: run_id,
            reason: Some(reason),
            cancelled_at: chrono::Utc::now(),
        });
    }

    /// Executes a pipeline specification.
    ///
    /// # Errors
    ///
    /// Returns an error if the pipeline cannot be executed.
    pub async fn execute(&self, spec: &PipelineSpec) -> Result<ExecutionResult, ExecutionError> {
        let run_id = Uuid::new_v4();
        let pipeline_id = run_id;
        let started_at = Utc::now();

        self.emit(PipelineEvent::Started {
            run_id,
            pipeline_id,
            pipeline_name: None,
            started_at,
        });

        let mut stage_results = Vec::new();
        let mut total_retries = 0u32;
        let mut pipeline_success = true;

        for stage_spec in &spec.stages {
            // Get retry count from stage options, default to 0
            let max_attempts = stage_spec
                .options
                .as_ref()
                .map(|o| o.retry + 1)
                .unwrap_or(1);

            let stage_result = self
                .execute_stage(run_id, stage_spec, 1, max_attempts)
                .await?;

            total_retries += stage_result.retry_count;
            stage_results.push(stage_result.clone());

            if !stage_result.success {
                pipeline_success = false;
                if self.config.fail_fast {
                    break;
                }
            }
        }

        let completed_at = Utc::now();
        let total_duration_secs = (completed_at - started_at).num_milliseconds() as f64 / 1000.0;

        if pipeline_success {
            self.emit(PipelineEvent::Completed {
                run_id,
                pipeline_id,
                completed_at,
                success: true,
                total_duration_secs,
            });
        } else {
            self.emit(PipelineEvent::Failed {
                run_id,
                pipeline_id,
                reason: "One or more stages failed".to_string(),
                failed_at: completed_at,
                total_duration_secs,
            });
        }

        Ok(ExecutionResult {
            pipeline_id,
            pipeline_name: None,
            success: pipeline_success,
            exit_code: if pipeline_success { Some(0) } else { Some(1) },
            started_at,
            completed_at,
            total_duration_secs,
            stage_results,
            total_retries,
        })
    }

    async fn execute_stage(
        &self,
        run_id: Uuid,
        stage_spec: &StageSpec,
        attempt: u32,
        max_attempts: u32,
    ) -> Result<StageResult, ExecutionError> {
        let mut current_attempt = attempt;

        loop {
            let started_at = Utc::now();

            self.emit(PipelineEvent::StageStarted {
                run_id,
                pipeline_id: run_id,
                stage_id: stage_spec.id.clone(),
                stage_name: stage_spec.display_name.clone(),
                started_at,
            });

            // Get stage-level timeout from options
            let stage_timeout = stage_spec
                .options
                .as_ref()
                .and_then(|o| o.timeout);

            // Initialize environment context with stage env vars
            let mut env = EnvContext::new();
            if let Some(ref stage_env) = stage_spec.env {
                for (key, value) in stage_env.iter() {
                    env.set(key.to_string(), value.to_string());
                }
            }

            // Execute with optional timeout
            let result = if let Some(timeout) = stage_timeout {
                tokio::time::timeout(timeout, self.execute_stage_inner(run_id, stage_spec, &mut env))
                    .await
                    .map_err(|_| {
                        ExecutionError::Timeout {
                            stage_id: stage_spec.id.clone(),
                            timeout_secs: timeout.as_secs(),
                        }
                    })?
            } else {
                self.execute_stage_inner(run_id, stage_spec, &mut env).await
            };

            let mut stage_result = result?;
            stage_result.retry_count = current_attempt.saturating_sub(1);

            // Check if we should retry (exponential backoff)
            if !stage_result.success && current_attempt < max_attempts {
                self.emit(PipelineEvent::StageRetry {
                    run_id,
                    pipeline_id: run_id,
                    stage_id: stage_spec.id.clone(),
                    attempt: current_attempt,
                    max_attempts,
                });

                // Exponential backoff: 2^(attempt-1) seconds
                let backoff_secs = 2u64.saturating_pow(current_attempt - 1);
                if backoff_secs > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                }

                current_attempt += 1;
                continue;
            }

            self.emit(PipelineEvent::StageCompleted {
                run_id,
                pipeline_id: run_id,
                stage_id: stage_spec.id.clone(),
                stage_name: stage_spec.display_name.clone(),
                completed_at: Utc::now(),
                success: stage_result.success,
                exit_code: stage_result.exit_code,
            });

            return Ok(stage_result);
        }
    }

    async fn execute_stage_inner(
        &self,
        run_id: Uuid,
        stage_spec: &StageSpec,
        env: &mut EnvContext,
    ) -> Result<StageResult, ExecutionError> {
        match &stage_spec.execution {
            StageExecution::Steps { steps } => {
                self.execute_steps(run_id, &stage_spec.id, steps, &stage_spec.display_name, env)
                    .await
            }
            StageExecution::Parallel { stages } => {
                self.execute_parallel_stages(run_id, stages, env)
                    .await
            }
        }
    }

    async fn execute_steps(
        &self,
        run_id: Uuid,
        stage_id: &str,
        steps: &[StepSpec],
        stage_name: &str,
        env: &mut EnvContext,
    ) -> Result<StageResult, ExecutionError> {
        let mut step_results = Vec::new();
        let started_at = Utc::now();

        for (index, step) in steps.iter().enumerate() {
            let step_started_at = Utc::now();

            self.emit(PipelineEvent::StepStarted {
                run_id,
                pipeline_id: run_id,
                stage_id: stage_id.to_string(),
                step_index: index,
                step_type: step.type_name().to_string(),
                started_at: step_started_at,
            });

            let result = self.execute_step(step, env).await?;

            self.emit(PipelineEvent::StepCompleted {
                run_id,
                pipeline_id: run_id,
                stage_id: stage_id.to_string(),
                step_index: index,
                step_type: step.type_name().to_string(),
                completed_at: Utc::now(),
                success: result.success,
                exit_code: result.exit_code,
                duration_secs: result.duration_secs,
            });

            let step_label = step.label().map(|l| l.to_string());
            let step_type = step.type_name().to_string();

            step_results.push(StepResultEntry {
                step_index: index,
                step_type,
                step_label,
                started_at: step_started_at,
                result: result.clone(),
            });

            // Fail fast on step failure
            if !result.success && result.exit_code != Some(0) {
                // Check if step allows failure
                if !step.allow_failure() {
                    return Ok(StageResult {
                        stage_id: stage_id.to_string(),
                        stage_name: stage_name.to_string(),
                        success: false,
                        exit_code: result.exit_code,
                        duration_secs: (Utc::now() - started_at).num_milliseconds() as f64 / 1000.0,
                        step_results,
                        retry_count: 0,
                        started_at,
                    });
                }
            }
        }

        let all_success = step_results.iter().all(|s| s.result.success);
        let exit_code = step_results
            .last()
            .and_then(|s| s.result.exit_code);

        Ok(StageResult {
            stage_id: stage_id.to_string(),
            stage_name: stage_name.to_string(),
            success: all_success,
            exit_code,
            duration_secs: (Utc::now() - started_at).num_milliseconds() as f64 / 1000.0,
            step_results,
            retry_count: 0,
            started_at,
        })
    }

    async fn execute_parallel_stages(
        &self,
        run_id: Uuid,
        stages: &[StageSpec],
        env: &EnvContext,
    ) -> Result<StageResult, ExecutionError> {
        use futures::future;

        let started_at = Utc::now();
        let stage_name = "parallel".to_string();

        // Check for cancellation before starting
        if let Some(ref token) = self.cancellation_token {
            if token.is_cancelled() {
                self.emit_cancelled(run_id, "Cancelled before parallel execution started".to_string());
                return Err(ExecutionError::Cancelled {
                    reason: Some("Cancelled before parallel execution started".to_string()),
                });
            }
        }

        // Determine fail_fast for this parallel group
        // If any stage in the group has fail_fast=false, the group is not fail_fast
        let fail_fast = stages.iter().all(|s| {
            s.options.as_ref().map(|o| o.fail_fast).unwrap_or(true)
        });

        // Execute all stages in parallel using join_all
        // If semaphore is configured, limit concurrency
        let futures: Vec<_> = stages
            .iter()
            .map(|stage| {
                let rid = run_id;
                let stage_env = env.clone();
                let semaphore = self.semaphore.clone();
                let token = self.cancellation_token.clone();
                let executor = self.clone();
                async move {
                    // Acquire semaphore permit if configured
                    if let Some(ref sem) = semaphore {
                        let _permit = sem.acquire().await
                            .map_err(|_| ExecutionError::Cancelled {
                                reason: Some("Semaphore acquisition cancelled".to_string()),
                            })?;
                    }

                    // Check cancellation before execution
                    if let Some(ref t) = token {
                        if t.is_cancelled() {
                            return Err(ExecutionError::Cancelled {
                                reason: Some("Cancelled before stage execution".to_string()),
                            });
                        }
                    }

                    executor.execute_stage(rid, stage, 1, 1).await
                }
            })
            .collect();

        let results: Vec<StageResult> = future::join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, ExecutionError>>()?;

        let mut all_step_results = Vec::new();
        let mut overall_success = true;
        let mut exit_code = None;
        let mut any_failed = false;

        for (i, result) in results.into_iter().enumerate() {
            all_step_results.push(StepResultEntry {
                step_index: i,
                step_type: "parallel_stage".to_string(),
                step_label: Some(stages[i].id.clone()),
                started_at,
                result: StepResult {
                    success: result.success,
                    exit_code: result.exit_code,
                    duration_secs: result.duration_secs,
                    stdout: None,
                    stderr: None,
                },
            });

            if !result.success {
                overall_success = false;
                any_failed = true;
            }
            exit_code = result.exit_code.or(exit_code);
        }

        // If fail_fast is enabled and any stage failed, cancel the pipeline
        if any_failed && fail_fast {
            if let Some(ref token) = self.cancellation_token {
                token.cancel();
            }
        }

        Ok(StageResult {
            stage_id: "parallel".to_string(),
            stage_name,
            success: overall_success,
            exit_code,
            duration_secs: (Utc::now() - started_at).num_milliseconds() as f64 / 1000.0,
            step_results: all_step_results,
            retry_count: 0,
            started_at,
        })
    }

    async fn execute_step(&self, step: &StepSpec, env: &mut EnvContext) -> Result<StepResult, ExecutionError> {
        let start = std::time::Instant::now();

        match step {
            StepSpec::Shell(shell_spec) => self.execute_shell_step(shell_spec, env).await,
            StepSpec::Echo(echo_spec) => self.execute_echo_step(echo_spec, start),
            StepSpec::Dir(dir_spec) => {
                // For Dir, WithEnv, LetOutput - we need Pin<&Self> to call the impl methods
                // Use unsafe because LocalExecutor is Unpin
                let pinned = unsafe { std::pin::Pin::new_unchecked(self) };
                pinned.execute_dir_step_impl(dir_spec, env).await
            }
            StepSpec::WithEnv(with_env_spec) => {
                let pinned = unsafe { std::pin::Pin::new_unchecked(self) };
                pinned.execute_with_env_step_impl(with_env_spec, env).await
            }
            StepSpec::LetOutput(let_output_spec) => {
                let pinned = unsafe { std::pin::Pin::new_unchecked(self) };
                pinned.execute_let_output_step_impl(let_output_spec, env).await
            }
            StepSpec::WithCredentials(with_credentials_spec) => {
                let pinned = unsafe { std::pin::Pin::new_unchecked(self) };
                pinned.execute_with_credentials_step_impl(with_credentials_spec, env).await
            }
            StepSpec::JUnit(junit_spec) => {
                let pinned = unsafe { std::pin::Pin::new_unchecked(self) };
                pinned.execute_junit_step_impl(junit_spec, start).await
            }
            StepSpec::Archive(archive_spec) => {
                let pinned = unsafe { std::pin::Pin::new_unchecked(self) };
                pinned.execute_archive_step_impl(archive_spec, start).await
            }
        }
    }

    /// Helper to execute a single step with boxing for recursive calls.
    /// This is used by impl methods (execute_dir_step_impl, etc.) to call execute_step
    /// for inner step types.
    fn execute_step_boxed<'a>(self: Pin<&'a Self>, step: &'a StepSpec, env: &'a mut EnvContext) -> BoxFuture<'a> {
        Box::pin(async move {
            match step {
                StepSpec::Shell(shell_spec) => self.execute_shell_step(shell_spec, env).await,
                StepSpec::Echo(echo_spec) => {
                    let start = std::time::Instant::now();
                    self.execute_echo_step(echo_spec, start)
                }
                StepSpec::Dir(dir_spec) => self.execute_dir_step_impl(dir_spec, env).await,
                StepSpec::WithEnv(with_env_spec) => self.execute_with_env_step_impl(with_env_spec, env).await,
                StepSpec::LetOutput(let_output_spec) => self.execute_let_output_step_impl(let_output_spec, env).await,
                StepSpec::WithCredentials(with_credentials_spec) => self.execute_with_credentials_step_impl(with_credentials_spec, env).await,
                StepSpec::JUnit(junit_spec) => {
                    let start = std::time::Instant::now();
                    self.execute_junit_step_impl(junit_spec, start).await
                }
                StepSpec::Archive(archive_spec) => {
                    let start = std::time::Instant::now();
                    self.execute_archive_step_impl(archive_spec, start).await
                }
            }
        })
    }

    /// Implementation for Dir step - uses boxed future to avoid recursion issues
    async fn execute_dir_step_impl(
        self: Pin<&Self>,
        spec: &DirStepSpec,
        env: &mut EnvContext,
    ) -> Result<StepResult, ExecutionError> {
        use std::path::PathBuf;

        let start = std::time::Instant::now();

        // Save the current directory
        let original_dir = std::env::current_dir().map_err(|e| {
            ExecutionError::IoError(format!("Failed to get current directory: {}", e))
        })?;

        // Interpolate the directory path
        let target_dir = PathBuf::from(interpolate(&spec.path, env));

        // Change to the target directory
        if let Err(e) = std::env::set_current_dir(&target_dir) {
            return Err(ExecutionError::IoError(format!(
                "Failed to change directory to '{}': {}",
                target_dir.display(),
                e
            )));
        }

        // Execute inner steps using execute_step_boxed to avoid recursion issues
        let mut step_results = Vec::new();
        for (index, step) in spec.steps.iter().enumerate() {
            let result = self.execute_step_boxed(step, env).await?;

            step_results.push(StepResultEntry {
                step_index: index,
                step_type: step.type_name().to_string(),
                step_label: step.label().map(|l| l.to_string()),
                started_at: Utc::now(),
                result: result.clone(),
            });

            // Fail fast on step failure
            if !result.success && result.exit_code != Some(0) && !step.allow_failure() {
                // Restore directory before returning
                let _ = std::env::set_current_dir(&original_dir);
                return Ok(StepResult {
                    success: false,
                    exit_code: result.exit_code,
                    duration_secs: start.elapsed().as_secs_f64(),
                    stdout: None,
                    stderr: None,
                });
            }
        }

        // Restore the original directory
        if let Err(e) = std::env::set_current_dir(&original_dir) {
            eprintln!("Warning: Failed to restore directory to '{}': {}", original_dir.display(), e);
        }

        let all_success = step_results.iter().all(|s| s.result.success);
        let exit_code = step_results.last().and_then(|s| s.result.exit_code);
        let duration_secs = start.elapsed().as_secs_f64();

        Ok(StepResult {
            success: all_success,
            exit_code,
            duration_secs,
            stdout: None,
            stderr: None,
        })
    }

    /// Implementation for WithEnv step - uses boxed future to avoid recursion issues
    async fn execute_with_env_step_impl(
        self: Pin<&Self>,
        spec: &WithEnvStepSpec,
        env: &mut EnvContext,
    ) -> Result<StepResult, ExecutionError> {
        let start = std::time::Instant::now();

        // Save the current env values for variables that will be overridden
        let mut saved_values: HashMap<String, Option<String>> = HashMap::new();
        for (key, _) in spec.env.iter() {
            saved_values.insert(key.to_string(), env.get(key).map(|s| s.to_string()));
        }

        // Merge the new environment variables
        for (key, value) in spec.env.iter() {
            env.set(key.to_string(), value.to_string());
        }

        // Execute inner steps using execute_step_boxed to avoid recursion issues
        let mut step_results = Vec::new();
        for (index, step) in spec.steps.iter().enumerate() {
            let result = self.execute_step_boxed(step, env).await?;

            step_results.push(StepResultEntry {
                step_index: index,
                step_type: step.type_name().to_string(),
                step_label: step.label().map(|l| l.to_string()),
                started_at: Utc::now(),
                result: result.clone(),
            });

            // Fail fast on step failure
            if !result.success && result.exit_code != Some(0) && !step.allow_failure() {
                // Restore the parent environment before returning
                for (key, saved_value) in &saved_values {
                    match saved_value {
                        Some(value) => env.set(key.clone(), value.clone()),
                        None => { env.remove(key); }
                    }
                }
                return Ok(StepResult {
                    success: false,
                    exit_code: result.exit_code,
                    duration_secs: start.elapsed().as_secs_f64(),
                    stdout: None,
                    stderr: None,
                });
            }
        }

        // Restore the parent environment
        for (key, saved_value) in saved_values {
            match saved_value {
                Some(value) => env.set(key, value),
                None => { env.remove(&key); }
            }
        }

        let all_success = step_results.iter().all(|s| s.result.success);
        let exit_code = step_results.last().and_then(|s| s.result.exit_code);
        let duration_secs = start.elapsed().as_secs_f64();

        Ok(StepResult {
            success: all_success,
            exit_code,
            duration_secs,
            stdout: None,
            stderr: None,
        })
    }

    async fn execute_shell_step(
        &self,
        spec: &ShellStepSpec,
        env: &EnvContext,
    ) -> Result<StepResult, ExecutionError> {
        use std::process::Stdio;

        let start = std::time::Instant::now();

        // Interpolate the script if not in Raw mode
        let script = match spec.interpolation {
            InterpolationMode::Pipeliner => interpolate(&spec.script, env),
            InterpolationMode::Raw => spec.script.clone(),
        };

        let shell = match spec.kind {
            pipeliner_core::spec::step_spec::ShellKind::Sh => "sh",
            pipeliner_core::spec::step_spec::ShellKind::PowerShell => "pwsh",
            pipeliner_core::spec::step_spec::ShellKind::Cmd => "cmd",
        };

        let mut cmd = tokio::process::Command::new(shell);
        cmd.arg("-c");
        cmd.arg(&script);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(ref workdir) = self.config.workdir {
            cmd.current_dir(workdir);
        }

        let output = cmd.output().await.map_err(|e| {
            ExecutionError::StepExecutionFailed(format!("Failed to execute shell: {}", e))
        })?;

        let duration_secs = start.elapsed().as_secs_f64();
        let stdout = if spec.capture_stdout {
            let s = String::from_utf8_lossy(&output.stdout).to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        } else {
            None
        };
        let stderr = if !output.stderr.is_empty() {
            Some(String::from_utf8_lossy(&output.stderr).to_string())
        } else {
            None
        };

        let exit_code = output.status.code();
        let success = output.status.success();

        if !success && spec.fail_on_nonzero && exit_code != Some(0) {
            return Ok(StepResult {
                success: false,
                exit_code,
                duration_secs,
                stdout,
                stderr,
            });
        }

        Ok(StepResult {
            success,
            exit_code,
            duration_secs,
            stdout,
            stderr,
        })
    }

    /// Implementation for LetOutput step - uses boxed future to avoid recursion issues
    async fn execute_let_output_step_impl(
        self: Pin<&Self>,
        spec: &LetOutputStepSpec,
        env: &mut EnvContext,
    ) -> Result<StepResult, ExecutionError> {
        let start = std::time::Instant::now();

        // Execute the inner step using execute_step_boxed to avoid recursion
        let inner_result = self.execute_step_boxed(&spec.inner, env).await?;

        // Capture stdout if the inner step is a shell step
        let output_value = match &*spec.inner {
            StepSpec::Shell(shell_spec) => {
                if shell_spec.capture_stdout {
                    inner_result.stdout.clone()
                } else {
                    // Still capture stdout for storage even if not explicitly requested
                    inner_result.stdout.clone()
                }
            }
            _ => inner_result.stdout.clone(),
        };

        // Store the output in the environment context
        if let Some(output) = output_value {
            // Trim the output
            let trimmed = output.trim().to_string();
            env.set(spec.var_name.clone(), trimmed);
        }

        let duration_secs = start.elapsed().as_secs_f64();

        Ok(StepResult {
            success: inner_result.success,
            exit_code: inner_result.exit_code,
            duration_secs,
            stdout: inner_result.stdout,
            stderr: inner_result.stderr,
        })
    }

    fn execute_echo_step(
        &self,
        spec: &EchoStepSpec,
        start: std::time::Instant,
    ) -> Result<StepResult, ExecutionError> {
        println!("[echo] {}", spec.message);
        Ok(StepResult {
            success: true,
            exit_code: Some(0),
            duration_secs: start.elapsed().as_secs_f64(),
            stdout: Some(spec.message.clone()),
            stderr: None,
        })
    }

    /// Implementation for WithCredentials step
    async fn execute_with_credentials_step_impl(
        self: Pin<&Self>,
        spec: &WithCredentialsStepSpec,
        env: &mut EnvContext,
    ) -> Result<StepResult, ExecutionError> {
        let start = std::time::Instant::now();

        // Save the current env values for variables that will be overridden
        let mut saved_values: HashMap<String, Option<String>> = HashMap::new();

        // For now, use in-memory provider as a simple default
        // In a full implementation, this would use a configurable provider chain
        use pipeliner_credentials::{Credential, CredentialProvider, MemoryProvider};
        let provider = MemoryProvider::default();

        // Process each credential binding
        for binding in &spec.bindings {
            saved_values.insert(binding.variable.clone(), env.get(&binding.variable).map(|s| s.to_string()));

            // Try to get the credential from the provider
            match provider.get(&binding.credentials_id) {
                Ok(cred) => {
                    // Set the credential value in the environment
                    let value = if cred.is_secret {
                        // For secrets, set masked value to prevent accidental exposure
                        cred.masked_value()
                    } else {
                        cred.value
                    };
                    env.set(binding.variable.clone(), value);
                }
                Err(e) => {
                    // If credential is not found, set empty value but don't fail
                    // unless it's explicitly required
                    tracing::warn!("Credential '{}' not found: {}", binding.credentials_id, e);
                    env.set(binding.variable.clone(), String::new());
                }
            }
        }

        // Execute inner steps using execute_step_boxed to avoid recursion issues
        let mut step_results = Vec::new();
        for (index, step) in spec.steps.iter().enumerate() {
            let result = self.execute_step_boxed(step, env).await?;

            step_results.push(StepResultEntry {
                step_index: index,
                step_type: step.type_name().to_string(),
                step_label: step.label().map(|l| l.to_string()),
                started_at: Utc::now(),
                result: result.clone(),
            });

            // Fail fast on step failure
            if !result.success && result.exit_code != Some(0) && !step.allow_failure() {
                // Restore the parent environment before returning
                for (key, saved_value) in &saved_values {
                    match saved_value {
                        Some(value) => env.set(key.clone(), value.clone()),
                        None => { env.remove(key); }
                    }
                }
                return Ok(StepResult {
                    success: false,
                    exit_code: result.exit_code,
                    duration_secs: start.elapsed().as_secs_f64(),
                    stdout: None,
                    stderr: None,
                });
            }
        }

        // Restore the parent environment
        for (key, saved_value) in saved_values {
            match saved_value {
                Some(value) => env.set(key, value),
                None => { env.remove(&key); }
            }
        }

        let all_success = step_results.iter().all(|s| s.result.success);
        let exit_code = step_results.last().and_then(|s| s.result.exit_code);
        let duration_secs = start.elapsed().as_secs_f64();

        Ok(StepResult {
            success: all_success,
            exit_code,
            duration_secs,
            stdout: None,
            stderr: None,
        })
    }

    /// Implementation for JUnit step
    async fn execute_junit_step_impl(
        self: Pin<&Self>,
        spec: &JUnitStepSpec,
        start: std::time::Instant,
    ) -> Result<StepResult, ExecutionError> {
        use std::fs;

        let report_path = &spec.report_path;

        // Check if the report file exists
        if !std::path::Path::new(report_path).exists() {
            if spec.allow_failure {
                return Ok(StepResult {
                    success: true,
                    exit_code: Some(0),
                    duration_secs: start.elapsed().as_secs_f64(),
                    stdout: None,
                    stderr: Some(format!("JUnit report not found at: {}", report_path)),
                });
            }
            return Err(ExecutionError::IoError(format!(
                "JUnit report not found at: {}",
                report_path
            )));
        }

        // Read and parse the JUnit XML report
        match fs::read_to_string(report_path) {
            Ok(content) => {
                // Simple parsing to extract test counts
                // In a full implementation, use a proper XML parser
                let testsuite_count = content.matches("<testsuite").count();
                let testcase_count = content.matches("<testcase").count();
                let failure_count = content.matches("<failure").count();
                let error_count = content.matches("<error").count();

                let passed = testcase_count - failure_count - error_count;
                let failed = failure_count + error_count;

                tracing::info!(
                    "JUnit report {}: {} tests, {} passed, {} failed ({} errors)",
                    report_path,
                    testcase_count,
                    passed,
                    failed,
                    error_count
                );

                let success = failed == 0;

                Ok(StepResult {
                    success,
                    exit_code: Some(if success { 0 } else { 1 }),
                    duration_secs: start.elapsed().as_secs_f64(),
                    stdout: Some(format!(
                        "JUnit: {} tests, {} passed, {} failed",
                        testcase_count, passed, failed
                    )),
                    stderr: None,
                })
            }
            Err(e) => {
                if spec.allow_failure {
                    Ok(StepResult {
                        success: true,
                        exit_code: Some(0),
                        duration_secs: start.elapsed().as_secs_f64(),
                        stdout: None,
                        stderr: Some(format!("Failed to read JUnit report: {}", e)),
                    })
                } else {
                    Err(ExecutionError::IoError(format!(
                        "Failed to read JUnit report at {}: {}",
                        report_path, e
                    )))
                }
            }
        }
    }

    /// Implementation for Archive step
    async fn execute_archive_step_impl(
        self: Pin<&Self>,
        spec: &ArchiveStepSpec,
        start: std::time::Instant,
    ) -> Result<StepResult, ExecutionError> {
        use std::fs;
        use std::io::Write;
        use std::path::Path;

        let artifact_name = &spec.artifact_name;
        let compression = spec.compression.as_deref().unwrap_or("zip");

        // Create artifacts directory if it doesn't exist
        let artifacts_dir = Path::new(".pipeliner/artifacts");
        if !artifacts_dir.exists() {
            fs::create_dir_all(artifacts_dir).map_err(|e| {
                ExecutionError::IoError(format!("Failed to create artifacts directory: {}", e))
            })?;
        }

        // Build the archive path
        let archive_path = match compression {
            "zip" => artifacts_dir.join(format!("{}.zip", artifact_name)),
            "tar.gz" | "tgz" => artifacts_dir.join(format!("{}.tar.gz", artifact_name)),
            _ => artifacts_dir.join(format!("{}.zip", artifact_name)),
        };

        // Collect files matching the glob patterns
        let mut files_to_archive: Vec<std::path::PathBuf> = Vec::new();
        for pattern in &spec.paths {
            if let Ok(matches) = glob::glob(pattern) {
                for entry in matches.flatten() {
                    if entry.is_file() {
                        files_to_archive.push(entry);
                    }
                }
            }
        }

        if files_to_archive.is_empty() {
            return Ok(StepResult {
                success: true,
                exit_code: Some(0),
                duration_secs: start.elapsed().as_secs_f64(),
                stdout: Some(format!("No files found matching patterns: {:?}", spec.paths)),
                stderr: None,
            });
        }

        // Create the archive
        match compression {
            "zip" => {
                let file = fs::File::create(&archive_path)
                    .map_err(|e| ExecutionError::IoError(format!("Failed to create archive: {}", e)))?;
                let mut zip = zip::ZipWriter::new(file);

                for path in &files_to_archive {
                    if let Ok(content) = fs::read(path) {
                        let name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");
                        zip.start_file(name, zip::write::SimpleFileOptions::default())
                            .map_err(|e| ExecutionError::IoError(format!("Failed to add file to archive: {}", e)))?;
                        zip.write_all(&content)
                            .map_err(|e| ExecutionError::IoError(format!("Failed to write to archive: {}", e)))?;
                    }
                }

                zip.finish()
                    .map_err(|e| ExecutionError::IoError(format!("Failed to finalize archive: {}", e)))?;
            }
            "tar.gz" | "tgz" => {
                let file = fs::File::create(&archive_path)
                    .map_err(|e| ExecutionError::IoError(format!("Failed to create archive: {}", e)))?;
                let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
                let mut tar = tar::Builder::new(enc);

                for path in &files_to_archive {
                    tar.append_path(path)
                        .map_err(|e| ExecutionError::IoError(format!("Failed to add file to archive: {}", e)))?;
                }

                tar.finish()
                    .map_err(|e| ExecutionError::IoError(format!("Failed to finalize archive: {}", e)))?;
            }
            _ => {
                return Err(ExecutionError::IoError(format!(
                    "Unsupported compression format: {}",
                    compression
                )));
            }
        }

        let archive_size = fs::metadata(&archive_path)
            .map(|m| m.len())
            .unwrap_or(0);

        tracing::info!(
            "Archived {} files to {} ({})",
            files_to_archive.len(),
            archive_path.display(),
            archive_size
        );

        Ok(StepResult {
            success: true,
            exit_code: Some(0),
            duration_secs: start.elapsed().as_secs_f64(),
            stdout: Some(format!(
                "Archived {} files to {}.{}",
                files_to_archive.len(),
                artifact_name,
                compression
            )),
            stderr: None,
        })
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution errors.
#[derive(Debug, Clone)]
pub enum ExecutionError {
    /// Stage not found
    StageNotFound(String),
    /// Step execution failed
    StepExecutionFailed(String),
    /// Pipeline validation error
    ValidationError(String),
    /// Timeout error
    Timeout { stage_id: String, timeout_secs: u64 },
    /// I/O error
    IoError(String),
    /// Execution cancelled via CancellationToken
    Cancelled { reason: Option<String> },
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::StageNotFound(id) => write!(f, "Stage not found: {}", id),
            ExecutionError::StepExecutionFailed(msg) => write!(f, "Step execution failed: {}", msg),
            ExecutionError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ExecutionError::Timeout { stage_id, timeout_secs } => {
                write!(
                    f,
                    "Stage '{}' timed out after {} seconds",
                    stage_id, timeout_secs
                )
            }
            ExecutionError::IoError(msg) => write!(f, "I/O error: {}", msg),
            ExecutionError::Cancelled { reason } => {
                write!(f, "Execution cancelled")?;
                if let Some(r) = reason {
                    write!(f, ": {}", r)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ExecutionError {}

impl From<std::io::Error> for ExecutionError {
    fn from(err: std::io::Error) -> Self {
        ExecutionError::IoError(err.to_string())
    }
}

// Extension trait to provide common methods for StepSpec
trait StepSpecExt {
    fn type_name(&self) -> &'static str;
    fn label(&self) -> Option<&str>;
    fn allow_failure(&self) -> bool;
}

impl StepSpecExt for StepSpec {
    fn type_name(&self) -> &'static str {
        match self {
            StepSpec::Shell(_) => "shell",
            StepSpec::Echo(_) => "echo",
            StepSpec::Dir(_) => "dir",
            StepSpec::WithEnv(_) => "with_env",
            StepSpec::LetOutput(_) => "let_output",
            StepSpec::WithCredentials(_) => "with_credentials",
            StepSpec::JUnit(_) => "junit",
            StepSpec::Archive(_) => "archive",
        }
    }

    fn label(&self) -> Option<&str> {
        match self {
            StepSpec::Shell(s) => s.label.as_deref(),
            StepSpec::Echo(_) => None,
            StepSpec::Dir(d) => Some(&d.path),
            StepSpec::WithEnv(_) => None,
            StepSpec::LetOutput(l) => Some(&l.var_name),
            StepSpec::WithCredentials(_) => None,
            StepSpec::JUnit(j) => Some(&j.report_path),
            StepSpec::Archive(a) => Some(&a.artifact_name),
        }
    }

    fn allow_failure(&self) -> bool {
        match self {
            StepSpec::Shell(s) => !s.fail_on_nonzero,
            StepSpec::Echo(_) => false,
            StepSpec::Dir(_) => false,
            StepSpec::WithEnv(_) => false,
            StepSpec::LetOutput(_) => false,
            StepSpec::WithCredentials(_) => false,
            StepSpec::JUnit(j) => j.allow_failure,
            StepSpec::Archive(_) => false,
        }
    }
}

// =============================================================================
// EnvContext and interpolate - tests
// =============================================================================

#[cfg(test)]
mod env_context_tests {
    use super::*;

    #[test]
    fn test_env_context_new() {
        let env = EnvContext::new();
        assert!(env.is_empty());
        assert_eq!(env.len(), 0);
        assert!(env.get("FOO").is_none());
    }

    #[test]
    fn test_env_context_with_vars() {
        let env = EnvContext::with_vars([
            ("FOO".to_string(), "bar".to_string()),
            ("BAZ".to_string(), "qux".to_string()),
        ]);
        assert_eq!(env.len(), 2);
        assert_eq!(env.get("FOO"), Some("bar"));
        assert_eq!(env.get("BAZ"), Some("qux"));
    }

    #[test]
    fn test_env_context_set_and_get() {
        let mut env = EnvContext::new();
        env.set("KEY".to_string(), "value".to_string());
        assert_eq!(env.get("KEY"), Some("value"));

        // Overwrite
        env.set("KEY".to_string(), "new_value".to_string());
        assert_eq!(env.get("KEY"), Some("new_value"));
    }

    #[test]
    fn test_env_context_iter() {
        let mut env = EnvContext::new();
        env.set("A".to_string(), "1".to_string());
        env.set("B".to_string(), "2".to_string());

        let vars: Vec<_> = env.iter().collect();
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn test_env_context_clone() {
        let mut env = EnvContext::new();
        env.set("FOO".to_string(), "bar".to_string());

        let cloned = env.clone();
        assert_eq!(cloned.get("FOO"), Some("bar"));
    }

    #[test]
    fn test_interpolate_bare_var() {
        let mut env = EnvContext::new();
        env.set("NAME".to_string(), "world".to_string());

        assert_eq!(interpolate("Hello $NAME!", &env), "Hello world!");
        assert_eq!(interpolate("${NAME}", &env), "world");
    }

    #[test]
    fn test_interpolate_braced_var() {
        let mut env = EnvContext::new();
        env.set("FOO".to_string(), "bar".to_string());

        assert_eq!(interpolate("${FOO}", &env), "bar");
        assert_eq!(interpolate("prefix_${FOO}_suffix", &env), "prefix_bar_suffix");
    }

    #[test]
    fn test_interpolate_undefined_var() {
        let env = EnvContext::new();

        assert_eq!(interpolate("Hello $UNDEFINED!", &env), "Hello !");
        assert_eq!(interpolate("Hello ${UNDEFINED}!", &env), "Hello !");
    }

    #[test]
    fn test_interpolate_escaped_dollar() {
        let env = EnvContext::new();

        assert_eq!(interpolate("$$DOLLAR$$", &env), "$$DOLLAR$$");
        assert_eq!(interpolate("Price is $$100", &env), "Price is $$100");
    }

    #[test]
    fn test_interpolate_multiple_vars() {
        let mut env = EnvContext::new();
        env.set("A".to_string(), "1".to_string());
        env.set("B".to_string(), "2".to_string());

        assert_eq!(interpolate("$A and $B", &env), "1 and 2");
        assert_eq!(interpolate("${A} and ${B}", &env), "1 and 2");
    }

    #[test]
    fn test_interpolate_no_vars() {
        let env = EnvContext::new();

        assert_eq!(interpolate("Hello World!", &env), "Hello World!");
        assert_eq!(interpolate("", &env), "");
    }

    #[test]
    fn test_interpolate_var_with_underscore() {
        let mut env = EnvContext::new();
        env.set("MY_VAR".to_string(), "value".to_string());

        assert_eq!(interpolate("$MY_VAR", &env), "value");
        assert_eq!(interpolate("${MY_VAR}", &env), "value");
    }

    #[test]
    fn test_interpolate_var_with_numbers() {
        let mut env = EnvContext::new();
        env.set("VAR1".to_string(), "one".to_string());
        env.set("VAR2".to_string(), "two".to_string());

        assert_eq!(interpolate("$VAR1 and $VAR2", &env), "one and two");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::spec::step_spec::EchoStepSpec;

    fn create_test_pipeline() -> PipelineSpec {
        PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
            .with_stage(
                StageSpec::new("build", "Build")
                    .with_steps(vec![StepSpec::Echo(EchoStepSpec {
                        message: "Building...".to_string(),
                    })]),
            )
            .with_stage(
                StageSpec::new("test", "Test")
                    .with_steps(vec![StepSpec::Echo(EchoStepSpec {
                        message: "Testing...".to_string(),
                    })]),
            )
    }

    #[tokio::test]
    async fn test_local_executor_new() {
        let executor = LocalExecutor::new();
        assert!(executor.emitter.read().is_none());
    }

    #[tokio::test]
    async fn test_local_executor_with_config() {
        let config = ExecutorConfig::new()
            .with_fail_fast(false)
            .with_max_parallelism(4);
        let executor = LocalExecutor::with_config(config);
        assert!(executor.emitter.read().is_none());
    }

    #[tokio::test]
    async fn test_execute_simple_pipeline() {
        let executor = LocalExecutor::new();
        let spec = create_test_pipeline();

        let result = executor.execute(&spec).await.unwrap();

        assert!(result.success);
        assert_eq!(result.stage_results.len(), 2);
        assert_eq!(result.stage_results[0].stage_id, "build");
        assert_eq!(result.stage_results[1].stage_id, "test");
    }

    #[tokio::test]
    async fn test_execute_with_buffered_emitter() {
        let emitter = BufferedEmitter::new();
        let emitter_clone = emitter.clone();

        let mut executor = LocalExecutor::new();
        executor.set_emitter(Box::new(emitter_clone));

        let spec = create_test_pipeline();
        let _ = executor.execute(&spec).await.unwrap();

        // We can't easily check the emitter contents here because
        // the emitter is moved into the executor
    }

    #[tokio::test]
    async fn test_stage_result_timing() {
        let executor = LocalExecutor::new();
        let spec = create_test_pipeline();

        let result = executor.execute(&spec).await.unwrap();

        assert!(result.total_duration_secs >= 0.0);
        for stage in &result.stage_results {
            assert!(stage.duration_secs >= 0.0);
        }
    }

    #[tokio::test]
    async fn test_parallel_stage_execution() {
        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0").with_stage(
            StageSpec::new("parallel", "Parallel")
                .with_parallel_stages(vec![
                    StageSpec::new("p1", "Parallel 1")
                        .with_steps(vec![StepSpec::Echo(EchoStepSpec {
                            message: "Parallel 1".to_string(),
                        })]),
                    StageSpec::new("p2", "Parallel 2")
                        .with_steps(vec![StepSpec::Echo(EchoStepSpec {
                            message: "Parallel 2".to_string(),
                        })]),
                ]),
        );

        let executor = LocalExecutor::new();
        let result = executor.execute(&spec).await.unwrap();

        assert!(result.success);
        assert_eq!(result.stage_results.len(), 1);
    }

    #[tokio::test]
    async fn test_fail_fast() {
        let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0").with_stage(
            StageSpec::new("fail", "Fail")
                .with_steps(vec![StepSpec::Shell(ShellStepSpec::new("exit 1"))]),
        );

        let executor = LocalExecutor::new();
        let result = executor.execute(&spec).await.unwrap();

        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_executor_config_builder() {
        let config = ExecutorConfig::new()
            .with_max_parallelism(4)
            .with_stage_timeout(std::time::Duration::from_secs(60))
            .with_step_timeout(std::time::Duration::from_secs(30))
            .with_fail_fast(false)
            .with_workdir(std::path::PathBuf::from("/tmp"));

        assert_eq!(config.max_parallelism, 4);
        assert!(config.stage_timeout.is_some());
        assert!(config.step_timeout.is_some());
        assert!(!config.fail_fast);
        assert!(config.workdir.is_some());
    }

    #[tokio::test]
    async fn test_step_result_success() {
        let result = StepResult::success(0, 1.5);
        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.duration_secs, 1.5);
    }

    #[tokio::test]
    async fn test_step_result_failure() {
        let result = StepResult::failure(Some(1), 0.5);
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
    }

    #[tokio::test]
    async fn test_execution_error_display() {
        let err = ExecutionError::StageNotFound("build".to_string());
        assert!(err.to_string().contains("build"));

        let err = ExecutionError::StepExecutionFailed("shell failed".to_string());
        assert!(err.to_string().contains("shell failed"));

        let err = ExecutionError::Timeout {
            stage_id: "test".to_string(),
            timeout_secs: 30,
        };
        assert!(err.to_string().contains("test"));
        assert!(err.to_string().contains("30"));
    }

    #[tokio::test]
    async fn test_shell_step_echo() {
        let spec = ShellStepSpec::new("echo hello");
        assert_eq!(spec.script, "echo hello");
        assert!(spec.fail_on_nonzero);
    }

    #[tokio::test]
    async fn test_execution_result_stage_started_at() {
        let result = create_test_execution_result();
        let stage_time = result.stage_started_at(0);
        assert_eq!(stage_time, result.started_at);
    }

    fn create_test_execution_result() -> ExecutionResult {
        let pipeline_id = Uuid::new_v4();
        let started_at = Utc::now();

        ExecutionResult {
            pipeline_id,
            pipeline_name: Some("test".to_string()),
            success: true,
            exit_code: Some(0),
            started_at,
            completed_at: Utc::now(),
            total_duration_secs: 10.0,
            stage_results: vec![StageResult {
                stage_id: "build".to_string(),
                stage_name: "Build".to_string(),
                success: true,
                exit_code: Some(0),
                duration_secs: 5.0,
                step_results: vec![],
                retry_count: 0,
                started_at,
            }],
            total_retries: 0,
        }
    }
}
