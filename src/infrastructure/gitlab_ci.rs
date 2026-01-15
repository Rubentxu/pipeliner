//! GitLab CI backend
//!
//! Translates Rustline pipelines to GitLab CI configurations.

use crate::pipeline::PipelineError;
use crate::pipeline::{Pipeline, Stage, Step};

/// Backend for generating GitLab CI configuration
pub struct GitLabCIBackend;

impl GitLabCIBackend {
    /// Creates a new GitLab CI backend
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Translates a pipeline to GitLab CI YAML
    #[allow(clippy::missing_errors_doc, clippy::format_push_string)]
    pub fn translate(&self, pipeline: &Pipeline) -> Result<String, PipelineError> {
        let mut yaml = String::new();

        // Add variables at the top if present
        if !pipeline.environment.vars.is_empty() {
            yaml.push_str("variables:\n");
            for (key, value) in &pipeline.environment.vars {
                yaml.push_str(&format!("  {key}: \"{value}\"\n"));
            }
            yaml.push('\n');
        }

        // Define stages
        yaml.push_str("stages:\n");
        let mut stage_names = std::collections::HashSet::new();
        for stage in &pipeline.stages {
            let name = sanitize_stage_name(&stage.name);
            if stage_names.insert(name.clone()) {
                yaml.push_str(&format!("  - {name}\n"));
            }
            // Add parallel branch stage names
            for branch in &stage.parallel {
                let branch_name = sanitize_stage_name(&format!("{}_{}", stage.name, branch.name));
                stage_names.insert(branch_name);
            }
        }
        yaml.push('\n');

        // Define jobs
        for stage in &pipeline.stages {
            yaml.push_str(&self.translate_stage(stage, &pipeline.environment.vars));
        }

        Ok(yaml)
    }

    /// Translates a stage to a GitLab CI job
    #[allow(clippy::format_push_string)]
    fn translate_stage(
        &self,
        stage: &Stage,
        _global_vars: &std::collections::HashMap<String, String>,
    ) -> String {
        // Handle parallel branches
        if !stage.parallel.is_empty() {
            let mut yaml = String::new();
            for branch in &stage.parallel {
                let job_name = sanitize_job_name(&format!("{}_{}", stage.name, branch.name));
                yaml.push_str(&format!("{job_name}:\n"));
                yaml.push_str(&format!("  stage: {}\n", sanitize_stage_name(&stage.name)));

                // Handle agent/image
                if let Some(ref agent) = stage.agent {
                    match agent {
                        crate::pipeline::AgentType::Docker(config) => {
                            yaml.push_str(&format!("  image: {}\n", config.image));
                        }
                        _ => {}
                    }
                }

                // Handle when condition (rules)
                if let Some(ref when) = stage.when {
                    yaml.push_str("  rules:\n");
                    yaml.push_str(&format!("  - {}\n", self.translate_when_condition(when)));
                }

                yaml.push_str("  script:\n");
                for step in &branch.stage.steps {
                    yaml.push_str(&self.translate_step(step));
                }
            }
            return yaml;
        }

        let mut job = String::new();

        job.push_str(&format!("{}:\n", sanitize_job_name(&stage.name)));
        job.push_str(&format!("  stage: {}\n", sanitize_stage_name(&stage.name)));

        // Handle agent/image
        if let Some(ref agent) = stage.agent {
            match agent {
                crate::pipeline::AgentType::Docker(config) => {
                    job.push_str(&format!("  image: {}\n", config.image));
                }
                crate::pipeline::AgentType::Label(label) => {
                    job.push_str(&format!("  tags: [{label}]\n"));
                }
                _ => {}
            }
        }

        // Handle timeout
        for step in &stage.steps {
            if let crate::pipeline::StepType::Timeout { duration, .. } = &step.step_type {
                let minutes = (*duration).as_secs() / 60;
                job.push_str(&format!("  timeout: {}m\n", minutes.max(1)));
                break;
            }
        }

        // Handle when condition (rules)
        if let Some(ref when) = stage.when {
            job.push_str("  rules:\n");
            job.push_str(&format!("  - {}\n", self.translate_when_condition(when)));
        }

        // Handle post-conditions
        for post in &stage.post {
            match post {
                crate::pipeline::PostCondition::Always { steps } => {
                    job.push_str("  after_script:\n");
                    for step in steps {
                        job.push_str(&self.translate_step(step));
                    }
                }
                _ => {}
            }
        }

        job.push_str("  script:\n");

        for step in &stage.steps {
            job.push_str(&self.translate_step(step));
        }

        job
    }

