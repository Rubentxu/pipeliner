//! # Runner Module
//!
//! Executes compiled Rust script binaries with pipeline context.
//!
//! The runner handles:
//! - Setting environment variables
//! - Changing working directory
//! - Capturing stdout/stderr
//! - Timeout handling
//! - Pipeline parameter injection
//!
//! ## Pipeline Context
//!
//! Scripts receive pipeline context via environment variables:
//!
//! ```ignore
//! PIPELINE_NAME=pipeline-name
//! PIPELINE_STAGE=stage-name
//! PIPELINE_STEP=step-name
//! PIPELINE_ROOT=/path/to/pipeline/root
//! PIPELINE_PARAM_<NAME>=value  // Custom parameters
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use pipeliner_script::{ScriptRunner, ScriptConfig};
//!
//! let config = ScriptConfig {
//!     binary_path: PathBuf::from("/tmp/script"),
//!     workdir: Some(PathBuf::from("/tmp")),
//!     env: vec![("KEY".to_string(), "value".to_string())],
//!     ..Default::default()
//! };
//!
//! let runner = ScriptRunner::new();
//! let result = runner.run(config).await?;
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

/// Pipeline context passed to script execution.
#[derive(Debug, Clone, Default)]
pub struct PipelineContext {
    /// Pipeline name
    pub pipeline_name: Option<String>,
    /// Current stage name
    pub stage_name: Option<String>,
    /// Current step name
    pub step_name: Option<String>,
    /// Pipeline root directory
    pub pipeline_root: Option<PathBuf>,
    /// Custom parameters (PIPELINE_PARAM_<NAME>)
    pub parameters: HashMap<String, String>,
}

impl PipelineContext {
    /// Creates a new empty pipeline context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the pipeline name.
    #[must_use]
    pub fn with_pipeline_name(mut self, name: impl Into<String>) -> Self {
        self.pipeline_name = Some(name.into());
        self
    }

    /// Sets the stage name.
    #[must_use]
    pub fn with_stage_name(mut self, name: impl Into<String>) -> Self {
        self.stage_name = Some(name.into());
        self
    }

    /// Sets the step name.
    #[must_use]
    pub fn with_step_name(mut self, name: impl Into<String>) -> Self {
        self.step_name = Some(name.into());
        self
    }

    /// Sets the pipeline root directory.
    #[must_use]
    pub fn with_pipeline_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.pipeline_root = Some(root.into());
        self
    }

    /// Adds a custom parameter.
    #[must_use]
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    /// Converts context to environment variables.
    #[must_use]
    pub fn to_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        if let Some(ref name) = self.pipeline_name {
            env.insert("PIPELINE_NAME".to_string(), name.clone());
        }
        if let Some(ref stage) = self.stage_name {
            env.insert("PIPELINE_STAGE".to_string(), stage.clone());
        }
        if let Some(ref step) = self.step_name {
            env.insert("PIPELINE_STEP".to_string(), step.clone());
        }
        if let Some(ref root) = self.pipeline_root {
            env.insert("PIPELINE_ROOT".to_string(), root.to_string_lossy().to_string());
        }

        for (key, value) in &self.parameters {
            env.insert(format!("PIPELINE_PARAM_{}", key.to_uppercase()), value.clone());
        }

        env
    }
}

/// Configuration for script execution.
#[derive(Debug, Clone)]
pub struct ScriptConfig {
    /// Path to the compiled binary
    pub binary_path: PathBuf,
    /// Working directory (None = current directory)
    pub workdir: Option<PathBuf>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// Command-line arguments to pass to the script
    pub args: Vec<String>,
    /// Timeout for execution (None = no timeout)
    pub timeout: Option<Duration>,
    /// Pipeline context
    pub pipeline_context: PipelineContext,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::new(),
            workdir: None,
            env: Vec::new(),
            args: Vec::new(),
            timeout: None,
            pipeline_context: PipelineContext::new(),
        }
    }
}

impl ScriptConfig {
    /// Creates a new script config with the given binary path.
    #[must_use]
    pub fn new(binary_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: binary_path.into(),
            ..Default::default()
        }
    }

    /// Sets the working directory.
    #[must_use]
    pub fn with_workdir(mut self, workdir: impl Into<PathBuf>) -> Self {
        self.workdir = Some(workdir.into());
        self
    }

    /// Adds an environment variable.
    #[must_use]
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Adds command-line arguments.
    #[must_use]
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.args = args.into_iter().map(|s| s.as_ref().to_string()).collect();
        self
    }

    /// Sets the timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the pipeline context.
    #[must_use]
    pub fn with_pipeline_context(mut self, context: PipelineContext) -> Self {
        self.pipeline_context = context;
        self
    }
}

/// Output from a script execution.
#[derive(Debug, Clone)]
pub struct ScriptOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Whether execution was killed due to timeout
    pub timed_out: bool,
    /// Execution duration
    pub duration_secs: f64,
}

