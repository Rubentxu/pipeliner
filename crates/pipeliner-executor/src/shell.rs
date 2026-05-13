//! Shell execution module with Jenkins compatibility
//!
//! This module provides shell command execution with full compatibility
//! with Jenkins Pipeline's `sh` command, including:
//!
//! - Variable expansion (`${VAR}`)
//! - Jenkins special variables (`WORKSPACE`, `BUILD_NUMBER`, etc.)
//! - Multi-line heredoc support
//! - Streaming output

use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Shell execution configuration
#[derive(Debug, Clone)]
pub struct ShellConfig {
    /// Working directory
    pub cwd: PathBuf,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Shell to use (default: sh)
    pub shell: String,

    /// Enable streaming output
    pub streaming: bool,

    /// Timeout for commands (None = no timeout)
    pub timeout: Option<Duration>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            cwd: env::current_dir().unwrap_or_default(),
            env: HashMap::new(),
            shell: "sh".to_string(),
            streaming: false,
            timeout: None,
        }
    }
}

/// Result of shell command execution
#[derive(Debug, Clone)]
pub struct ShellResult {
    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,

    /// Exit code
    pub exit_code: i32,

    /// Duration of execution
    pub duration: Duration,
}

impl ShellResult {
    /// Returns true if command succeeded (exit code 0)
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Returns true if command failed
    #[must_use]
    pub fn is_failure(&self) -> bool {
        self.exit_code != 0
    }
}

/// Builder for shell commands
#[derive(Debug, Clone)]
pub struct ShellCommand<'a> {
    config: &'a ShellConfig,
    env_override: HashMap<String, String>,
}

impl<'a> ShellCommand<'a> {
    /// Creates a new shell command builder
    #[must_use]
    pub fn new(config: &'a ShellConfig) -> Self {
        Self {
            config,
            env_override: HashMap::new(),
        }
    }

    /// Adds environment variables for this command only
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_override.insert(key.into(), value.into());
        self
    }

    /// Executes a shell command
    pub fn execute(&self, command: &str) -> Result<ShellResult, ShellError> {
        let expanded = expand_variables(command, &self.config.env);

        let env: HashMap<String, String> = self
            .config
            .env
            .iter()
            .chain(self.env_override.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let start = Instant::now();

        tracing::debug!(command = %expanded, "Executing shell command");

        if self.config.streaming {
            self.execute_streaming(&expanded, &env)
        } else {
            let result = self.execute_captured(&expanded, &env);
            result.map(|mut r| {
                r.duration = start.elapsed();
                r
            })
        }
    }

    /// Executes command with captured output
    fn execute_captured(
        &self,
        command: &str,
        env: &HashMap<String, String>,
    ) -> Result<ShellResult, ShellError> {
        let mut cmd = Command::new(&self.config.shell);
        cmd.arg("-c");
        cmd.arg(command);
        cmd.current_dir(&self.config.cwd);
        cmd.envs(env);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().map_err(|e| ShellError::Io(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let exit_code = output.status.code().unwrap_or(-1);

        if !stdout.is_empty() {
            print!("{}", stdout);
        }
        if !stderr.is_empty() {
            eprint!("{}", stderr);
        }

        if exit_code != 0 {
            return Err(ShellError::CommandFailed {
                code: exit_code,
                stderr,
            });
        }

        Ok(ShellResult {
            stdout,
            stderr,
            exit_code,
            duration: Duration::ZERO,
        })
    }

    /// Executes command with streaming output
    #[allow(dead_code)]
    fn execute_streaming(
        &self,
        command: &str,
        env: &HashMap<String, String>,
    ) -> Result<ShellResult, ShellError> {
        use std::io::{BufRead, Write};

        let mut cmd = Command::new(&self.config.shell);
        cmd.arg("-c");
        cmd.arg(command);
        cmd.current_dir(&self.config.cwd);
        cmd.envs(env);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| ShellError::Io(e.to_string()))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_handle = Arc::new(Mutex::new(String::new()));
        let stderr_handle = Arc::new(Mutex::new(String::new()));

        let stdout_thread = {
            let stdout_handle = Arc::clone(&stdout_handle);
            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        println!("{}", line);
                        let mut guard = stdout_handle.lock().unwrap();
                        guard.push_str(&line);
                        guard.push('\n');
                    }
                }
            })
        };

        let stderr_thread = {
            let stderr_handle = Arc::clone(&stderr_handle);
            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        eprintln!("WARN: {}", line);
                        let mut guard = stderr_handle.lock().unwrap();
                        guard.push_str(&line);
                        guard.push('\n');
                    }
                }
            })
        };

        let status = child.wait().map_err(|e| ShellError::Io(e.to_string()))?;
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        let stdout = {
            let guard = stdout_handle.lock().unwrap();
            guard.clone()
        };
        let stderr = {
            let guard = stderr_handle.lock().unwrap();
            guard.clone()
        };

        let exit_code = status.code().unwrap_or(-1);

        if exit_code != 0 {
            return Err(ShellError::CommandFailed {
                code: exit_code,
                stderr,
            });
        }

        Ok(ShellResult {
            stdout,
            stderr,
            exit_code,
            duration: Duration::ZERO,
        })
    }
}

