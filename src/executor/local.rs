use super::shell::{ShellCommand, ShellConfig};
use super::traits::{ExecutorCapabilities, HealthStatus, PipelineContext, PipelineExecutor};
use crate::pipeline::{Pipeline, Stage, StageResult, Step, StepType, Validate};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

/// Local executor that runs commands on host system
#[derive(Debug, Clone)]
pub struct LocalExecutor {
    /// Configuration for executor
    config: ExecutorConfig,
}

/// Configuration for local executor
#[derive(Debug, Clone, Default)]
pub struct ExecutorConfig {
    /// Current working directory
    pub cwd: std::path::PathBuf,

    /// Environment variables
    pub env: std::collections::HashMap<String, String>,

    /// Shell to use (default: sh)
    pub shell: String,
}

impl LocalExecutor {
    /// Creates a new local executor
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ExecutorConfig::default(),
        }
    }

    /// Sets current working directory
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<std::path::PathBuf>) -> Self {
        self.config.cwd = cwd.into();
        self
    }

    /// Adds an environment variable
    #[must_use]
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.env.insert(key.into(), value.into());
        self
    }

    /// Sets shell to use
    #[must_use]
    pub fn with_shell(mut self, shell: impl Into<String>) -> Self {
        self.config.shell = shell.into();
        self
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineExecutor for LocalExecutor {
    fn execute(&self, pipeline: &Pipeline) -> Result<StageResult, crate::pipeline::PipelineError> {
        let pipeline_id = pipeline
            .name
            .clone()
            .unwrap_or_else(|| "unnamed".to_string());
        tracing::info!(
            pipeline_id = %pipeline_id,
            stages_count = pipeline.stages.len(),
            "Starting pipeline execution"
        );

        let mut context = PipelineContext::new();

        // Set environment variables from pipeline
        for (key, value) in &pipeline.environment.vars {
            context.set_env(key, value);
        }

        // Execute each stage
        for stage in &pipeline.stages {
            let stage_name = stage.name.clone();
            tracing::info!(stage = %stage_name, "Executing stage");

            let start = Instant::now();

            // Execute stage
            let result = self.execute_stage(stage, &context)?;

            let duration = start.elapsed();
            tracing::info!(
                stage = %stage_name,
                result = %result,
                duration_ms = duration.as_millis(),
                "Stage completed"
            );

            // Record result
            context.record_stage_result(&stage_name, result);

            // If stage failed and no retry, stop pipeline
            if result.is_failure() && pipeline.options.retry.is_none() {
                tracing::error!(stage = %stage_name, "Stage failed, stopping pipeline");
                return Ok(result);
            }
        }

        // Execute post-conditions
        for post in &pipeline.post {
            if let Some(last_result) = context.stage_results.values().last().copied()
                && post.should_execute(last_result, None)
            {
                self.execute_steps(post.steps(), &context)?;
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

        // Validate pipeline first
        pipeline
            .validate()
            .map_err(crate::pipeline::PipelineError::Validation)?;

        // Simulate execution without side effects
        for stage in &pipeline.stages {
            tracing::info!(stage = %stage.name, "Would execute stage with {} steps", stage.steps.len());
            for step in &stage.steps {
                tracing::debug!(step = %step.step_type, "Would execute step");
            }
        }

        // Simulate post-conditions
        for post in &pipeline.post {
            tracing::debug!(post = %post, "Would execute post-condition");
        }

        Ok(StageResult::Success)
    }

    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            can_execute_shell: true,
            can_run_docker: false,
            can_run_kubernetes: false,
            supports_parallel: true,
            supports_caching: false,
            supports_timeout: true,
            supports_retry: true,
        }
    }

    fn health_check(&self) -> HealthStatus {
        // Check if shell is available
        let shell = if self.config.shell.is_empty() {
            "sh"
        } else {
            &self.config.shell
        };

        let result = Command::new(shell).arg("-c").arg("echo test").output();

        match result {
            Ok(output) if output.status.success() => HealthStatus::Healthy,
            Ok(_) => HealthStatus::Unhealthy {
                reason: "Shell command returned non-zero exit code".to_string(),
            },
            Err(e) => HealthStatus::Unhealthy {
                #[allow(clippy::uninlined_format_args)]
                reason: format!("Shell not available: {}", e),
            },
        }
    }
}

impl LocalExecutor {
    /// Executes a single stage
    fn execute_stage(
        &self,
        stage: &Stage,
        context: &PipelineContext,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        // Handle matrix configuration - generate parallel branches
        if let Some(ref matrix) = stage.matrix {
            let combinations = matrix.generate_combinations();
            let mut branches = Vec::new();

            for combo in &combinations {
                let branch_name = combo
                    .iter()
                    .map(|(k, v)| format!("{}_{}", k, v))
                    .collect::<Vec<_>>()
                    .join("-");

                let mut branch_steps = stage.steps.clone();

                for (key, value) in combo {
                    branch_steps.insert(0, Step::shell(&format!("export {}={}", key, value)));
                }

                branches.push(crate::pipeline::ParallelBranch {
                    name: branch_name.clone(),
                    stage: Stage::new(branch_name, branch_steps),
                });
            }

            return self.execute_parallel_branches(&branches, context);
        }

        // Execute parallel branches if present
        if !stage.parallel.is_empty() {
            self.execute_parallel_branches(&stage.parallel, context)?;
        }

        // Execute steps (only if no parallel branches, or after parallel)
        if !stage.parallel.is_empty() || !stage.steps.is_empty() {
            let result = self.execute_steps(&stage.steps, context)?;

            // Execute post-conditions
            for post in &stage.post {
                if post.should_execute(result, None) {
                    self.execute_steps(post.steps(), context)?;
                }
            }

            return Ok(result);
        }

        // Execute post-conditions for parallel-only stage
        let result = StageResult::Success;
        for post in &stage.post {
            if post.should_execute(result, None) {
                self.execute_steps(post.steps(), context)?;
            }
        }

        Ok(result)
    }

    /// Executes parallel branches concurrently
    fn execute_parallel_branches(
        &self,
        branches: &[crate::pipeline::ParallelBranch],
        context: &PipelineContext,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let results = Arc::new(Mutex::new(Vec::new()));
        let context = Arc::new(context.clone());

        let handles: Vec<_> = branches
            .iter()
            .map(|branch| {
                let results = Arc::clone(&results);
                let context = Arc::clone(&context);
                let branch_name = branch.name.clone();
                let stage = branch.stage.clone();

                thread::spawn(move || {
                    let result = Self::execute_stage_static(&stage, &context);
                    let mut results = results.lock().unwrap();
                    results.push((branch_name, result));
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let results = results.lock().unwrap();

        let has_error = results.iter().any(|(_, r)| r.is_err());
        if has_error {
            let first_error = results.iter().find(|(_, r)| r.is_err()).unwrap().1.clone();
            return first_error;
        }

        Ok(StageResult::Success)
    }

    /// Static method to execute a stage
    fn execute_stage_static(
        stage: &Stage,
        context: &Arc<PipelineContext>,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        if !stage.parallel.is_empty() {
            Self::execute_parallel_branches_static(&stage.parallel, context)?;
        }

        if !stage.parallel.is_empty() || !stage.steps.is_empty() {
            let result = Self::execute_steps_static(&stage.steps, context)?;
            return Ok(result);
        }

        Ok(StageResult::Success)
    }

    /// Static method to execute parallel branches
    fn execute_parallel_branches_static(
        branches: &[crate::pipeline::ParallelBranch],
        context: &Arc<PipelineContext>,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let results = Arc::new(Mutex::new(Vec::new()));

        let handles: Vec<_> = branches
            .iter()
            .map(|branch| {
                let results = Arc::clone(&results);
                let context = Arc::clone(&context);
                let branch_name = branch.name.clone();
                let stage = branch.stage.clone();

                thread::spawn(move || {
                    let result = Self::execute_stage_static(&stage, &context);
                    let mut results = results.lock().unwrap();
                    results.push((branch_name, result));
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let results = results.lock().unwrap();

        let has_error = results.iter().any(|(_, r)| r.is_err());
        if has_error {
            let first_error = results.iter().find(|(_, r)| r.is_err()).unwrap().1.clone();
            return first_error;
        }

        Ok(StageResult::Success)
    }

    /// Static method to execute steps
    fn execute_steps_static(
        steps: &[Step],
        context: &Arc<PipelineContext>,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        for step in steps {
            Self::execute_step_static(step, context)?;
        }
        Ok(StageResult::Success)
    }

    /// Static method to execute a single step
    fn execute_step_static(
        step: &Step,
        context: &Arc<PipelineContext>,
    ) -> Result<(), crate::pipeline::PipelineError> {
        match &step.step_type {
            StepType::Shell { command } => {
                let shell_config = ShellConfig {
                    cwd: context.cwd.clone(),
                    env: context.env.clone(),
                    shell: "sh".to_string(),
                    streaming: false,
                    timeout: None,
                };

                let shell_command = ShellCommand::new(&shell_config);
                let result = shell_command.execute(command)?;

                if !result.is_success() {
                    return Err(crate::pipeline::PipelineError::CommandFailed {
                        code: result.exit_code,
                        stderr: result.stderr,
                    });
                }
            }
            StepType::Echo { message } => {
                println!("{message}");
            }
            _ => {}
        }
        Ok(())
    }

    /// Executes a list of steps
    fn execute_steps(
        &self,
        steps: &[Step],
        context: &PipelineContext,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        for step in steps {
            self.execute_step(step, context)?;
        }
        Ok(StageResult::Success)
    }

    /// Executes a single step
    fn execute_step(
        &self,
        step: &Step,
        context: &PipelineContext,
    ) -> Result<(), crate::pipeline::PipelineError> {
        match &step.step_type {
            StepType::Shell { command } => {
                self.execute_shell(command, context)?;
            }
            StepType::Echo { message } => {
                println!("{message}");
            }
            StepType::Retry { count, step } => {
                let mut last_error = None;
                let mut succeeded = false;
                for attempt in 0..*count {
                    match self.execute_step(step.as_ref(), context) {
                        Ok(()) => {
                            succeeded = true;
                            break;
                        }
                        Err(e) => {
                            last_error = Some(e);
                            tracing::warn!(
                                attempt = attempt + 1,
                                total = count,
                                "Step failed, retrying"
                            );
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                }
                if !succeeded {
                    return Err(last_error.unwrap());
                }
            }
            StepType::Timeout { duration, step } => {
                self.execute_timeout(*duration, step.as_ref(), context)?;
            }
            _ => {
                tracing::warn!(step_type = %step.step_type, "Step type not yet implemented");
            }
        }
        Ok(())
    }

    /// Executes a shell command
    fn execute_shell(
        &self,
        command: &str,
        context: &PipelineContext,
    ) -> Result<(), crate::pipeline::PipelineError> {
        let shell_config = ShellConfig {
            cwd: context.cwd.clone(),
            env: context.env.clone(),
            shell: if self.config.shell.is_empty() {
                "sh".to_string()
            } else {
                self.config.shell.clone()
            },
            streaming: false,
            timeout: None,
        };

        let shell_command = ShellCommand::new(&shell_config);
        let result = shell_command.execute(command)?;

        if !result.is_success() {
            return Err(crate::pipeline::PipelineError::CommandFailed {
                code: result.exit_code,
                stderr: result.stderr,
            });
        }

        Ok(())
    }

    /// Executes a step with timeout
    fn execute_timeout(
        &self,
        duration: std::time::Duration,
        step: &Step,
        context: &PipelineContext,
    ) -> Result<(), crate::pipeline::PipelineError> {
        let (tx, rx) = std::sync::mpsc::channel();

        let executor = self.clone();
        let step = Arc::new(step.clone());
        let context = Arc::new(context.clone());

        std::thread::spawn(move || {
            let result = executor.execute_step_arc(&step, &context);
            let _ = tx.send(result);
        });

        match rx.recv_timeout(duration) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(crate::pipeline::PipelineError::Timeout { duration }),
        }
    }

    /// Executes a step with Arc references
    fn execute_step_arc(
        &self,
        step: &Arc<Step>,
        context: &Arc<PipelineContext>,
    ) -> Result<(), crate::pipeline::PipelineError> {
        match &step.step_type {
            StepType::Shell { command } => {
                let shell_config = ShellConfig {
                    cwd: context.cwd.clone(),
                    env: context.env.clone(),
                    shell: if self.config.shell.is_empty() {
                        "sh".to_string()
                    } else {
                        self.config.shell.clone()
                    },
                    streaming: false,
                    timeout: None,
                };

                let shell_command = ShellCommand::new(&shell_config);
                let result = shell_command.execute(command)?;

                if !result.is_success() {
                    return Err(crate::pipeline::PipelineError::CommandFailed {
                        code: result.exit_code,
                        stderr: result.stderr,
                    });
                }
            }
            StepType::Echo { message } => {
                println!("{message}");
            }
            StepType::Retry { count, step } => {
                let mut last_error = None;
                let mut succeeded = false;
                let step_arc = Arc::new(step.as_ref().clone());
                for _ in 0..*count {
                    match self.execute_step_arc(&step_arc, context) {
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
            StepType::Timeout { duration, step } => {
                self.execute_timeout_arc(*duration, step, context)?;
            }
            _ => {
                tracing::warn!(step_type = %step.step_type, "Step type not yet implemented");
            }
        }
        Ok(())
    }

    /// Executes a step with timeout using Arc references
    fn execute_timeout_arc(
        &self,
        duration: std::time::Duration,
        step: &Step,
        context: &Arc<PipelineContext>,
    ) -> Result<(), crate::pipeline::PipelineError> {
        let (tx, rx) = std::sync::mpsc::channel();

        let executor = self.clone();
        let step = Arc::new(step.clone());
        let context = Arc::clone(context);

        std::thread::spawn(move || {
            let result = executor.execute_step_arc(&step, &context);
            let _ = tx.send(result);
        });

        match rx.recv_timeout(duration) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(crate::pipeline::PipelineError::Timeout { duration }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::AgentType;

    #[test]
    fn test_local_executor_creation() {
        let executor = LocalExecutor::new();
        let caps = executor.capabilities();

        assert!(caps.can_execute_shell);
        assert!(!caps.can_run_docker);
        assert!(!caps.can_run_kubernetes);
    }

    #[test]
    fn test_local_executor_health() {
        let executor = LocalExecutor::new();
        let health = executor.health_check();

        assert!(health.is_operational());
    }

    #[test]
    fn test_pipeline_context_creation() {
        let context = PipelineContext::new();

        assert!(!context.env.is_empty());
        assert!(context.stage_results.is_empty());
    }

    #[test]
    fn test_pipeline_context_set_get_env() {
        let mut context = PipelineContext::new();

        context.set_env("TEST_VAR", "test_value");

        assert_eq!(context.get_env("TEST_VAR"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_pipeline_context_record_stage_result() {
        let mut context = PipelineContext::new();

        context.record_stage_result("Build", StageResult::Success);

        assert_eq!(
            context.get_stage_result("Build"),
            Some(&StageResult::Success)
        );
    }

    #[test]
    fn test_local_executor_validate_valid_pipeline() {
        let executor = LocalExecutor::new();
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![Step::shell("echo test")])])
            .build_unchecked();

        let result = executor.validate(&pipeline);

        assert!(result.is_ok());
    }

    #[test]
    fn test_local_executor_validate_invalid_pipeline() {
        let executor = LocalExecutor::new();
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![])
            .build_unchecked();

        let result = executor.validate(&pipeline);

        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_validation_empty_stages() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![])
            .build_unchecked();

        let result = pipeline.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Pipeline must have at least one stage");
    }

    #[test]
    fn test_pipeline_validation_invalid_agent_label() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Label("".to_string()))
            .stages(vec![Stage::new("Build", vec![Step::shell("echo test")])])
            .build_unchecked();

        let result = pipeline.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Label cannot be empty"));
    }

    #[test]
    fn test_pipeline_validation_valid_pipeline() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Label("linux".to_string()))
            .stages(vec![Stage::new("Build", vec![Step::shell("echo test")])])
            .build_unchecked();

        let result = pipeline.validate();

        assert!(result.is_ok());
    }

    #[test]
    fn test_local_executor_dry_run() {
        let executor = LocalExecutor::new();
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![Step::shell("echo test")])])
            .build_unchecked();

        let result = executor.dry_run(&pipeline);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StageResult::Success);
    }

    #[test]
    fn test_retry_step_success_first_attempt() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let step = Step::retry(3, Step::shell("echo success"));

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![step])])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn test_retry_step_all_fail() {
        let executor = LocalExecutor::new();
        let step = Step::retry(2, Step::shell("exit 1"));

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![step])])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_err());
    }

    #[test]
    fn test_timeout_step_completes_in_time() {
        let executor = LocalExecutor::new();
        let step = Step::timeout(std::time::Duration::from_secs(5), Step::shell("echo quick"));

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![step])])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn test_timeout_step_times_out() {
        let executor = LocalExecutor::new();
        let step = Step::timeout(
            std::time::Duration::from_millis(100),
            Step::shell("sleep 10"),
        );

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![step])])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_err());
    }

    #[test]
    fn test_parallel_execution_success() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let branch1 = crate::pipeline::ParallelBranch {
            name: "Branch1".to_string(),
            stage: Stage::new("Build1", vec![Step::shell("echo branch1")]),
        };
        let branch2 = crate::pipeline::ParallelBranch {
            name: "Branch2".to_string(),
            stage: Stage::new("Build2", vec![Step::shell("echo branch2")]),
        };

        let stage = Stage::new("Parallel", vec![]).with_parallel(vec![branch1, branch2]);

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parallel_execution_one_branch_fails() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let branch1 = crate::pipeline::ParallelBranch {
            name: "Branch1".to_string(),
            stage: Stage::new("Build1", vec![Step::shell("echo branch1")]),
        };
        let branch2 = crate::pipeline::ParallelBranch {
            name: "Branch2".to_string(),
            stage: Stage::new("Build2", vec![Step::shell("exit 1")]),
        };

        let stage = Stage::new("Parallel", vec![]).with_parallel(vec![branch1, branch2]);

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_err());
    }

    #[test]
    fn test_parallel_execution_single_branch() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let branch = crate::pipeline::ParallelBranch {
            name: "SingleBranch".to_string(),
            stage: Stage::new("Build", vec![Step::shell("echo single")]),
        };

        let stage = Stage::new("Parallel", vec![]).with_parallel(vec![branch]);

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stage_with_parallel_and_steps() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let branch = crate::pipeline::ParallelBranch {
            name: "Branch1".to_string(),
            stage: Stage::new("Build1", vec![Step::shell("echo parallel")]),
        };

        let stage = Stage::new("Mixed", vec![Step::shell("echo before")])
            .with_parallel(vec![branch])
            .with_post(crate::pipeline::PostCondition::always(vec![Step::shell(
                "echo after",
            )]));

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matrix_execution_success() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let matrix = crate::pipeline::MatrixConfig::new()
            .add_axis("os", vec!["linux".to_string(), "macos".to_string()])
            .add_axis("version", vec!["1.0".to_string(), "2.0".to_string()]);

        let stage = Stage::new("Matrix", vec![Step::shell("echo test")]).with_matrix(matrix);

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matrix_execution_single_axis() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let matrix = crate::pipeline::MatrixConfig::new()
            .add_axis("os", vec!["linux".to_string(), "windows".to_string()]);

        let stage = Stage::new("Matrix", vec![Step::shell("echo single axis")]).with_matrix(matrix);

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matrix_execution_with_excludes() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let matrix = crate::pipeline::MatrixConfig::new()
            .add_axis("os", vec!["linux".to_string(), "macos".to_string()])
            .add_axis("version", vec!["1.0".to_string(), "2.0".to_string()])
            .add_exclude(vec![
                ("os".to_string(), "linux".to_string()),
                ("version".to_string(), "2.0".to_string()),
            ]);

        let stage = Stage::new("Matrix", vec![Step::shell("echo excluded")]).with_matrix(matrix);

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matrix_generates_correct_combinations() {
        let matrix = crate::pipeline::MatrixConfig::new()
            .add_axis("os", vec!["linux".to_string(), "macos".to_string()])
            .add_axis("arch", vec!["x64".to_string(), "arm64".to_string()]);

        let combos = matrix.generate_combinations();

        assert_eq!(combos.len(), 4);
        assert!(combos.contains(&vec![
            ("os".to_string(), "linux".to_string()),
            ("arch".to_string(), "x64".to_string())
        ]));
        assert!(combos.contains(&vec![
            ("os".to_string(), "linux".to_string()),
            ("arch".to_string(), "arm64".to_string())
        ]));
        assert!(combos.contains(&vec![
            ("os".to_string(), "macos".to_string()),
            ("arch".to_string(), "x64".to_string())
        ]));
        assert!(combos.contains(&vec![
            ("os".to_string(), "macos".to_string()),
            ("arch".to_string(), "arm64".to_string())
        ]));
    }

    #[test]
    fn test_variable_expansion_in_shell() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let step = Step::shell("echo ${MY_VAR}");

        let mut context = PipelineContext::new();
        context.set_env("MY_VAR", "test_value");

        let result = executor.execute_step(&step, &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_jenkins_variables_available() {
        let executor = LocalExecutor::new().with_cwd("/tmp");
        let step = Step::shell("echo ${BUILD_NUMBER}");

        let mut context = PipelineContext::new();
        context.set_env("BUILD_NUMBER", "42");

        let result = executor.execute_step(&step, &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shell_with_temp_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let executor = LocalExecutor::new().with_cwd(temp_dir.path());
        let step = Step::shell("echo test");

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Test", vec![step])])
            .build_unchecked();

        let result = executor.execute(&pipeline);
        assert!(result.is_ok());
    }
}