impl ScriptOutput {
    /// Returns true if the script exited successfully (exit code 0).
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Returns true if the script was killed due to timeout.
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        self.timed_out
    }

    /// Returns the combined output (stdout + stderr).
    #[must_use]
    pub fn combined(&self) -> String {
        let mut output = self.stdout.clone();
        if !self.stderr.is_empty() {
            output.push_str("\n--- STDERR ---\n");
            output.push_str(&self.stderr);
        }
        output
    }
}

/// Result of script execution.
pub type ScriptResult = Result<ScriptOutput, ScriptError>;

/// Script runner for executing compiled binaries.
#[derive(Debug, Clone)]
pub struct ScriptRunner {
    /// Default timeout if not specified in config
    default_timeout: Option<Duration>,
}

impl ScriptRunner {
    /// Creates a new script runner.
    #[must_use]
    pub fn new() -> Self {
        Self {
            default_timeout: None,
        }
    }

    /// Creates a runner with a default timeout.
    #[must_use]
    pub fn with_default_timeout(timeout: Duration) -> Self {
        Self {
            default_timeout: Some(timeout),
        }
    }

    /// Runs a script with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns `ScriptError` if execution fails.
    pub async fn run(&self, config: ScriptConfig) -> ScriptResult {
        let start = std::time::Instant::now();

        // Check binary exists
        if !config.binary_path.exists() {
            return Err(ScriptError::BinaryNotFound(
                config.binary_path.to_string_lossy().to_string(),
            ));
        }

        // Use default timeout if not specified
        let timeout_dur = config.timeout.or(self.default_timeout);

        // Build command
        let mut cmd = Command::new(&config.binary_path);
        cmd.args(&config.args);

        // Set working directory
        if let Some(ref workdir) = config.workdir {
            cmd.current_dir(workdir);
        }

        // Set environment variables
        let pipeline_env = config.pipeline_context.to_env();
        for (key, value) in &pipeline_env {
            cmd.env(key, value);
        }
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd
            .spawn()
            .map_err(|e| ScriptError::ExecutionFailed(format!("Failed to spawn: {}", e)))?;

        // Capture stdout
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (stdout_out, stderr_out) = if let (Some(stdout), Some(stderr)) = (stdout, stderr) {
            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();

            let mut stdout_out = String::new();
            let mut stderr_out = String::new();

            // Read output in a separate task
            let stdout_handle = tokio::spawn(async move {
                let mut lines = String::new();
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    lines.push_str(&line);
                    lines.push('\n');
                }
                lines
            });

            let stderr_handle = tokio::spawn(async move {
                let mut lines = String::new();
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    lines.push_str(&line);
                    lines.push('\n');
                }
                lines
            });

            // Wait for both with optional timeout
            let combined_future = async {
                let (a, b) = tokio::join!(stdout_handle, stderr_handle);
                (
                    a.unwrap_or_default(),
                    b.unwrap_or_default(),
                )
            };

            if let Some(dur) = timeout_dur {
                match timeout(dur, combined_future).await {
                    Ok(result) => result,
                    Err(_) => {
                        // Kill the child and return timeout error
                        let _ = child.kill().await;
                        child.wait().await.ok();
                        return Ok(ScriptOutput {
                            stdout: String::new(),
                            stderr: String::new(),
                            exit_code: None,
                            timed_out: true,
                            duration_secs: start.elapsed().as_secs_f64(),
                        });
                    }
                }
            } else {
                combined_future.await
            }
        } else {
            (String::new(), String::new())
        };

        // Wait for process to complete
        let status = child
            .wait()
            .await
            .map_err(|e| ScriptError::ExecutionFailed(format!("Failed to wait: {}", e)))?;

        let duration_secs = start.elapsed().as_secs_f64();

        Ok(ScriptOutput {
            stdout: stdout_out,
            stderr: stderr_out,
            exit_code: status.code(),
            timed_out: false,
            duration_secs,
        })
    }

    /// Runs a script file (blocking, with default timeout).
    ///
    /// This is a convenience method for simple script execution.
    pub async fn run_file(
        &self,
        binary_path: impl Into<PathBuf>,
    ) -> ScriptResult {
        let config = ScriptConfig::new(binary_path);
        self.run(config).await
    }
}

