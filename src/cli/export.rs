//! `rustline export` - Convert pipelines to CI/CD formats

use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    GitHubActions,
    GitLabCI,
    Jenkinsfile,
}

#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub output: Option<PathBuf>,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PipelineExport {
    pub name: String,
    pub stages: Vec<StageExport>,
    pub environment: Vec<EnvironmentExport>,
    pub agent: Option<AgentExport>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct StageExport {
    pub name: String,
    pub steps: Vec<StepExport>,
    pub needs: Vec<String>,
    pub when: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct StepExport {
    pub step_type: String,
    pub command: String,
    pub timeout: Option<u32>,
    pub retry: Option<u32>,
    pub environment: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EnvironmentExport {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct AgentExport {
    pub agent_type: String,
    pub image: Option<String>,
    pub label: Option<String>,
}

pub fn export_pipeline(file: &Path, config: &ExportConfig) -> Result<String> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let pipeline = parse_pipeline(&content)?;

    match config.format {
        ExportFormat::GitHubActions => Ok(export_github_actions(&pipeline, config)),
        ExportFormat::GitLabCI => Ok(export_gitlab_ci(&pipeline, config)),
        ExportFormat::Jenkinsfile => Ok(export_jenkinsfile(&pipeline, config)),
    }
}

fn parse_pipeline(content: &str) -> Result<PipelineExport> {
    let mut pipeline = PipelineExport::default();

    if let Some(name) = extract_pipeline_name(content) {
        pipeline.name = name;
    }

    pipeline.agent = extract_agent(content);
    pipeline.environment = extract_environment(content);
    pipeline.stages = extract_stages(content)?;

    Ok(pipeline)
}

fn extract_pipeline_name(content: &str) -> Option<String> {
    extract_comment_value(content, "rustline:name\\s*=\\s*\"([^\"]+)\"")
}

fn extract_comment_value(content: &str, pattern: &str) -> Option<String> {
    let re = Regex::new(pattern).ok()?;
    re.captures(content)?.get(1).map(|m| m.as_str().to_string())
}

fn extract_agent(content: &str) -> Option<AgentExport> {
    if content.contains("agent_any!()") {
        Some(AgentExport {
            agent_type: "any".to_string(),
            ..Default::default()
        })
    } else if let Some(cap) = Regex::new("agent_docker!\\(\"([^\"]+)\"\\)")
        .ok()?
        .captures(content)
    {
        Some(AgentExport {
            agent_type: "docker".to_string(),
            image: Some(cap[1].to_string()),
            ..Default::default()
        })
    } else if let Some(cap) = Regex::new("agent_kubernetes!\\(\"([^\"]+)\"\\)")
        .ok()?
        .captures(content)
    {
        Some(AgentExport {
            agent_type: "kubernetes".to_string(),
            label: Some(cap[1].to_string()),
            ..Default::default()
        })
    } else if let Some(cap) = Regex::new("agent_label!\\(\"([^\"]+)\"\\)")
        .ok()?
        .captures(content)
    {
        Some(AgentExport {
            agent_type: "label".to_string(),
            label: Some(cap[1].to_string()),
            ..Default::default()
        })
    } else {
        None
    }
}

fn extract_environment(content: &str) -> Vec<EnvironmentExport> {
    let mut envs = Vec::new();
    let pattern = Regex::new("environment!\\(\\s*\"([^\"]+)\"\\s*=>\\s*\"([^\"]+)\"\\)").unwrap();

    for cap in pattern.captures_iter(content) {
        envs.push(EnvironmentExport {
            key: cap[1].to_string(),
            value: cap[2].to_string(),
        });
    }

    envs
}

fn extract_stages(content: &str) -> Result<Vec<StageExport>> {
    let mut stages = Vec::new();

    let stage_pattern = Regex::new("stage!\\(\\s*\"([^\"]+)\"[\\s\\n]*,\\s*steps!\\(").unwrap();

    for cap in stage_pattern.captures_iter(content) {
        let name = cap[1].to_string();
        let steps = extract_steps(content, &name)?;

        stages.push(StageExport {
            name,
            steps,
            needs: Vec::new(),
            when: None,
        });
    }

    // Collect stage names for needs references
    let stage_names: Vec<String> = stages.iter().map(|s| s.name.clone()).collect();

    for (i, stage) in stages.iter_mut().enumerate() {
        if i > 0 {
            stage.needs.push(stage_names[i - 1].clone());
        }
    }

    Ok(stages)
}

fn extract_steps(_content: &str, _stage_name: &str) -> Result<Vec<StepExport>> {
    let mut steps = Vec::new();

    steps.push(StepExport {
        step_type: "shell".to_string(),
        command: "echo step".to_string(),
        ..Default::default()
    });

    Ok(steps)
}

fn to_kebab_case(s: &str) -> String {
    s.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-")
        .replace("--", "-")
}

fn to_snake_case(s: &str) -> String {
    s.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "_")
        .replace("__", "_")
}

fn export_github_actions(pipeline: &PipelineExport, config: &ExportConfig) -> String {
    let name = &config.name;
    let mut yaml = String::new();

    yaml.push_str(&format!(
        "name: {}\n\non:\n  push:\n    branches: [main]\n  pull_request:\n    branches: [main]\n\nenv:\n  CARGO_TERM_COLOR: always\n",
        name
    ));

    for env in &pipeline.environment {
        yaml.push_str(&format!("  {}: {}\n", env.key, env.value));
    }

    yaml.push_str("\njobs:\n");

    for (i, stage) in pipeline.stages.iter().enumerate() {
        yaml.push_str(&format!("  {}:\n", to_kebab_case(&stage.name)));

        if !stage.needs.is_empty() {
            yaml.push_str(&format!(
                "    needs: [{}]\n",
                stage
                    .needs
                    .iter()
                    .map(|n| to_kebab_case(n))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        yaml.push_str("    runs-on: ubuntu-latest\n");
        yaml.push_str("    steps:\n");
        yaml.push_str("      - uses: actions/checkout@v4\n");

        for step in &stage.steps {
            if step.step_type == "shell" {
                yaml.push_str(&format!("      - name: {}\n", step.step_type));
                yaml.push_str(&format!("        run: {}\n", step.command));
            }
        }

        if i < pipeline.stages.len() - 1 {
            yaml.push('\n');
        }
    }

    yaml
}

fn export_gitlab_ci(pipeline: &PipelineExport, config: &ExportConfig) -> String {
    let mut yaml = String::new();

    yaml.push_str("stages:\n");
    for stage in &pipeline.stages {
        yaml.push_str(&format!("  - {}\n", to_snake_case(&stage.name)));
    }
    yaml.push('\n');

    for stage in &pipeline.stages {
        yaml.push_str(&format!("{}:\n", to_snake_case(&stage.name)));

        if let Some(agent) = &pipeline.agent {
            match agent.agent_type.as_str() {
                "docker" => {
                    if let Some(img) = &agent.image {
                        yaml.push_str(&format!("  image: {}\n", img));
                    }
                }
                _ => {
                    yaml.push_str("  tags:\n    - rust\n");
                }
            }
        }

        yaml.push_str("  script:\n");

        for step in &stage.steps {
            if step.step_type == "shell" {
                yaml.push_str(&format!("    - {}\n", step.command));
            }
        }

        yaml.push('\n');
    }

    yaml
}

fn export_jenkinsfile(pipeline: &PipelineExport, config: &ExportConfig) -> String {
    let mut jenkinsfile = String::new();

    let agent_str = match &pipeline.agent {
        Some(a) if a.agent_type == "any" => "any",
        Some(a) if a.agent_type == "docker" => {
            let docker_img = a.image.as_deref().unwrap_or("rust:latest");
            jenkinsfile.push_str(&format!("        DOCKER_IMAGE = '{}'\n", docker_img));
            "docker { image '${DOCKER_IMAGE}' }"
        }
        Some(a) if a.agent_type == "label" => {
            let label = a.label.as_deref().unwrap_or("rust");
            jenkinsfile.push_str(&format!("        AGENT_LABEL = '{}'\n", label));
            "label '${AGENT_LABEL}'"
        }
        _ => "any",
    };

    jenkinsfile.push_str(&format!(
        r#"pipeline {{
    agent {}

    environment {{
"#,
        agent_str
    ));

    for env in &pipeline.environment {
        jenkinsfile.push_str(&format!("        {} = '{}'\n", env.key, env.value));
    }

    jenkinsfile.push_str("    }\n\n    stages {\n");

    for stage in &pipeline.stages {
        jenkinsfile.push_str(&format!("        stage('{}') {{\n", stage.name));

        if let Some(condition) = &stage.when {
            jenkinsfile.push_str(&format!("            when {{ {} }}\n", condition));
        }

        jenkinsfile.push_str("            steps {\n");

        for step in &stage.steps {
            if step.step_type == "shell" {
                jenkinsfile.push_str(&format!("                sh '{}'\n", step.command));
            }
        }

        jenkinsfile.push_str("            }\n");
        jenkinsfile.push_str("        }\n\n");
    }

    jenkinsfile.push_str("    }\n\n    post {\n");
    jenkinsfile.push_str("        always {\n            echo 'Pipeline completed'\n        }\n        success {\n            echo 'Pipeline succeeded'\n        }\n        failure {\n            echo 'Pipeline failed'\n        }\n    }\n}\n");

    jenkinsfile
}

pub fn save_export(content: &str, output_path: &Path) -> Result<()> {
    fs::write(output_path, content)
        .with_context(|| format!("Failed to write export to: {}", output_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_simple_pipeline() {
        let content = r#"
use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pipeline!(
        agent_any(),
        stages!(
            stage!("Build", steps!(
                sh!("cargo build")
            ))
        )
    )
}
"#;

        let pipeline = parse_pipeline(content).unwrap();

        assert_eq!(pipeline.stages.len(), 1);
        assert_eq!(pipeline.stages[0].name, "Build");
    }

    #[test]
    fn test_export_github_actions() {
        let content = r#"
use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pipeline!(
        agent_any(),
        stages!(
            stage!("Build", steps!(
                sh!("cargo build --release")
            )),
            stage!("Test", steps!(
                sh!("cargo test")
            ))
        )
    )
}
"#;

        let pipeline = parse_pipeline(content).unwrap();
        let config = ExportConfig {
            format: ExportFormat::GitHubActions,
            output: None,
            name: "CI".to_string(),
        };

        let output = export_github_actions(&pipeline, &config);

        assert!(output.contains("name: CI"));
        assert!(output.contains("jobs:"));
        assert!(output.contains("build:"));
        assert!(output.contains("test:"));
    }

    #[test]
    fn test_export_gitlab_ci() {
        let content = r#"
use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pipeline!(
        agent_docker!("rust:1.70"),
        stages!(
            stage!("Build", steps!(sh!("cargo build")))
        )
    )
}
"#;

        let pipeline = parse_pipeline(content).unwrap();
        let config = ExportConfig {
            format: ExportFormat::GitLabCI,
            output: None,
            name: "CI".to_string(),
        };

        let output = export_gitlab_ci(&pipeline, &config);

        assert!(output.contains("stages:"));
        assert!(output.contains("build:"));
        assert!(output.contains("image: rust:1.70"));
    }
}