    /// Translates when condition to GitLab CI rules expression
    #[allow(clippy::unused_self)]
    fn translate_when_condition(&self, condition: &crate::pipeline::WhenCondition) -> String {
        match condition {
            crate::pipeline::WhenCondition::Branch { branch } => {
                format!("if: $CI_COMMIT_BRANCH == \"{branch}\"")
            }
            crate::pipeline::WhenCondition::Tag { tag } => {
                format!("if: $CI_COMMIT_TAG == \"{tag}\"")
            }
            crate::pipeline::WhenCondition::Environment { name, value } => {
                format!("if: ${name} == \"{value}\"")
            }
            crate::pipeline::WhenCondition::Expression { expression } => {
                format!("if: {expression}")
            }
            crate::pipeline::WhenCondition::AllOf { conditions } => {
                let conditions_str = conditions
                    .iter()
                    .map(|c| self.translate_when_condition(c))
                    .collect::<Vec<_>>()
                    .join(" && ");
                format!("if: {conditions_str}")
            }
            crate::pipeline::WhenCondition::AnyOf { conditions } => {
                let conditions_str = conditions
                    .iter()
                    .map(|c| self.translate_when_condition(c))
                    .collect::<Vec<_>>()
                    .join(" || ");
                format!("if: {conditions_str}")
            }
        }
    }

    /// Translates a step to a GitLab CI script
    fn translate_step(&self, step: &Step) -> String {
        match &step.step_type {
            crate::pipeline::StepType::Shell { command } => {
                format!("    - {command}\n")
            }
            crate::pipeline::StepType::Echo { message } => {
                format!("    - echo '{message}'\n")
            }
            _ => "    - # Step type not supported\n".to_string(),
        }
    }
}

