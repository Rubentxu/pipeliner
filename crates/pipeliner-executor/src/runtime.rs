//! Runtime for executing pipeline steps.
//!
//! This module provides the step executor that handles the execution
//! of individual pipeline steps.

use async_trait::async_trait;
use std::path::PathBuf;
use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

use pipeliner_core::{Step, StepType};

use crate::{ExecutionContext, ExecutionStatus, ExecutorResult};

/// Step executor trait
#[async_trait]
pub trait StepExecutorTrait: Send + Sync {
    /// Executes a step
    async fn execute(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus>;
}

/// Built-in step executor
#[derive(Debug, Default)]
pub struct StepExecutor;

impl StepExecutor {
    /// Creates a new step executor
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StepExecutorTrait for StepExecutor {
    async fn execute(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let step_name = step.name.clone().unwrap_or_else(|| "unnamed".to_string());
        context.set_current_step(&step_name);

        debug!("Executing step: {}", step_name);

        let result = match &step.step_type {
            StepType::Shell { command } => self.execute_shell(command, step, context).await,
            StepType::Echo { message } => self.execute_echo(message, step, context).await,
            StepType::Retry { count, step: inner } => {
                self.execute_retry(inner.as_ref(), *count, step, context)
                    .await
            }
            StepType::Timeout {
                duration,
                step: inner,
            } => {
                self.execute_timeout(inner.as_ref(), *duration, step, context)
                    .await
            }
            StepType::Stash {
                name,
                includes,
                excludes,
            } => {
                self.execute_stash(name, includes, excludes, step, context)
                    .await
            }
            StepType::Unstash { name } => self.execute_unstash(name, step, context).await,
            StepType::Input { message, .. } => self.execute_input(message, step, context).await,
            StepType::Dir { path, steps } => self.execute_dir(path, steps, step, context).await,
            StepType::Script { content } => self.execute_script(content, step, context).await,
            StepType::Archive {
                artifacts,
                excludes,
                fingerprint,
            } => {
                self.execute_archive(artifacts, excludes, *fingerprint, step, context)
                    .await
            }
            StepType::Custom { name, config } => {
                self.execute_custom(name, config, step, context).await
            }
        };

        context.clear_current_step();
        result
    }
}

impl StepExecutor {
    async fn execute_shell(
        &self,
        command: &str,
        _step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let resolved_command = self.resolve_variables(command, context);

        info!("Executing shell: {}", resolved_command);

        let output = self.run_command(&resolved_command, context).await?;

        if output.status.success() {
            Ok(ExecutionStatus::Success)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Shell command failed: {}", stderr);
            Ok(ExecutionStatus::Failure)
        }
    }

    async fn execute_echo(
        &self,
        message: &str,
        _step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let resolved_message = self.resolve_variables(message, context);
        info!("{}", resolved_message);
        Ok(ExecutionStatus::Success)
    }

    async fn execute_retry(
        &self,
        inner: &Step,
        count: usize,
        _step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let mut last_error = None;

        for attempt in 0..=count {
            if attempt > 0 {
                info!("Retry attempt {}/{}", attempt, count);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            match self.execute(inner, context).await {
                Ok(ExecutionStatus::Success) => return Ok(ExecutionStatus::Success),
                Ok(status) => return Ok(status),
                Err(e) => {
                    last_error = Some(e);
                    warn!("Retry attempt {} failed", attempt);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            crate::ExecutorError::from(crate::ExecutorErrorKind::RetryExhausted {
                attempts: count + 1,
            })
        }))
    }

    async fn execute_timeout(
        &self,
        inner: &Step,
        duration: Duration,
        _step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let result = tokio::time::timeout(duration, self.execute(inner, context)).await;

        match result {
            Ok(Ok(status)) => Ok(status),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                warn!("Step timed out after {:?}", duration);
                Ok(ExecutionStatus::Timeout)
            }
        }
    }

    async fn execute_stash(
        &self,
        name: &str,
        includes: &[String],
        excludes: &[String],
        _step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let stash_path = context.cwd().join(".pipeliner").join("stashes").join(name);
        tokio::fs::create_dir_all(&stash_path).await?;

        for pattern in includes {
            self.copy_files(pattern, &stash_path, excludes).await?;
        }

        context.stash(name, stash_path).await;
        Ok(ExecutionStatus::Success)
    }

    async fn execute_unstash(
        &self,
        name: &str,
        _step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        if let Some(path) = context.unstash(name).await {
            self.copy_all(&path, context.cwd()).await?;
            return Ok(ExecutionStatus::Success);
        }

        error!("Stash '{}' not found", name);
        Ok(ExecutionStatus::Failure)
    }

    async fn execute_input(
        &self,
        message: &str,
        _step: &Step,
        _context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        warn!("Input step requires interactive input: {}", message);
        Ok(ExecutionStatus::Success)
    }

    async fn execute_dir(
        &self,
        path: &PathBuf,
        steps: &[Step],
        _step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        context.push_dir(path.clone());

        let result = self.execute_steps(steps, context).await;

        context.pop_dir();
        result
    }

    async fn execute_script(
        &self,
        content: &str,
        _step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let script_path = context.cwd().join(".pipeliner").join("script.sh");
        tokio::fs::write(&script_path, content).await?;

        let output = self
            .run_command(&format!("bash {}", script_path.display()), context)
            .await?;

        Ok(if output.status.success() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Failure
        })
    }

