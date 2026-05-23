//! Describe command - Show pipeline information

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;
use tracing::debug;

use crate::config::OutputFormat;
use pipeliner_core::pipeline::{Pipeline, StageOrParallel};

/// Arguments for the `pipeliner describe` subcommand.
#[derive(Args, Debug)]
pub struct DescribeArgs {
    /// Pipeline script file to describe
    #[arg(value_name = "SCRIPT")]
    pub script: PathBuf,

    /// Show detailed output including step definitions
    #[arg(short, long, default_value = "false")]
    pub detailed: bool,
}

/// Pipeline description for output
#[derive(serde::Serialize, serde::Deserialize)]
struct PipelineDescription {
    name: String,
    description: Option<String>,
    stages: Vec<StageDescription>,
    #[serde(skip_serializing_if = "Option::is_none")]
    steps_count: Option<usize>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StageDescription {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    steps_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel: Option<bool>,
}

/// Show pipeline information
pub fn describe_pipeline_cmd(args: DescribeArgs, format: OutputFormat) -> Result<()> {
    debug!("Describing pipeline from {:?}", args.script);

    if !args.script.exists() {
        anyhow::bail!("Pipeline file not found: {}", args.script.display());
    }

    // Read and parse the pipeline
    let content = std::fs::read_to_string(&args.script)
        .with_context(|| format!("Failed to read pipeline file: {}", args.script.display()))?;

    // Detect file type and parse accordingly
    let pipeline = if args.script.extension().and_then(|e| e.to_str()) == Some("rs") {
        // .rs file - try to extract pipeline! macro or run --describe
        describe_from_rust_script(&content)?
    } else if args.script.extension().and_then(|e| e.to_str()) == Some("dsl") {
        // .dsl file - use runtime parser
        pipeliner_core::dsl::parse_pipeline(&content)
            .map_err(|e| anyhow::anyhow!("DSL parse error: {}", e))?
    } else {
        // JSON file
        serde_json::from_str(&content)
            .context("Failed to parse pipeline JSON")?
    };

    // Generate description based on format
    match format {
        OutputFormat::Json => {
            let description = create_description(&pipeline);
            println!("{}", serde_json::to_string_pretty(&description)?);
        }
        OutputFormat::Yaml => {
            let description = create_description(&pipeline);
            println!("{}", serde_yaml::to_string(&description)?);
        }
        OutputFormat::Human => {
            print_pipeline_human(&pipeline, args.detailed);
        }
    }

    Ok(())
}

/// Create a pipeline description struct
fn create_description(pipeline: &Pipeline) -> PipelineDescription {
    let stages: Vec<StageDescription> = pipeline.stages
        .iter()
        .map(|stage_or_parallel| {
            StageDescription {
                name: stage_or_parallel.name().unwrap_or("Unnamed").to_string(),
                steps_count: Some(stage_or_parallel.step_count()),
                parallel: Some(stage_or_parallel.is_parallel()),
            }
        })
        .collect();

    let steps_count = pipeline.stages.iter().map(|s| s.step_count()).sum::<usize>();

    PipelineDescription {
        name: pipeline.name.clone().unwrap_or_else(|| "Unnamed".to_string()),
        description: pipeline.description.clone(),
        stages,
        steps_count: if steps_count > 0 { Some(steps_count) } else { None },
    }
}

/// Describe a pipeline from a Rust script file
fn describe_from_rust_script(content: &str) -> Result<Pipeline> {
    // First try to parse as regular JSON/DSL if it looks like that
    let trimmed = content.trim();
    if trimmed.starts_with('{') {
        // Looks like JSON
        return serde_json::from_str(content)
            .context("Failed to parse pipeline JSON");
    }

    if trimmed.starts_with("pipeline") || trimmed.contains("stage(") {
        // Looks like DSL
        return pipeliner_core::dsl::parse_pipeline(content)
            .map_err(|e| anyhow::anyhow!("DSL parse error: {}", e));
    }

    // For .rs files that contain pipeline! macro, we need to compile and run with --describe
    // This is a simplified version - full implementation would use the script runner
    anyhow::bail!(
        "Cannot describe .rs file directly. \
         Please ensure the file contains a pipeline! macro or provide a JSON/DSL file."
    );
}

/// Print pipeline description in human-readable format
fn print_pipeline_human(pipeline: &Pipeline, detailed: bool) {
    println!("Pipeline: {}", pipeline.name.as_deref().unwrap_or("Unnamed"));

    if let Some(ref description) = pipeline.description {
        println!("Description: {}", description);
    }

    println!("\nStages ({}):", pipeline.stages.len());
    for (idx, stage_or_parallel) in pipeline.stages.iter().enumerate() {
        let stage_name = stage_or_parallel.name().map(String::from).unwrap_or_else(|| format!("Stage {}", idx + 1));
        let steps_count = stage_or_parallel.step_count();
        let parallel_info = if stage_or_parallel.is_parallel() { " [parallel]" } else { "" };
        println!("  {}. {}{} ({} steps)", idx + 1, stage_name, parallel_info, steps_count);

        if detailed {
            if let Some(stage) = stage_or_parallel.as_stage() {
                for step in &stage.steps {
                    println!("     - {:?}", step.step_type);
                }
            } else if let Some(parallel) = stage_or_parallel.as_parallel() {
                for (p_idx, p_stage) in parallel.stages.iter().enumerate() {
                    println!("       {}.{} {} ({} steps)", idx + 1, p_idx + 1, p_stage.name, p_stage.steps.len());
                    if detailed {
                        for step in &p_stage.steps {
                            println!("          - {:?}", step.step_type);
                        }
                    }
                }
            }
        }
    }

    if detailed {
        println!("\nFull Pipeline Structure:");
        println!("{:#?}", pipeline);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_describe_nonexistent_file() {
        let args = DescribeArgs {
            script: PathBuf::from("/nonexistent/pipeline.json"),
            detailed: false,
        };

        let result = describe_pipeline_cmd(args, OutputFormat::Human);
        assert!(result.is_err());
    }

    #[test]
    fn test_describe_json_file() {
        let dir = TempDir::new().unwrap();
        let pipeline_path = dir.path().join("pipeline.json");
        let json = r#"{
            "name": "test-pipeline",
            "stages": [{"type": "stage", "name": "build", "steps": [{"type": "shell", "command": "echo hi"}]}]
        }"#;
        fs::write(&pipeline_path, json).unwrap();

        let args = DescribeArgs {
            script: pipeline_path,
            detailed: false,
        };

        let result = describe_pipeline_cmd(args, OutputFormat::Human);
        assert!(result.is_ok());
    }

    #[test]
    fn test_describe_dsl_file() {
        let dir = TempDir::new().unwrap();
        let dsl_path = dir.path().join("pipeline.dsl");
        let dsl = r#"
pipeline {
    name = "test-dsl"
    stage("build") {
        sh "echo building"
    }
}
"#;
        fs::write(&dsl_path, dsl).unwrap();

        let args = DescribeArgs {
            script: dsl_path,
            detailed: false,
        };

        let result = describe_pipeline_cmd(args, OutputFormat::Human);
        assert!(result.is_ok());
    }

    #[test]
    fn test_describe_with_detailed_flag() {
        let dir = TempDir::new().unwrap();
        let pipeline_path = dir.path().join("pipeline.json");
        let json = r#"{
            "name": "detailed-pipeline",
            "stages": [{"type": "stage", "name": "build", "steps": [{"type": "shell", "command": "echo hi"}]}]
        }"#;
        fs::write(&pipeline_path, json).unwrap();

        let args = DescribeArgs {
            script: pipeline_path,
            detailed: true,
        };

        let result = describe_pipeline_cmd(args, OutputFormat::Human);
        assert!(result.is_ok());
    }

    #[test]
    fn test_describe_json_format() {
        let dir = TempDir::new().unwrap();
        let pipeline_path = dir.path().join("pipeline.json");
        let json = r#"{
            "name": "json-format-test",
            "stages": []
        }"#;
        fs::write(&pipeline_path, json).unwrap();

        let args = DescribeArgs {
            script: pipeline_path,
            detailed: false,
        };

        let result = describe_pipeline_cmd(args, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_description() {
        let pipeline = Pipeline {
            name: Some("test".to_string()),
            description: Some("A test pipeline".to_string()),
            stages: vec![],
            ..Default::default()
        };

        let desc = create_description(&pipeline);
        assert_eq!(desc.name, "test");
        assert_eq!(desc.description, Some("A test pipeline".to_string()));
    }
}
