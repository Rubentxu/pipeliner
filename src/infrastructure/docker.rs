//! Docker executor
//!
//! Executes pipeline stages inside Docker containers.

use crate::executor::{ExecutorCapabilities, HealthStatus, PipelineContext, PipelineExecutor};
use crate::pipeline::{Pipeline, Stage, StageResult, Step, StepType, Validate};
use std::process::Command;
use std::time::Instant;

/// Executor that runs stages inside Docker containers
#[derive(Debug, Clone)]
pub struct DockerExecutor {
    /// Default image to use
    default_image: String,
}

impl DockerExecutor {
    /// Creates a new Docker executor
    #[must_use]
    pub fn new() -> Self {
        Self {
            default_image: "rust:latest".to_string(),
        }
    }

    /// Sets the default image
    #[must_use]
    pub fn with_default_image(mut self, image: impl Into<String>) -> Self {
        self.default_image = image.into();
        self
    }

    /// Checks if Docker is available
    fn is_docker_available(&self) -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Runs a command inside a Docker container
    fn run_in_container(
        &self,
        image: &str,
        command: &str,
        context: &PipelineContext,
    ) -> Result<(), crate::pipeline::PipelineError> {
        let mut cmd = Command::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("-e")
            .arg(format!("RUSTLINE_STAGE={}", context.pipeline_id));

        for (key, value) in &context.env {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }

        let cwd_str = context.cwd.to_string_lossy().into_owned();
        cmd.arg("-w").arg(&cwd_str);
        cmd.arg("-v").arg(format!("{}:{}", cwd_str, cwd_str));
        cmd.arg(image).arg("sh").arg("-c").arg(command);

        let output = cmd
            .output()
            .map_err(|e| crate::pipeline::PipelineError::Io(e.to_string()))?;

        if !output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }

        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }

        if !output.status.success() {
            return Err(crate::pipeline::PipelineError::CommandFailed {
                code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }
}

impl Default for DockerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineExecutor for DockerExecutor {
    fn execute(&self, pipeline: &Pipeline) -> Result<StageResult, crate::pipeline::PipelineError> {
        let pipeline_id = pipeline
            .name
            .clone()
            .unwrap_or_else(|| "unnamed".to_string());

        tracing::info!(pipeline_id = %pipeline_id, "Starting Docker pipeline execution");

        let mut context = PipelineContext::new();

        for (key, value) in &pipeline.environment.vars {
            context.set_env(key, value);
        }

        for stage in &pipeline.stages {
            let stage_name = stage.name.clone();
            tracing::info!(stage = %stage_name, "Executing stage in Docker");

            let start = Instant::now();

            let result = self.execute_stage(stage, &context)?;

            let duration = start.elapsed();
            tracing::info!(
                stage = %stage_name,
                result = %result,
                duration_ms = duration.as_millis(),
                "Stage completed"
            );

            context.record_stage_result(&stage_name, result);

            if result.is_failure() && pipeline.options.retry.is_none() {
                tracing::error!(stage = %stage_name, "Stage failed, stopping pipeline");
                return Ok(result);
            }
        }

        Ok(StageResult::Success)
    }

    fn validate(&self, pipeline: &Pipeline) -> Result<(), crate::pipeline::ValidationError> {
        pipeline.validate()
    }

    fn dry_run(&self, pipeline: &Pipeline) -> Result<StageResult, crate::pipeline::PipelineError> {
        tracing::info!(
            pipeline = %pipeline.name.clone().unwrap_or_default(),
            "Starting dry run"
        );

        pipeline
            .validate()
            .map_err(crate::pipeline::PipelineError::Validation)?;

        for stage in &pipeline.stages {
            tracing::info!(stage = %stage.name, "Would execute stage in Docker");
            for step in &stage.steps {
                tracing::debug!(step = %step.step_type, "Would execute step");
            }
        }

        Ok(StageResult::Success)
    }

    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            can_execute_shell: true,
            can_run_docker: self.is_docker_available(),
            can_run_kubernetes: false,
            supports_parallel: false,
            supports_caching: false,
            supports_timeout: true,
            supports_retry: true,
        }
    }

    fn health_check(&self) -> HealthStatus {
        if !self.is_docker_available() {
            return HealthStatus::Unhealthy {
                reason: "Docker is not available".to_string(),
            };
        }

        let output = Command::new("docker").arg("info").output();

        match output {
            Ok(o) if o.status.success() => HealthStatus::Healthy,
            Ok(_) => HealthStatus::Degraded {
                reason: "Docker daemon may not be running".to_string(),
            },
            Err(e) => HealthStatus::Unhealthy {
                reason: format!("Docker error: {}", e),
            },
        }
    }
}

impl DockerExecutor {
    fn execute_stage(
        &self,
        stage: &Stage,
        context: &PipelineContext,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        let image = match stage.agent {
            Some(crate::pipeline::AgentType::Docker(ref config)) => &config.image,
            Some(_) => &self.default_image,
            None => &self.default_image,
        };

        let result = self.execute_steps(&stage.steps, image, context)?;

        for post in &stage.post {
            if post.should_execute(result, None) {
                self.execute_steps(post.steps(), image, context)?;
            }
        }

        Ok(result)
    }

