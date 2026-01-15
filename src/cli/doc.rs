//! `rustline doc` - Generate documentation from pipeline comments

use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize)]
pub struct PipelineDoc {
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub stages: Vec<StageDoc>,
    pub parameters: Vec<ParameterDoc>,
    pub environment: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StageDoc {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<StepDoc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepDoc {
    pub step_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParameterDoc {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy)]
pub enum DocFormat {
    Markdown,
    Json,
    Html,
}

pub fn generate_doc(file: &Path, format: DocFormat) -> Result<String> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let doc = parse_pipeline_doc(&content)?;

    match format {
        DocFormat::Markdown => Ok(render_markdown(&doc)),
        DocFormat::Json => Ok(render_json(&doc)),
        DocFormat::Html => Ok(render_html(&doc)),
    }
}

fn parse_pipeline_doc(content: &str) -> Result<PipelineDoc> {
    let mut doc = PipelineDoc::default();

    doc.title = extract_comment_value(content, "rustline:title\\s*=\\s*\"([^\"]+)\"");
    doc.description = extract_comment_value(content, "rustline:description\\s*=\\s*\"([^\"]+)\"");
    doc.author = extract_comment_value(content, "rustline:author\\s*=\\s*\"([^\"]+)\"");

    if let Some(tags_str) =
        extract_comment_value(content, "rustline:tags\\s*=\\s*\"?([^\"\\n]+)\"?")
    {
        doc.tags = tags_str.split(',').map(|s| s.trim().to_string()).collect();
    }

    doc.stages = extract_stages(content)?;
    doc.parameters = extract_parameters(content)?;
    doc.environment = extract_environment(content)?;

    Ok(doc)
}

fn extract_comment_value(content: &str, pattern: &str) -> Option<String> {
    let re = Regex::new(pattern).ok()?;
    re.captures(content)?.get(1).map(|m| m.as_str().to_string())
}

fn extract_stages(content: &str) -> Result<Vec<StageDoc>> {
    let mut stages = Vec::new();

    let stage_pattern =
        Regex::new("stage!\\(\\s*\"([^\"]+)\"[\\s\\n]*,[\\s\\n]*steps!\\(").unwrap();

    for cap in stage_pattern.captures_iter(content) {
        let name = cap[1].to_string();

        let mut stage = StageDoc {
            name,
            description: None,
            steps: Vec::new(),
        };

        let step_pattern = Regex::new("sh!\\(\\s*\"([^\"]+)\"").unwrap();
        let after_stages = &content[cap.get(0).unwrap().end()..];
        let steps_content = after_stages
            .lines()
            .take_while(|l| !l.trim_start().starts_with("))"))
            .collect::<Vec<_>>()
            .join(" ");

        for step_cap in step_pattern.captures_iter(&steps_content) {
            stage.steps.push(StepDoc {
                step_type: "sh".to_string(),
                description: Some(step_cap[1].to_string()),
            });
        }

        stages.push(stage);
    }

    Ok(stages)
}

fn extract_parameters(content: &str) -> Result<Vec<ParameterDoc>> {
    let mut params = Vec::new();

    let param_pattern = Regex::new("//!\\s*rustline:param\\s+(\\w+)\\s*=\\s*\"([^\"]+)\"").unwrap();

    for cap in param_pattern.captures_iter(content) {
        params.push(ParameterDoc {
            name: cap[1].to_string(),
            description: cap[2].to_string(),
        });
    }

    Ok(params)
}

fn extract_environment(content: &str) -> Result<Vec<String>> {
    let mut envs = Vec::new();

    let env_pattern = Regex::new("environment!\\(\\s*\"([^\"]+)\"").unwrap();

    for cap in env_pattern.captures_iter(content) {
        envs.push(cap[1].to_string());
    }

    Ok(envs)
}

fn render_markdown(doc: &PipelineDoc) -> String {
    let mut output = String::new();

    if let Some(title) = &doc.title {
        output.push_str(&format!("# {}\n\n", title));
    } else {
        output.push_str("# Pipeline Documentation\n\n");
    }

    if let Some(desc) = &doc.description {
        output.push_str(&format!("{}\n\n", desc));
    }

    if doc.author.is_some() || !doc.tags.is_empty() {
        output.push_str("## Metadata\n\n");
        if let Some(author) = &doc.author {
            output.push_str(&format!("- **Author**: {}\n", author));
        }
        if !doc.tags.is_empty() {
            output.push_str(&format!("- **Tags**: {}\n", doc.tags.join(", ")));
        }
        output.push('\n');
    }

    if !doc.stages.is_empty() {
        output.push_str("## Stages\n\n");

        for stage in &doc.stages {
            output.push_str(&format!("### {}\n\n", stage.name));

            if !stage.steps.is_empty() {
                output.push_str("#### Steps\n\n");
                for step in &stage.steps {
                    output.push_str(&format!("- `{}`", step.step_type));
                    if let Some(desc) = &step.description {
                        output.push_str(&format!(": {}", desc));
                    }
                    output.push('\n');
                }
                output.push('\n');
            }
        }
    }

    if !doc.environment.is_empty() {
        output.push_str("## Environment Variables\n\n");
        for env in &doc.environment {
            output.push_str(&format!("- `{}`\n", env));
        }
        output.push('\n');
    }

    output
}

fn render_json(doc: &PipelineDoc) -> String {
    serde_json::to_string_pretty(doc).unwrap_or_else(|_| "{}".to_string())
}

fn render_html(doc: &PipelineDoc) -> String {
    let markdown = render_markdown(doc);
    let mut html = String::new();

    for line in markdown.lines() {
        if line.starts_with("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", &line[2..]));
        } else if line.starts_with("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", &line[3..]));
        } else if line.starts_with("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", &line[4..]));
        } else if line.trim().is_empty() {
            html.push_str("<br/>\n");
        } else {
            html.push_str(&format!("<p>{}</p>\n", line));
        }
    }

    html
}

pub fn save_doc(doc: &str, output_path: &Path) -> Result<()> {
    fs::write(output_path, doc).with_context(|| {
        format!(
            "Failed to write documentation to: {}",
            output_path.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_pipeline_doc() {
        let content = r#"
//! rustline:title = "My Pipeline"
//! rustline:description = "A test pipeline"
//! rustline:author = "Test Author"
//! rustline:tags = docker, rust, testing

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

        let doc = parse_pipeline_doc(content).unwrap();

        assert_eq!(doc.title, Some("My Pipeline".to_string()));
        assert_eq!(doc.description, Some("A test pipeline".to_string()));
        assert_eq!(doc.author, Some("Test Author".to_string()));
        assert_eq!(doc.tags, vec!["docker", "rust", "testing"]);
    }

    #[test]
    fn test_generate_markdown() {
        let content = r#"
//! rustline:title = "Test Pipeline"

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

        let doc = parse_pipeline_doc(content).unwrap();
        let markdown = render_markdown(&doc);

        assert!(markdown.contains("# Test Pipeline"));
        assert!(markdown.contains("Build"));
    }
}
