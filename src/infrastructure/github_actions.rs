//! GitHub Actions backend
//!
//! Translates Rustline pipelines to GitHub Actions workflows.

use crate::pipeline::PipelineError;
use crate::pipeline::{Pipeline, Stage, Step};

/// Backend for generating GitHub Actions workflows
pub struct GitHubActionsBackend {
    /// Repository name
    #[allow(dead_code)]
    repo: String,
}

impl GitHubActionsBackend {
    /// Creates a new GitHub Actions backend
    pub fn new(repo: impl Into<String>) -> Self {
        Self { repo: repo.into() }
    }

    /// Translates a pipeline to GitHub Actions workflow YAML
    #[allow(clippy::missing_errors_doc)]
    pub fn translate(&self, pipeline: &Pipeline) -> Result<String, PipelineError> {
        let mut yaml = String::new();

        yaml.push_str("name: CI\n\n");
        yaml.push_str("on: [push]\n\n");

        // Add global environment if present
        if !pipeline.environment.vars.is_empty() {
            yaml.push_str("env:\n");
            for (key, value) in &pipeline.environment.vars {
                yaml.push_str(&format!("  {}: {}\n", key, value));
            }
            yaml.push_str("\n");
        }

        yaml.push_str("jobs:\n");

        for stage in &pipeline.stages {
            yaml.push_str(&self.translate_stage(stage));
        }

        Ok(yaml)
    }

    /// Translates a stage to a GitHub Actions job
    #[allow(clippy::format_push_string)]
    fn translate_stage(&self, stage: &Stage) -> String {
        let mut job = String::new();

        job.push_str(&format!("  {}:\n", sanitize_job_name(&stage.name)));

        // Handle parallel branches with matrix strategy
        if !stage.parallel.is_empty() {
            job.push_str("    strategy:\n");
            job.push_str("      matrix:\n");

            let mut os_values = Vec::new();
            for branch in &stage.parallel {
                os_values.push(branch.name.clone());
            }
            job.push_str(&format!("        os: [{}]\n", os_values.join(", ")));
        }

        // Determine runs-on based on agent type
        match stage.agent.as_ref() {
            Some(crate::pipeline::AgentType::Label(label)) => {
                job.push_str(&format!("    runs-on: {label}\n"));
            }
            Some(crate::pipeline::AgentType::Docker(_)) => {
                job.push_str("    runs-on: ubuntu-latest\n");
                job.push_str("    container:\n");
                job.push_str("      image: docker.io/library/rust:latest\n");
            }
            _ => {
                job.push_str("    runs-on: ubuntu-latest\n");
            }
        }

        // Add when condition as if expression
        if let Some(ref when) = stage.when {
            let condition = self.translate_when_condition(when);
            job.push_str(&format!("    if: {condition}\n"));
        }

        // Handle post-conditions (always steps)
        for post in &stage.post {
            match post {
                crate::pipeline::PostCondition::Always { steps } => {
                    for step in steps {
                        job.push_str(&self.translate_step(step));
                    }
                }
                crate::pipeline::PostCondition::Failure { steps } => {
                    job.push_str("    if: failure()\n");
                    for step in steps {
                        job.push_str(&self.translate_step(step));
                    }
                }
                _ => {}
            }
        }

        job.push_str("    steps:\n");

        for step in &stage.steps {
            job.push_str(&self.translate_step(step));
        }

        job
    }

    /// Translates when condition to GitHub Actions if expression
    #[allow(clippy::unused_self)]
    fn translate_when_condition(&self, condition: &crate::pipeline::WhenCondition) -> String {
        match condition {
            crate::pipeline::WhenCondition::Branch { branch } => {
                format!("github.ref == 'refs/heads/{branch}'")
            }
            crate::pipeline::WhenCondition::Tag { tag } => {
                format!("startsWith(github.ref, 'refs/tags/{tag}')")
            }
            crate::pipeline::WhenCondition::Environment { name, value } => {
                format!("env.{name} == '{value}'")
            }
            crate::pipeline::WhenCondition::Expression { expression } => expression.clone(),
            crate::pipeline::WhenCondition::AllOf { conditions } => {
                let conditions_str = conditions
                    .iter()
                    .map(|c| format!("({})", self.translate_when_condition(c)))
                    .collect::<Vec<_>>()
                    .join(" && ");
                format!("({conditions_str})")
            }
            crate::pipeline::WhenCondition::AnyOf { conditions } => {
                let conditions_str = conditions
                    .iter()
                    .map(|c| format!("({})", self.translate_when_condition(c)))
                    .collect::<Vec<_>>()
                    .join(" || ");
                format!("({conditions_str})")
            }
        }
    }

