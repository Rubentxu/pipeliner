//! `rustline lint` - Analyze pipelines for best practices

use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct LintMessage {
    pub code: String,
    pub message: String,
    pub line: usize,
    pub severity: LintSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "severity")]
pub enum LintSeverity {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LintSeverity::Info => write!(f, "info"),
            LintSeverity::Warning => write!(f, "warning"),
            LintSeverity::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug)]
pub struct LintConfig {
    pub min_severity: LintSeverity,
    pub show_suggestions: bool,
    pub format: OutputFormat,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            min_severity: LintSeverity::Info,
            show_suggestions: false,
            format: OutputFormat::Text,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Text,
    Json,
}

pub fn lint_pipeline(file: &Path, config: &LintConfig) -> Result<Vec<LintMessage>> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let mut messages = Vec::new();

    messages.extend(check_missing_agent(&content)?);
    messages.extend(check_timeouts(&content)?);
    messages.extend(check_retry(&content)?);
    messages.extend(check_empty_stages(&content)?);
    messages.extend(check_hardcoded_secrets(&content)?);
    messages.extend(check_post_conditions(&content)?);

    messages.retain(|msg| {
        let severity_order = match msg.severity {
            LintSeverity::Info => 0,
            LintSeverity::Warning => 1,
            LintSeverity::Error => 2,
        };
        let min_order = match config.min_severity {
            LintSeverity::Info => 0,
            LintSeverity::Warning => 1,
            LintSeverity::Error => 2,
        };
        severity_order >= min_order
    });

    Ok(messages)
}

fn check_missing_agent(content: &str) -> Result<Vec<LintMessage>> {
    let mut messages = Vec::new();

    if !content.contains("pipeline!") {
        return Ok(messages);
    }

    let has_agent = content.contains("agent_any!")
        || content.contains("agent_none!")
        || content.contains("agent_docker!")
        || content.contains("agent_kubernetes!")
        || content.contains("agent_label!");

    if !has_agent {
        let stage_pattern = Regex::new("stage!\\(\"([^\"]+)\"").unwrap();
        let stages: Vec<_> = stage_pattern.captures_iter(content).collect();

        if !stages.is_empty() {
            messages.push(LintMessage {
                code: "P001".to_string(),
                message: "No agent specification found in pipeline".to_string(),
                line: 1,
                severity: LintSeverity::Warning,
                suggestion: Some(
                    "Add agent_any!() or agent_docker!() to specify where stages run".to_string(),
                ),
            });
        }
    }

    Ok(messages)
}

fn check_timeouts(content: &str) -> Result<Vec<LintMessage>> {
    let mut messages = Vec::new();

    let stage_pattern = Regex::new("stage!\\(\"([^\"]+)\"[\\s\\n]*,\\s*steps!\\(").unwrap();

    for cap in stage_pattern.captures_iter(content) {
        let stage_name = &cap[1];

        let start_idx = cap.get(0).unwrap().start();
        let stage_block = &content[start_idx..start_idx + 500.min(content.len() - start_idx)];

        let has_timeout = stage_block.contains("timeout!");
        let has_sh = stage_block.contains("sh!");

        if !has_timeout && has_sh {
            messages.push(LintMessage {
                code: "P002".to_string(),
                message: format!("Stage '{}' may need a timeout", stage_name),
                line: content[..start_idx].lines().count(),
                severity: LintSeverity::Info,
                suggestion: Some(
                    "Add timeout!(<minutes>, sh!(...)) to prevent hanging stages".to_string(),
                ),
            });
        }
    }

    Ok(messages)
}

fn check_retry(content: &str) -> Result<Vec<LintMessage>> {
    let mut messages = Vec::new();

    let unreliable_patterns = [
        "git clone",
        "git fetch",
        "docker pull",
        "curl ",
        "wget ",
        "scp ",
        "ssh ",
    ];

    for (line_num, line) in content.lines().enumerate() {
        if (line.contains("sh!(\"") || line.contains("sh!('")) && !line.contains("retry!") {
            for pattern in &unreliable_patterns {
                if line.contains(pattern) {
                    messages.push(LintMessage {
                        code: "P003".to_string(),
                        message: format!("Unreliable command '{}' without retry", pattern),
                        line: line_num + 1,
                        severity: LintSeverity::Info,
                        suggestion: Some("Consider wrapping with retry!(count, step)".to_string()),
                    });
                    break;
                }
            }
        }
    }

    Ok(messages)
}

fn check_empty_stages(content: &str) -> Result<Vec<LintMessage>> {
    let mut messages = Vec::new();

    let stage_pattern = Regex::new("stage!\\(\"([^\"]+)\"\\s*,\\s*steps!\\(\\)").unwrap();

    for cap in stage_pattern.captures_iter(content) {
        messages.push(LintMessage {
            code: "P004".to_string(),
            message: format!("Stage '{}' has no steps", &cap[1]),
            line: 1,
            severity: LintSeverity::Error,
            suggestion: Some("Add at least one step to the stage".to_string()),
        });
    }

    Ok(messages)
}

fn check_hardcoded_secrets(content: &str) -> Result<Vec<LintMessage>> {
    let mut messages = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if line.contains("password") && line.contains("\"") && line.len() > 20 {
            messages.push(LintMessage {
                code: "P005".to_string(),
                message: "Possible hardcoded password".to_string(),
                line: line_num + 1,
                severity: LintSeverity::Error,
                suggestion: Some("Use environment variables or secrets management".to_string()),
            });
        }
    }

    Ok(messages)
}

fn check_post_conditions(content: &str) -> Result<Vec<LintMessage>> {
    let mut messages = Vec::new();

    if content.contains("pipeline!") && !content.contains("post!") {
        messages.push(LintMessage {
            code: "P007".to_string(),
            message: "No post-conditions defined".to_string(),
            line: 1,
            severity: LintSeverity::Info,
            suggestion: Some(
                "Add post!(always(...), success(...), failure(...)) for cleanup and notifications"
                    .to_string(),
            ),
        });
    }

    Ok(messages)
}

pub fn format_lint_messages(messages: &[LintMessage], format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => {
            if messages.is_empty() {
                "No lint issues found.".to_string()
            } else {
                let mut output = String::new();
                for msg in messages {
                    output.push_str(&format!(
                        "{}: {} (line {}) [{}]\n  {}\n",
                        msg.code,
                        msg.message,
                        msg.line,
                        msg.severity,
                        msg.suggestion.clone().unwrap_or_default()
                    ));
                }
                output
            }
        }
        OutputFormat::Json => {
            serde_json::to_string_pretty(messages).unwrap_or_else(|_| "[]".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_lint_empty_stage() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.rs");

        let empty_pipeline = r#"
use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pipeline!(
        agent_any(),
        stages!(
            stage!("Empty", steps!())
        )
    )
}
"#;

        fs::write(&file_path, empty_pipeline).unwrap();
        let config = LintConfig::default();
        let messages = lint_pipeline(&file_path, &config).unwrap();

        let has_p004 = messages.iter().any(|m| m.code == "P004");
        assert!(has_p004, "Should detect empty stage");
    }
}
