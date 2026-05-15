//! Execution strategies for pipeline stages.
//!
//! This module provides different execution strategies for running
//! pipeline stages, including sequential and parallel execution.

use async_trait::async_trait;
use tokio::sync::Semaphore;
use tracing::{debug, info};

use pipeliner_core::{Pipeline, pipeline::StageOrParallel, Stage};

use crate::{ExecutionContext, ExecutionResult, ExecutionStatus, ExecutorResult};

/// Helper: get all stages from Vec<StageOrParallel> (owned clone)
fn flatten_stages(items: &[StageOrParallel]) -> Vec<Stage> {
    let mut stages = Vec::new();
    for item in items {
        match item {
            StageOrParallel::Stage(stage) => stages.push(stage.clone()),
            StageOrParallel::Parallel(group) => {
                for stage in &group.stages {
                    stages.push(stage.clone());
                }
            }
        }
    }
    stages
}

/// Execution strategy trait
#[async_trait]
pub trait ExecutionStrategy: Send + Sync {
    /// Executes the pipeline according to this strategy
    async fn execute(
        &self,
        pipeline: &Pipeline,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionResult>;
}

/// Sequential execution strategy
#[derive(Debug, Default)]
pub struct SequentialStrategy;

impl SequentialStrategy {
    /// Creates a new sequential strategy
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecutionStrategy for SequentialStrategy {
    async fn execute(
        &self,
        pipeline: &Pipeline,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionResult> {
        let start_time = chrono::Utc::now();
        let mut stages_executed = 0;
        let mut steps_executed = 0;

        info!("Starting sequential execution of pipeline: {:?}", pipeline.name());

        // Flatten all stages from StageOrParallel items
        let all_stages = flatten_stages(&pipeline.stages);
        
        for stage in &all_stages {
            context.set_current_stage(&stage.name);
            
            let result = execute_stage(stage).await;

            stages_executed += 1;
            steps_executed += stage.steps.len();

            match &result {
                Ok(ExecutionStatus::Success) => {
                    debug!("Stage '{}' completed successfully", stage.name);
                }
                Ok(ExecutionStatus::Unstable) => {
                    debug!("Stage '{}' completed with unstable status", stage.name);
                }
                Ok(status) => {
                    let duration = chrono::Utc::now().signed_duration_since(start_time);
                    return Ok(ExecutionResult::failure(
                        stages_executed,
                        steps_executed,
                        duration,
                        format!("Stage '{}' failed with status: {:?}", stage.name, status),
                    ));
                }
                Err(e) => {
                    let duration = chrono::Utc::now().signed_duration_since(start_time);
                    return Ok(ExecutionResult::failure(
                        stages_executed,
                        steps_executed,
                        duration,
                        format!("Stage '{}' error: {}", stage.name, e),
                    ));
                }
            }

            context.clear_current_stage();
        }

        let duration = chrono::Utc::now().signed_duration_since(start_time);
        Ok(ExecutionResult::success(stages_executed, steps_executed, duration))
    }
}

/// Parallel execution strategy
#[derive(Debug, Default)]
pub struct ParallelStrategy {
    /// Maximum concurrent stages
    pub max_concurrent: usize,
}

impl ParallelStrategy {
    /// Creates a new parallel strategy
    #[must_use]
    pub fn new(max_concurrent: usize) -> Self {
        Self { max_concurrent }
    }
}

#[async_trait]
impl ExecutionStrategy for ParallelStrategy {
    async fn execute(
        &self,
        pipeline: &Pipeline,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionResult> {
        let start_time = chrono::Utc::now();
        let mut stages_executed = 0;
        let mut steps_executed = 0;

        info!(
            "Starting parallel execution of pipeline: {:?} (max {} concurrent)",
            pipeline.name(),
            self.max_concurrent
        );

        let semaphore = std::sync::Arc::new(Semaphore::new(self.max_concurrent));
        let mut handles = Vec::new();

        // Flatten all stages
        let all_stages = flatten_stages(&pipeline.stages);

        for stage in &all_stages {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let stage = stage.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;
                execute_stage(&stage).await
            });

            handles.push(handle);
        }

        let mut has_failure = false;
        let mut failure_reason = None;

        for handle in handles {
            match handle.await {
                Ok(Ok(status)) => {
                    stages_executed += 1;
                    if !status.is_success() && !matches!(status, ExecutionStatus::Unstable) {
                        has_failure = true;
                    }
                }
                Ok(Err(e)) => {
                    has_failure = true;
                    failure_reason = Some(e.to_string());
                }
                Err(e) => {
                    has_failure = true;
                    failure_reason = Some(e.to_string());
                }
            }
        }

        let duration = chrono::Utc::now().signed_duration_since(start_time);

        if has_failure {
            return Ok(ExecutionResult::failure(
                stages_executed,
                steps_executed,
                duration,
                failure_reason.unwrap_or_else(|| "One or more stages failed".to_string()),
            ));
        }

        Ok(ExecutionResult::success(stages_executed, steps_executed, duration))
    }
}

/// Matrix execution strategy
#[derive(Debug, Default)]
pub struct MatrixStrategy {
    /// Maximum concurrent cells
    pub max_concurrent: usize,
}

impl MatrixStrategy {
    /// Creates a new matrix strategy
    #[must_use]
    pub fn new(max_concurrent: usize) -> Self {
        Self { max_concurrent }
    }
}

#[async_trait]
impl ExecutionStrategy for MatrixStrategy {
    async fn execute(
        &self,
        pipeline: &Pipeline,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionResult> {
        let matrix = match &pipeline.matrix {
            Some(m) => m,
            None => {
                return SequentialStrategy::new().execute(pipeline, context).await;
            }
        };

        let cells = matrix.generate_cells();
        let start_time = chrono::Utc::now();

        info!("Starting matrix execution with {} cells", cells.len());

        let mut cells_executed = 0;
        let mut cells_failed = 0;

        for cell in &cells {
            let cell_values = cell.values.clone();
            let cell_name = cell.name.clone();

            context.set_metadata("matrix_cell", &cell_name);

            for (key, value) in &cell_values {
                context.set_parameter(key, value);
            }

            match SequentialStrategy::new().execute(pipeline, context).await {
                Ok(_) => cells_executed += 1,
                Err(_) => cells_failed += 1,
            }
        }

        let duration = chrono::Utc::now().signed_duration_since(start_time);

        if cells_failed > 0 {
            return Ok(ExecutionResult::failure(
                cells_executed,
                cells_failed,
                duration,
                format!("{} matrix cells failed", cells_failed),
            ));
        }

        Ok(ExecutionResult::success(cells_executed, cells_failed, duration))
    }
}

/// Executes a single stage
async fn execute_stage(stage: &Stage) -> ExecutorResult<ExecutionStatus> {
    use crate::runtime::{StepExecutor, StepExecutorTrait};
    
    let executor = StepExecutor::new();
    let mut context = ExecutionContext::new();
    
    context.set_current_stage(&stage.name);
    
    for step in &stage.steps {
        match executor.execute(step, &mut context, None).await {
            Ok(ExecutionStatus::Success) => continue,
            Ok(status) => return Ok(status),
            Err(e) => return Err(e),
        }
    }
    
    context.clear_current_stage();
    Ok(ExecutionStatus::Success)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::{pipeline::StageOrParallel, Pipeline, Stage, Step, StepType, agent::AgentType};

    fn create_test_pipeline() -> Pipeline {
        Pipeline::new()
            .with_name("Test")
            .with_agent(AgentType::any())
            .with_stage(Stage::new("Stage1").with_step(Step::echo("test")))
            .with_stage(Stage::new("Stage2").with_step(Step::echo("test2")))
    }

    #[tokio::test]
    async fn test_sequential_strategy() {
        let pipeline = create_test_pipeline();
        let mut context = ExecutionContext::new();
        let strategy = SequentialStrategy::new();

        let result = strategy.execute(&pipeline, &mut context).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_parallel_strategy() {
        let pipeline = create_test_pipeline();
        let mut context = ExecutionContext::new();
        let strategy = ParallelStrategy::new(10);

        let result = strategy.execute(&pipeline, &mut context).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_success());
    }

    #[test]
    fn test_parallel_strategy_new() {
        let strategy = ParallelStrategy::new(5);
        assert_eq!(strategy.max_concurrent, 5);
    }
}