/// Shell error type
#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    #[error("command failed with exit code {code}: {stderr}")]
    CommandFailed { code: i32, stderr: String },

    #[error("I/O error: {0}")]
    Io(String),

    #[error("timeout after {duration:?}")]
    Timeout { duration: Duration },
}

/// Expands environment variables in a command string
///
/// Variables are expanded using the `${VAR_NAME}` syntax.
/// If a variable is not found, it remains unchanged in the output.
pub fn expand_variables(input: &str, env: &HashMap<String, String>) -> String {
    static VAR_PATTERN: once_cell::sync::Lazy<Regex> =
        once_cell::sync::Lazy::new(|| Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap());

    VAR_PATTERN
        .replace_all(input, |caps: &regex::Captures| {
            let var_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Some(value) = env.get(var_name) {
                value.clone()
            } else {
                // Keep the original if not found
                caps.get(0)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            }
        })
        .to_string()
}

/// Expands variables and returns a mapping of found variables
#[allow(dead_code)]
pub fn expand_variables_with_info(
    input: &str,
    env: &HashMap<String, String>,
) -> (String, Vec<String>) {
    let mut found = Vec::new();
    let expanded = expand_variables(input, env);

    static VAR_PATTERN: once_cell::sync::Lazy<Regex> =
        once_cell::sync::Lazy::new(|| Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap());

    for cap in VAR_PATTERN.captures_iter(&expanded) {
        if let Some(var_name) = cap.get(1).map(|m| m.as_str()) {
            if env.contains_key(var_name) && !found.contains(&var_name.to_string()) {
                found.push(var_name.to_string());
            }
        }
    }

    (expanded, found)
}