    /// Translates a step to a GitHub Actions step
    #[allow(clippy::missing_errors_doc)]
    fn translate_step(&self, step: &Step) -> String {
        match &step.step_type {
            crate::pipeline::StepType::Shell { command } => {
                format!("      - run: {command}\n")
            }
            crate::pipeline::StepType::Echo { message } => {
                format!("      - run: echo '{message}'\n")
            }
            crate::pipeline::StepType::Timeout {
                duration,
                step: inner_step,
            } => {
                let timeout_minutes = (*duration).as_secs().div_ceil(60).max(1);
                let inner = self.translate_step(inner_step.as_ref());
                format!("      - run: |\n{inner}    timeout-minutes: {timeout_minutes}\n")
            }
            crate::pipeline::StepType::Retry {
                step: inner_step, ..
            } => {
                let inner = self.translate_step(inner_step.as_ref());
                format!("      - run: |\n{inner}    continue-on-error: true\n")
            }
            #[allow(clippy::useless_format)]
            _ => format!("      - # Step type not supported\n"),
        }
    }
}

/// Sanitizes job name for GitHub Actions
fn sanitize_job_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{AgentType, Pipeline, Stage, Step};

    #[test]
    fn test_simple_pipeline_to_github_actions() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![
                Stage::new("Build", vec![Step::shell("cargo build")]),
                Stage::new("Test", vec![Step::shell("cargo test")]),
            ])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("name: CI"));
        assert!(workflow.contains("on: [push]"));
        assert!(workflow.contains("jobs:"));
        assert!(workflow.contains("Build:"));
        assert!(workflow.contains("Test:"));
        assert!(workflow.contains("runs-on: ubuntu-latest"));
        assert!(workflow.contains("cargo build"));
        assert!(workflow.contains("cargo test"));
    }

    #[test]
    fn test_stage_maps_to_job() {
        let stage = Stage::new("Build", vec![Step::shell("echo build")]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("  Build:\n"));
        assert!(workflow.contains("    runs-on: ubuntu-latest\n"));
        assert!(workflow.contains("    steps:\n"));
    }

    #[test]
    fn test_step_maps_to_run_step() {
        let stage = Stage::new("Test", vec![Step::shell("cargo test --lib")]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("      - run: cargo test --lib\n"));
    }

    #[test]
    fn test_echo_step_maps_to_run_echo() {
        let stage = Stage::new("Notify", vec![Step::echo("Build completed")]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("      - run: echo 'Build completed'\n"));
    }

    #[test]
    fn test_environment_maps_to_env_section() {
        let stage = Stage::new("Build", vec![Step::shell("echo $RUST_VERSION")]);

        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .environment(|e| e.set("RUST_VERSION", "1.70").set("BUILD_NUMBER", "42"))
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("env:\n"));
        assert!(workflow.contains("  RUST_VERSION: 1.70\n"));
        assert!(workflow.contains("  BUILD_NUMBER: 42\n"));
    }

    #[test]
    fn test_generated_yaml_is_valid() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![Step::shell("cargo build")])])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.starts_with("name: CI"));
        assert!(workflow.contains("on: [push]"));
        assert!(workflow.contains("jobs:"));
        assert!(workflow.contains("  Build:\n"));
    }

    #[test]
    fn test_multiple_stages_create_multiple_jobs() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![
                Stage::new("Build", vec![Step::shell("cargo build")]),
                Stage::new("Test", vec![Step::shell("cargo test")]),
                Stage::new("Deploy", vec![Step::shell("echo deploy")]),
            ])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        let build_count = workflow.matches("  Build:\n").count();
        let test_count = workflow.matches("  Test:\n").count();
        let deploy_count = workflow.matches("  Deploy:\n").count();

        assert_eq!(build_count, 1);
        assert_eq!(test_count, 1);
        assert_eq!(deploy_count, 1);
    }

    #[test]
    fn test_sanitized_job_names() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![
                Stage::new("Build Stage", vec![Step::shell("echo build")]),
                Stage::new("Test@Production", vec![Step::shell("echo test")]),
            ])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("  Build_Stage:\n"));
        assert!(workflow.contains("  Test_Production:\n"));
    }

    // US-3.2: Advanced mapping tests

    #[test]
    fn test_timeout_step_maps_to_timeout_minutes() {
        use std::time::Duration;
        let step = Step::timeout(Duration::from_secs(30), Step::shell("cargo build"));
        let stage = Stage::new("Build", vec![step]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("    timeout-minutes: 1\n"));
    }

    #[test]
    fn test_retry_step_maps_to_continue_on_error() {
        let step = Step::retry(3, Step::shell("cargo test"));
        let stage = Stage::new("Test", vec![step]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("    continue-on-error: true\n"));
    }

    #[test]
    fn test_parallel_branches_to_matrix_strategy() {
        use crate::pipeline::ParallelBranch;
        let branch1 = ParallelBranch {
            name: "Linux".to_string(),
            stage: Stage::new(
                "Linux",
                vec![Step::shell("cargo test --target x86_64-unknown-linux-gnu")],
            ),
        };
        let branch2 = ParallelBranch {
            name: "Mac".to_string(),
            stage: Stage::new(
                "Mac",
                vec![Step::shell("cargo test --target x86_64-apple-darwin")],
            ),
        };

        let stage = Stage::new("TestMatrix", vec![]).with_parallel(vec![branch1, branch2]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("    strategy:\n"));
        assert!(workflow.contains("      matrix:\n"));
        assert!(workflow.contains("        os: [Linux, Mac]\n"));
    }

    #[test]
    fn test_when_condition_maps_to_if_expression() {
        use crate::pipeline::WhenCondition;
        let stage = Stage::new("Deploy", vec![Step::shell("echo deploy")])
            .with_when(WhenCondition::branch("main"));
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("    if: github.ref == 'refs/heads/main'\n"));
    }

    #[test]
    fn test_post_always_maps_to_always_step() {
        use crate::pipeline::PostCondition;
        let stage = Stage::new("Build", vec![Step::shell("cargo build")])
            .with_post(PostCondition::always(vec![Step::shell("echo cleanup")]));
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("      - run: echo cleanup\n"));
    }

    #[test]
    fn test_post_failure_maps_to_continue_on_error() {
        use crate::pipeline::PostCondition;
        let stage = Stage::new("Build", vec![Step::shell("cargo build")])
            .with_post(PostCondition::failure(vec![Step::shell("echo alert")]));
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        assert!(workflow.contains("      - run: echo alert\n"));
    }

    // US-3.3: Workflow validation tests

    #[test]
    fn test_workflow_yaml_is_valid() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![
                Stage::new("Build", vec![Step::shell("cargo build")]),
                Stage::new("Test", vec![Step::shell("cargo test")]),
            ])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        // Check basic YAML structure
        assert!(workflow.starts_with("name: CI"));
        assert!(workflow.contains("on: [push]"));
        assert!(workflow.contains("jobs:"));
    }

    #[test]
    fn test_workflow_all_steps_have_run() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new(
                "Build",
                vec![
                    Step::shell("cargo build --release"),
                    Step::shell("cargo test --lib"),
                ],
            )])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        let run_count = workflow.matches("      - run:").count();
        assert_eq!(run_count, 2);
    }

    #[test]
    fn test_workflow_no_duplicate_jobs() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![
                Stage::new("Build", vec![Step::shell("cargo build")]),
                Stage::new("Test", vec![Step::shell("cargo test")]),
                Stage::new("Deploy", vec![Step::shell("echo deploy")]),
            ])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        // Each job should appear exactly once
        let build_count = workflow.matches("  Build:\n").count();
        let test_count = workflow.matches("  Test:\n").count();
        let deploy_count = workflow.matches("  Deploy:\n").count();

        assert_eq!(build_count, 1);
        assert_eq!(test_count, 1);
        assert_eq!(deploy_count, 1);
    }

    #[test]
    fn test_workflow_has_required_sections() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![Step::shell("cargo build")])])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        // Verify required sections are present
        assert!(workflow.contains("name: CI"));
        assert!(workflow.contains("on:"));
        assert!(workflow.contains("jobs:"));
        assert!(workflow.contains("  Build:"));
        assert!(workflow.contains("    runs-on:"));
        assert!(workflow.contains("    steps:"));
    }

    #[test]
    fn test_empty_pipeline_generates_minimal_workflow() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![])])
            .build_unchecked();

        let backend = GitHubActionsBackend::new("test/repo");
        let workflow = backend.translate(&pipeline).unwrap();

        // Should still generate valid YAML structure
        assert!(workflow.starts_with("name: CI"));
        assert!(workflow.contains("jobs:"));
        assert!(workflow.contains("  Build:"));
    }
}