impl Default for ScriptRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Script execution errors.
#[derive(Debug, Clone)]
pub enum ScriptError {
    /// Binary not found
    BinaryNotFound(String),
    /// Execution failed
    ExecutionFailed(String),
    /// Script failed with non-zero exit code
    ScriptFailed { exit_code: i32, stderr: String },
    /// Script timed out
    Timeout { timeout_secs: u64 },
    /// I/O error
    IoError(String),
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptError::BinaryNotFound(path) => {
                write!(f, "Binary not found: {}", path)
            }
            ScriptError::ExecutionFailed(msg) => {
                write!(f, "Execution failed: {}", msg)
            }
            ScriptError::ScriptFailed { exit_code, stderr } => {
                write!(f, "Script failed with exit code {}: {}", exit_code, stderr)
            }
            ScriptError::Timeout { timeout_secs } => {
                write!(f, "Script timed out after {} seconds", timeout_secs)
            }
            ScriptError::IoError(msg) => {
                write!(f, "I/O error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ScriptError {}

impl From<std::io::Error> for ScriptError {
    fn from(err: std::io::Error) -> Self {
        ScriptError::IoError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_context_new() {
        let ctx = PipelineContext::new();
        assert!(ctx.pipeline_name.is_none());
        assert!(ctx.parameters.is_empty());
    }

    #[test]
    fn test_pipeline_context_builder() {
        let ctx = PipelineContext::new()
            .with_pipeline_name("my-pipeline")
            .with_stage_name("build")
            .with_step_name("compile")
            .with_pipeline_root("/workspace")
            .with_param("VERSION", "1.0.0");

        assert_eq!(ctx.pipeline_name, Some("my-pipeline".to_string()));
        assert_eq!(ctx.stage_name, Some("build".to_string()));
        assert_eq!(ctx.step_name, Some("compile".to_string()));
        assert_eq!(ctx.pipeline_root, Some(PathBuf::from("/workspace")));
        assert_eq!(ctx.parameters.get("VERSION"), Some(&"1.0.0".to_string()));
    }

    #[test]
    fn test_pipeline_context_to_env() {
        let ctx = PipelineContext::new()
            .with_pipeline_name("test-pipeline")
            .with_stage_name("test-stage")
            .with_step_name("test-step")
            .with_pipeline_root("/root")
            .with_param("FOO", "bar");

        let env = ctx.to_env();

        assert_eq!(env.get("PIPELINE_NAME"), Some(&"test-pipeline".to_string()));
        assert_eq!(env.get("PIPELINE_STAGE"), Some(&"test-stage".to_string()));
        assert_eq!(env.get("PIPELINE_STEP"), Some(&"test-step".to_string()));
        assert_eq!(env.get("PIPELINE_ROOT"), Some(&"/root".to_string()));
        assert_eq!(env.get("PIPELINE_PARAM_FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_script_config_default() {
        let config = ScriptConfig::default();
        assert!(config.binary_path.as_os_str().is_empty());
        assert!(config.workdir.is_none());
        assert!(config.env.is_empty());
        assert!(config.args.is_empty());
    }

    #[test]
    fn test_script_config_builder() {
        let config = ScriptConfig::new("/tmp/bin")
            .with_workdir("/workspace")
            .with_env("RUST_LOG", "debug")
            .with_env("DEBUG", "true")
            .with_args(["--verbose", "--flag"])
            .with_timeout(Duration::from_secs(30));

        assert_eq!(config.binary_path, PathBuf::from("/tmp/bin"));
        assert_eq!(config.workdir, Some(PathBuf::from("/workspace")));
        assert_eq!(config.env.len(), 2);
        assert_eq!(config.args, vec!["--verbose", "--flag"]);
        assert_eq!(config.timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_script_output_is_success() {
        let success = ScriptOutput {
            stdout: "ok".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            timed_out: false,
            duration_secs: 0.1,
        };
        assert!(success.is_success());

        let failure = ScriptOutput {
            stdout: String::new(),
            stderr: "error".to_string(),
            exit_code: Some(1),
            timed_out: false,
            duration_secs: 0.1,
        };
        assert!(!failure.is_success());
    }

    #[test]
    fn test_script_output_is_timeout() {
        let timed_out = ScriptOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            timed_out: true,
            duration_secs: 30.0,
        };
        assert!(timed_out.is_timeout());
        assert!(!timed_out.is_success());
    }

    #[test]
    fn test_script_output_combined() {
        let output = ScriptOutput {
            stdout: "stdout content".to_string(),
            stderr: "stderr content".to_string(),
            exit_code: Some(0),
            timed_out: false,
            duration_secs: 0.1,
        };

        let combined = output.combined();
        assert!(combined.contains("stdout content"));
        assert!(combined.contains("--- STDERR ---"));
        assert!(combined.contains("stderr content"));
    }

    #[test]
    fn test_script_runner_new() {
        let runner = ScriptRunner::new();
        assert!(runner.default_timeout.is_none());
    }

    #[test]
    fn test_script_runner_with_default_timeout() {
        let runner = ScriptRunner::with_default_timeout(Duration::from_secs(60));
        assert_eq!(runner.default_timeout, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_script_error_display() {
        let err = ScriptError::BinaryNotFound("/path".to_string());
        assert!(err.to_string().contains("/path"));

        let err = ScriptError::ScriptFailed {
            exit_code: 42,
            stderr: "oops".to_string(),
        };
        assert!(err.to_string().contains("42"));
        assert!(err.to_string().contains("oops"));

        let err = ScriptError::Timeout { timeout_secs: 30 };
        assert!(err.to_string().contains("30"));
    }

    #[tokio::test]
    async fn test_run_nonexistent_binary() {
        let runner = ScriptRunner::new();
        let config = ScriptConfig::new("/nonexistent/binary/path");
        let result = runner.run(config).await;

        assert!(result.is_err());
        if let Err(ScriptError::BinaryNotFound(_)) = result {
            // Expected
        } else {
            panic!("Expected BinaryNotFound error");
        }
    }
}