    async fn execute_archive(
        &self,
        artifacts: &[String],
        excludes: &[String],
        _fingerprint: bool,
        _step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let archive_dir = context.cwd().join(".pipeliner").join("archive");
        tokio::fs::create_dir_all(&archive_dir).await?;

        for pattern in artifacts {
            self.copy_files(pattern, &archive_dir, excludes).await?;
        }

        Ok(ExecutionStatus::Success)
    }

    async fn execute_custom(
        &self,
        name: &str,
        _config: &serde_json::Value,
        _step: &Step,
        _context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        warn!("Custom step '{}' not implemented", name);
        Ok(ExecutionStatus::Success)
    }

    async fn execute_steps(
        &self,
        steps: &[Step],
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        for step in steps {
            match self.execute(step, context).await {
                Ok(ExecutionStatus::Success) => continue,
                Ok(status) => return Ok(status),
                Err(e) => return Err(e),
            }
        }
        Ok(ExecutionStatus::Success)
    }

    async fn run_command(
        &self,
        command: &str,
        context: &ExecutionContext,
    ) -> ExecutorResult<Output> {
        let resolved_command = self.resolve_variables(command, context);

        let mut parts = shell_words::split(&resolved_command).map_err(|e| {
            crate::ExecutorError::from(crate::ExecutorErrorKind::StepFailed {
                reason: format!("Failed to parse command: {}", e),
            })
        })?;

        if parts.is_empty() {
            return Err(crate::ExecutorError::from(
                crate::ExecutorErrorKind::StepFailed {
                    reason: "Empty command".to_string(),
                },
            ));
        }

        let mut cmd = Command::new(&parts[0]);
        cmd.args(&parts[1..])
            .current_dir(context.cwd())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        for (key, value) in context.environment.iter() {
            cmd.env(key, value.to_string());
        }

        let mut child = cmd.spawn().map_err(|e| {
            crate::ExecutorError::from(crate::ExecutorErrorKind::IoError { reason: e })
        })?;

        let status = child.wait().await.map_err(|e| {
            crate::ExecutorError::from(crate::ExecutorErrorKind::IoError { reason: e })
        })?;

        let mut stdout_buf = Vec::new();
        if let Some(mut stdout) = child.stdout {
            let _ = stdout.read_to_end(&mut stdout_buf).await;
        }

        let mut stderr_buf = Vec::new();
        if let Some(mut stderr) = child.stderr {
            let _ = stderr.read_to_end(&mut stderr_buf).await;
        }

        let output = Output {
            status,
            stdout: stdout_buf,
            stderr: stderr_buf,
        };

        Ok(output)
    }

    fn resolve_variables(&self, input: &str, context: &ExecutionContext) -> String {
        let mut result = input.to_string();

        for (key, value) in context.environment.iter() {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, &value.to_string());
        }

        result
    }

    async fn copy_files(
        &self,
        pattern: &str,
        dest: &PathBuf,
        excludes: &[String],
    ) -> ExecutorResult<()> {
        let matches = glob::glob(pattern).map_err(|e| {
            crate::ExecutorError::from(crate::ExecutorErrorKind::StepFailed {
                reason: format!("Glob pattern error: {}", e),
            })
        })?;

        for path in matches.flatten() {
            if excludes.iter().any(|e| path.to_string_lossy().contains(e)) {
                continue;
            }

            if path.is_file() {
                let dest_path = dest.join(path.file_name().unwrap_or_default());
                tokio::fs::copy(&path, &dest_path).await?;
            }
        }

        Ok(())
    }

    async fn copy_all(&self, from: &PathBuf, to: &PathBuf) -> ExecutorResult<()> {
        if from.is_dir() {
            let mut stack = vec![(from.clone(), to.clone())];
            while let Some((src, dest)) = stack.pop() {
                if src.is_dir() {
                    tokio::fs::create_dir_all(&dest).await?;
                    let mut entries = tokio::fs::read_dir(&src).await?;
                    while let Some(entry) = entries.next_entry().await? {
                        let entry_dest = dest.join(entry.file_name());
                        stack.push((entry.path(), entry_dest));
                    }
                } else {
                    tokio::fs::copy(&src, &dest).await?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_step() -> Step {
        Step {
            step_type: StepType::Echo {
                message: "test".to_string(),
            },
            name: Some("test-step".to_string()),
            timeout: None,
            retry: None,
        }
    }

    #[tokio::test]
    async fn test_echo_execution() {
        let step = create_test_step();
        let mut context = ExecutionContext::new();
        let executor = StepExecutor::new();

        let result = executor.execute(&step, &mut context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ExecutionStatus::Success);
    }

    #[tokio::test]
    async fn test_shell_execution_success() {
        let step = Step {
            step_type: StepType::Shell {
                command: "echo hello".to_string(),
            },
            name: Some("shell-step".to_string()),
            timeout: None,
            retry: None,
        };
        let mut context = ExecutionContext::new();
        let executor = StepExecutor::new();

        let result = executor.execute(&step, &mut context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_variable_resolution() {
        let mut context = ExecutionContext::new();
        context.environment.insert("FOO", "bar");

        let executor = StepExecutor::new();
        let result = executor.resolve_variables("${FOO}", &context);
        assert_eq!(result, "bar");
    }
}