impl Default for GitLabCIBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitizes stage name for GitLab CI
fn sanitize_stage_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Sanitizes job name for GitLab CI
fn sanitize_job_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{AgentType, Pipeline, Stage, Step};

    #[test]
    fn test_gitlab_ci_backend_creation() {
        let _backend = GitLabCIBackend::new();
    }

    #[test]
    fn test_sanitize_stage_name() {
        assert_eq!(sanitize_stage_name("Build Stage"), "Build-Stage");
        assert_eq!(sanitize_stage_name("Test@123"), "Test-123");
    }

    #[test]
    fn test_sanitize_job_name() {
        assert_eq!(sanitize_job_name("Build Stage"), "Build_Stage");
        assert_eq!(sanitize_job_name("Test@123"), "Test_123");
    }

    #[test]
    fn test_simple_pipeline_to_gitlab_ci() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![
                Stage::new("Build", vec![Step::shell("cargo build")]),
                Stage::new("Test", vec![Step::shell("cargo test")]),
            ])
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.contains("stages:"));
        assert!(gitlab_ci.contains("  - Build\n"));
        assert!(gitlab_ci.contains("  - Test\n"));
        assert!(gitlab_ci.contains("Build:\n"));
        assert!(gitlab_ci.contains("Test:\n"));
        assert!(gitlab_ci.contains("  stage: Build\n"));
        assert!(gitlab_ci.contains("  script:\n"));
        assert!(gitlab_ci.contains("    - cargo build\n"));
        assert!(gitlab_ci.contains("    - cargo test\n"));
    }

    #[test]
    fn test_stages_map_to_gitlab_stages() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![
                Stage::new("Build", vec![Step::shell("echo build")]),
                Stage::new("Test", vec![Step::shell("echo test")]),
                Stage::new("Deploy", vec![Step::shell("echo deploy")]),
            ])
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.contains("stages:"));
        assert!(gitlab_ci.contains("  - Build\n"));
        assert!(gitlab_ci.contains("  - Test\n"));
        assert!(gitlab_ci.contains("  - Deploy\n"));
    }

    #[test]
    fn test_step_maps_to_script_line() {
        let stage = Stage::new("build", vec![Step::shell("cargo build --release")]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.contains("    - cargo build --release\n"));
    }

    #[test]
    fn test_echo_step_maps_to_echo_script() {
        let stage = Stage::new("notify", vec![Step::echo("Build completed")]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.contains("    - echo 'Build completed'\n"));
    }

    #[test]
    fn test_environment_maps_to_variables() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new(
                "build",
                vec![Step::shell("echo $RUST_VERSION")],
            )])
            .environment(|e| e.set("RUST_VERSION", "1.70").set("BUILD_NUMBER", "42"))
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.contains("variables:\n"));
        assert!(gitlab_ci.contains("  RUST_VERSION: \"1.70\"\n"));
        assert!(gitlab_ci.contains("  BUILD_NUMBER: \"42\"\n"));
    }

    #[test]
    fn test_generated_yaml_is_valid() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![Stage::new("Build", vec![Step::shell("cargo build")])])
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.starts_with("stages:"));
        assert!(gitlab_ci.contains("Build:\n"));
        assert!(gitlab_ci.contains("  stage: Build\n"));
        assert!(gitlab_ci.contains("  script:\n"));
    }

    #[test]
    fn test_sanitized_names_for_gitlab() {
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![
                Stage::new("Build Stage", vec![Step::shell("echo build")]),
                Stage::new("Test@Production", vec![Step::shell("echo test")]),
            ])
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        // Job names use underscores
        assert!(gitlab_ci.contains("Build_Stage:\n"));
        assert!(gitlab_ci.contains("Test_Production:\n"));
        // Stage names use hyphens
        assert!(gitlab_ci.contains("  - Build-Stage\n"));
        assert!(gitlab_ci.contains("  - Test-Production\n"));
    }

    #[test]
    fn test_parallel_to_parallel_jobs() {
        use crate::pipeline::ParallelBranch;
        let branch1 = ParallelBranch {
            name: "linux".to_string(),
            stage: Stage::new(
                "linux",
                vec![Step::shell("cargo test --target x86_64-unknown-linux-gnu")],
            ),
        };
        let branch2 = ParallelBranch {
            name: "macos".to_string(),
            stage: Stage::new(
                "macos",
                vec![Step::shell("cargo test --target x86_64-apple-darwin")],
            ),
        };

        let stage = Stage::new("TestMatrix", vec![]).with_parallel(vec![branch1, branch2]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.contains("TestMatrix_linux:\n"));
        assert!(gitlab_ci.contains("TestMatrix_macos:\n"));
    }

    #[test]
    fn test_when_condition_to_rules() {
        use crate::pipeline::WhenCondition;
        let stage = Stage::new("deploy", vec![Step::shell("echo deploy")])
            .with_when(WhenCondition::branch("main"));
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.contains("  rules:\n"));
        assert!(gitlab_ci.contains("  - if: $CI_COMMIT_BRANCH == \"main\"\n"));
    }

    #[test]
    fn test_timeout_to_timeout_in_seconds() {
        use std::time::Duration;
        let step = Step::timeout(Duration::from_secs(300), Step::shell("cargo build"));
        let stage = Stage::new("build", vec![step]);
        let pipeline = Pipeline::builder()
            .agent(AgentType::Any)
            .stages(vec![stage])
            .build_unchecked();

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.contains("  timeout: 5m\n"));
    }

    #[test]
    fn test_docker_image_to_image_section() {
        let stage = Stage::new("build", vec![Step::shell("cargo build")]).with_agent(
            AgentType::Docker(crate::pipeline::DockerConfig {
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

        let backend = GitLabCIBackend::new();
        let gitlab_ci = backend.translate(&pipeline).unwrap();

        assert!(gitlab_ci.contains("  image: rust:1.70\n"));
    }
}