    fn execute_steps(
        &self,
        steps: &[Step],
        image: &str,
        context: &PipelineContext,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        for step in steps {
            self.execute_step(step, image, context)?;
        }
        Ok(StageResult::Success)
    }

    fn execute_step(
        &self,
        step: &Step,
        image: &str,
        context: &PipelineContext,
    ) -> Result<(), crate::pipeline::PipelineError> {
        match &step.step_type {
            StepType::Shell { command } => {
                self.run_in_container(image, command, context)?;
            }
            StepType::Echo { message } => {
                println!("{message}");
            }
            StepType::Retry { count, step: inner } => {
                let mut last_error = None;
                let mut succeeded = false;
                for _ in 0..*count {
                    match self.execute_step(inner.as_ref(), image, context) {
                        Ok(()) => {
                            succeeded = true;
                            break;
                        }
                        Err(e) => {
                            last_error = Some(e);
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                }
                if !succeeded {
                    return Err(last_error.unwrap());
                }
            }
            StepType::Timeout {
                duration,
                step: inner,
            } => {
                let (tx, rx) = std::sync::mpsc::channel();
                let image = image.to_string();
                let context = context.clone();

                let executor = self.clone();
                let inner_step = inner.clone();

                std::thread::spawn(move || {
                    let result = executor.execute_step(&inner_step, &image, &context);
                    let _ = tx.send(result);
                });

                match rx.recv_timeout(*duration) {
                    Ok(Ok(())) => return Ok(()),
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        return Err(crate::pipeline::PipelineError::Timeout {
                            duration: *duration,
                        });
                    }
                }
            }
            _ => {
                tracing::warn!(step_type = %step.step_type, "Step type not implemented in Docker");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{AgentType, DockerConfig, Pipeline, Stage, Step};

    #[test]
    fn test_docker_executor_creation() {
        let executor = DockerExecutor::new();
        assert_eq!(executor.default_image, "rust:latest");
    }

    #[test]
    fn test_docker_executor_with_custom_image() {
        let executor = DockerExecutor::new().with_default_image("rust:1.70");
        assert_eq!(executor.default_image, "rust:1.70");
    }

    #[test]
    fn test_docker_executor_capabilities() {
        let executor = DockerExecutor::new();
        let caps = executor.capabilities();

        assert!(caps.can_execute_shell);
        assert!(caps.supports_timeout);
        assert!(caps.supports_retry);
    }

    #[test]
    fn test_docker_executor_health_check() {
        let executor = DockerExecutor::new();
        let health = executor.health_check();

        assert!(
            matches!(health, HealthStatus::Healthy)
                || matches!(health, HealthStatus::Unhealthy { .. })
        );
    }

    #[test]
    fn test_docker_executor_dry_run() {
        let executor = DockerExecutor::new();
        let pipeline = Pipeline::builder()
            .agent(AgentType::Docker(DockerConfig {
                image: "rust:1.70".to_string(),
                registry: None,
                args: Vec::new(),
                environment: std::collections::HashMap::new(),
            }))
            .stages(vec![Stage::new("Build", vec![Step::shell("cargo build")])])
            .build_unchecked();

        let result = executor.dry_run(&pipeline);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StageResult::Success);
    }

    #[test]
    fn test_docker_executor_validates_pipeline() {
        let executor = DockerExecutor::new();
        let pipeline = Pipeline::builder()
            .agent(AgentType::Docker(DockerConfig {
                image: "rust:1.70".to_string(),
                registry: None,
                args: Vec::new(),
                environment: std::collections::HashMap::new(),
            }))
            .stages(vec![Stage::new("Build", vec![Step::shell("cargo build")])])
            .build_unchecked();

        let result = executor.validate(&pipeline);

        assert!(result.is_ok());
    }

    #[test]
    fn test_docker_executor_rejects_empty_pipeline() {
        let executor = DockerExecutor::new();
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![])
            .build_unchecked();

        let result = executor.validate(&pipeline);

        assert!(result.is_err());
    }

    #[test]
    fn test_docker_stage_with_docker_agent() {
        let stage = Stage::new("Build", vec![Step::shell("cargo build")]).with_agent(
            AgentType::Docker(DockerConfig {
                image: "rust:1.70".to_string(),
                registry: None,
                args: Vec::new(),
                environment: std::collections::HashMap::new(),
            }),
        );

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let executor = DockerExecutor::new();
        let result = executor.validate(&pipeline);

        assert!(result.is_ok());
    }

    #[test]
    fn test_docker_executor_with_environment() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Docker(DockerConfig {
                image: "rust:1.70".to_string(),
                registry: None,
                args: Vec::new(),
                environment: std::collections::HashMap::new(),
            }))
            .stages(vec![Stage::new(
                "Build",
                vec![Step::shell("echo $RUST_VERSION")],
            )])
            .environment(|e| e.set("RUST_VERSION", "1.70"))
            .build_unchecked();

        let executor = DockerExecutor::new();
        let result = executor.validate(&pipeline);

        assert!(result.is_ok());
    }
}
