//! # Local Executor
//!
//! Local pipeline executor for development and testing.
//! Provides a simple way to run pipelines on the current machine.

use pipeliner_core::{Pipeline, Step, StepType};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Local execution result
#[derive(Debug, Clone)]
pub struct LocalResult {
    pub success: bool,
    pub stage: String,
    pub output: String,
    pub duration_ms: u64,
}

/// Local executor for running pipelines on the current machine
#[derive(Debug)]
pub struct LocalExecutor {
    _private: (),
}

impl LocalExecutor {
    /// Creates a new local executor
    #[must_use]
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Execute a single step
    pub async fn execute_step(&self, step: &Step) -> LocalResult {
        Box::pin(self._execute_step_impl(step)).await
    }

    async fn _execute_step_impl(&self, step: &Step) -> LocalResult {
        let start = std::time::Instant::now();
        let step_name = step.name.clone().unwrap_or_else(|| "unnamed".to_string());

        match &step.step_type {
            StepType::Shell { command } => {
                info!("[{}] Running: {}", step_name, command);
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output();

                match output {
                    Ok(output) => {
                        let success = output.status.success();
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                        if success {
                            info!("[{}] Success", step_name);
                            debug!("Output: {}", stdout.trim());
                        } else {
                            error!("[{}] Failed", step_name);
                            error!("Error: {}", stderr.trim());
                        }

                        LocalResult {
                            success,
                            stage: step_name,
                            output: if !stdout.is_empty() { stdout } else { stderr },
                            duration_ms: start.elapsed().as_millis() as u64,
                        }
                    }
                    Err(e) => LocalResult {
                        success: false,
                        stage: step_name,
                        output: e.to_string(),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
            StepType::Echo { message } => {
                info!("[{}] {}", step_name, message);
                LocalResult {
                    success: true,
                    stage: step_name,
                    output: message.clone(),
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            StepType::Retry { count, step: inner } => {
                let mut last_error = String::new();
                for attempt in 0..*count {
                    info!("[{}] Retry attempt {}/{}", step_name, attempt + 1, count);
                    let result = self.execute_step(inner.as_ref()).await;
                    if result.success {
                        return result;
                    }
                    last_error = result.output.clone();
                    sleep(Duration::from_secs(1)).await;
                }
                LocalResult {
                    success: false,
                    stage: step_name,
                    output: format!("Retry failed after {} attempts: {}", count, last_error),
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            StepType::Timeout {
                duration,
                step: inner,
            } => {
                let result =
                    tokio::time::timeout(*duration, self.execute_step(inner.as_ref())).await;

                match result {
                    Ok(r) => r,
                    Err(_) => LocalResult {
                        success: false,
                        stage: step_name,
                        output: format!("Timeout after {} seconds", duration.as_secs()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
            _ => LocalResult {
                success: true,
                stage: step_name,
                output: "Step type not implemented for local execution".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Execute a pipeline
    pub async fn execute(&self, pipeline: &Pipeline) -> Vec<LocalResult> {
        info!("========================================");
        info!("   Pipeliner - Local Execution");
        info!("========================================");
        info!("Pipeline: {:?}", pipeline.name());
        info!("Stages: {}", pipeline.stages.len());
        info!("");

        let mut results = Vec::new();

        for (stage_idx, stage) in pipeline.stages.iter().enumerate() {
            info!(
                "[Stage {}/{}] {}",
                stage_idx + 1,
                pipeline.stages.len(),
                stage.name
            );
            info!("----------------------------------------");

            for (_step_idx, step) in stage.steps.iter().enumerate() {
                let result = self.execute_step(step).await;
                results.push(result.clone());

                if !result.success {
                    warn!("Pipeline aborted due to step failure");
                    return results;
                }
            }

            info!("");
        }

        let success_count = results.iter().filter(|r| r.success).count();
        let total_count = results.len();

        info!("========================================");
        info!("   Execution Complete");
        info!("========================================");
        info!("Steps: {}/{} successful", success_count, total_count);
        info!(
            "Total time: {}ms",
            results.iter().map(|r| r.duration_ms).sum::<u64>()
        );

        results
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::Pipeline;

    #[tokio::test]
    async fn test_echo_step() {
        let executor = LocalExecutor::new();
        let step = Step::echo("Hello from test").with_name("test-echo");
        let result = executor.execute_step(&step).await;
        assert!(result.success);
        assert_eq!(result.stage, "test-echo");
    }

    #[tokio::test]
    async fn test_simple_pipeline() {
        let executor = LocalExecutor::new();
        let pipeline = Pipeline::new().with_name("test-pipeline");

        let results = executor.execute(&pipeline).await;
        assert_eq!(results.len(), 0); // No stages in test pipeline
    }
}