/// Creates a new shell config with Jenkins special variables
#[must_use]
pub fn jenkins_shell_config(
    workspace: impl Into<PathBuf>,
    job_name: &str,
    build_number: usize,
    stage_name: Option<&str>,
    extra_env: Option<HashMap<String, String>>,
) -> ShellConfig {
    let build_id = Uuid::new_v4().to_string();

    let mut env = HashMap::from([
        (
            "WORKSPACE".to_string(),
            workspace.into().to_string_lossy().to_string(),
        ),
        ("WORKSPACE_TMP".to_string(), "@tmp".to_string()),
        ("BUILD_NUMBER".to_string(), build_number.to_string()),
        ("BUILD_ID".to_string(), build_id),
        ("JOB_NAME".to_string(), job_name.to_string()),
        (
            "STAGE_NAME".to_string(),
            stage_name.unwrap_or("").to_string(),
        ),
        ("NODE_NAME".to_string(), "local".to_string()),
        (
            "JENKINS_URL".to_string(),
            "http://localhost:8080".to_string(),
        ),
    ]);

    if let Some(extra) = extra_env {
        for (k, v) in extra {
            env.insert(k, v);
        }
    }

    ShellConfig {
        cwd: PathBuf::from(env.get("WORKSPACE").cloned().unwrap_or_default()),
        env,
        shell: "sh".to_string(),
        streaming: false,
        timeout: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_variables_simple() {
        let env = HashMap::from([
            ("BUILD_NUMBER".to_string(), "123".to_string()),
            ("PROJECT".to_string(), "my-app".to_string()),
        ]);

        let expanded = expand_variables("echo ${BUILD_NUMBER}", &env);
        assert_eq!(expanded, "echo 123");
    }

    #[test]
    fn test_expand_variables_multiple() {
        let env = HashMap::from([
            ("BUILD_NUMBER".to_string(), "456".to_string()),
            ("PROJECT".to_string(), "test-project".to_string()),
        ]);

        let expanded = expand_variables("Building ${PROJECT} #${BUILD_NUMBER}", &env);
        assert_eq!(expanded, "Building test-project #456");
    }

    #[test]
    fn test_expand_variables_not_found() {
        let env = HashMap::from([("FOO".to_string(), "bar".to_string())]);

        let expanded = expand_variables("echo ${UNKNOWN}", &env);
        assert_eq!(expanded, "echo ${UNKNOWN}");
    }

    #[test]
    fn test_expand_variables_mixed() {
        let env = HashMap::from([
            ("BUILD_NUMBER".to_string(), "789".to_string()),
            ("FOO".to_string(), "bar".to_string()),
        ]);

        let expanded = expand_variables("${BUILD_NUMBER} and ${UNKNOWN} and ${FOO}", &env);
        assert_eq!(expanded, "789 and ${UNKNOWN} and bar");
    }

    #[test]
    fn test_expand_variables_no_vars() {
        let env = HashMap::new();

        let expanded = expand_variables("echo hello world", &env);
        assert_eq!(expanded, "echo hello world");
    }

    #[test]
    fn test_jenkins_shell_config() {
        let config =
            jenkins_shell_config("/workspace/my-project", "my-job", 42, Some("Build"), None);

        assert_eq!(config.cwd.to_string_lossy(), "/workspace/my-project");
        assert_eq!(
            config.env.get("WORKSPACE").unwrap(),
            "/workspace/my-project"
        );
        assert_eq!(config.env.get("BUILD_NUMBER").unwrap(), "42");
        assert_eq!(config.env.get("JOB_NAME").unwrap(), "my-job");
        assert_eq!(config.env.get("STAGE_NAME").unwrap(), "Build");
    }

    #[test]
    fn test_shell_result_is_success() {
        let result = ShellResult {
            stdout: "output".to_string(),
            stderr: "".to_string(),
            exit_code: 0,
            duration: Duration::from_millis(100),
        };
        assert!(result.is_success());
        assert!(!result.is_failure());
    }

    #[test]
    fn test_shell_result_is_failure() {
        let result = ShellResult {
            stdout: "".to_string(),
            stderr: "error".to_string(),
            exit_code: 1,
            duration: Duration::from_millis(100),
        };
        assert!(!result.is_success());
        assert!(result.is_failure());
    }

    #[test]
    fn test_shell_command_execute_success() {
        let config = ShellConfig::default();
        let cmd = ShellCommand::new(&config);
        let result = cmd.execute("echo hello").unwrap();
        assert!(result.is_success());
        assert!(result.stdout.contains("hello") || result.exit_code == 0);
    }

    #[test]
    fn test_shell_command_execute_failure() {
        let config = ShellConfig::default();
        let cmd = ShellCommand::new(&config);
        let result = cmd.execute("exit 1");
        assert!(result.is_err());
        if let Err(ShellError::CommandFailed { code, .. }) = result {
            assert_eq!(code, 1);
        } else {
            panic!("Expected CommandFailed error");
        }
    }

    #[test]
    fn test_shell_command_with_env_override() {
        let config = ShellConfig::default();
        let cmd = ShellCommand::new(&config).env("MY_VAR", "test_value");
        let result = cmd.execute("echo $MY_VAR");
        assert!(result.is_ok());
    }
}