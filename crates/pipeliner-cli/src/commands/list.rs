//! List command - List available pipelines

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;
use tracing::debug;

use crate::config::OutputFormat;

/// Arguments for the `pipeliner list` subcommand.
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Directory to search for pipelines (defaults to current directory)
    #[arg(default_value = ".")]
    pub directory: PathBuf,

    /// Recursively search subdirectories
    #[arg(short, long, default_value = "false")]
    pub recursive: bool,

    /// Pattern to match pipeline files (e.g., "*.json", "*.dsl")
    #[arg(short, long, default_value = "*")]
    pub pattern: String,
}

/// List available pipelines in a directory
pub fn list_pipelines(args: ListArgs, format: OutputFormat) -> Result<()> {
    debug!("Listing pipelines in {:?}", args.directory);

    if !args.directory.exists() {
        anyhow::bail!("Directory does not exist: {:?}", args.directory);
    }

    let pipelines = find_pipelines(&args.directory, args.recursive, &args.pattern)?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&pipelines)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&pipelines)?);
        }
        OutputFormat::Human => {
            if pipelines.is_empty() {
                println!("No pipelines found in {:?}", args.directory);
            } else {
                println!("Available pipelines in {:?}:", args.directory);
                for pipeline in &pipelines {
                    println!("  - {}", pipeline.display());
                }
                println!("\n{} pipeline(s) found", pipelines.len());
            }
        }
    }

    Ok(())
}

/// Find pipeline files in a directory
fn find_pipelines(dir: &PathBuf, recursive: bool, pattern: &str) -> Result<Vec<PathBuf>> {
    let mut pipelines = Vec::new();

    let glob_pattern = if recursive {
        format!("**/{}", pattern)
    } else {
        pattern.to_string()
    };

    for entry in glob::glob(&dir.join(&glob_pattern).to_string_lossy())? {
        let path = entry.context("Failed to read glob entry")?;
        if path.is_file() {
            // Check if it looks like a pipeline file
            if is_pipeline_file(&path) {
                pipelines.push(path);
            }
        }
    }

    // Sort by name
    pipelines.sort_by(|a, b| a.cmp(b));

    Ok(pipelines)
}

/// Check if a file looks like a pipeline file
fn is_pipeline_file(path: &PathBuf) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        matches!(ext_str.as_str(), "json" | "dsl" | "jenkins" | "rs")
            || path.file_name()
                .map(|n| n.to_string_lossy().contains("pipeline"))
                .unwrap_or(false)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_list_pipelines_empty_directory() {
        let dir = TempDir::new().unwrap();
        let args = ListArgs {
            directory: dir.path().to_path_buf(),
            recursive: false,
            pattern: "*.json".to_string(),
        };

        let result = list_pipelines(args, OutputFormat::Human);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_pipelines_with_json_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pipeline1.json"), "{}").unwrap();
        fs::write(dir.path().join("pipeline2.json"), "{}").unwrap();
        fs::write(dir.path().join("other.txt"), "not a pipeline").unwrap();

        let args = ListArgs {
            directory: dir.path().to_path_buf(),
            recursive: false,
            pattern: "*.json".to_string(),
        };

        let result = list_pipelines(args, OutputFormat::Human);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_pipelines_recursive() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pipeline.json"), "{}").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir").join("nested_pipeline.json"), "{}").unwrap();

        let args = ListArgs {
            directory: dir.path().to_path_buf(),
            recursive: true,
            pattern: "*.json".to_string(),
        };

        let result = list_pipelines(args, OutputFormat::Human);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_pipelines_nonexistent_directory() {
        let args = ListArgs {
            directory: PathBuf::from("/nonexistent/path"),
            recursive: false,
            pattern: "*.json".to_string(),
        };

        let result = list_pipelines(args, OutputFormat::Human);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_pipeline_file() {
        assert!(is_pipeline_file(&PathBuf::from("pipeline.json")));
        assert!(is_pipeline_file(&PathBuf::from("my-pipeline.dsl")));
        assert!(is_pipeline_file(&PathBuf::from("build.jenkins")));
        assert!(is_pipeline_file(&PathBuf::from("pipeline.rs")));
        assert!(is_pipeline_file(&PathBuf::from("src/pipeline_definition.json")));
        assert!(is_pipeline_file(&PathBuf::from("script.rs"))); // .rs files are pipeline scripts
        assert!(is_pipeline_file(&PathBuf::from("main.rs"))); // .rs files could be pipelines
        assert!(!is_pipeline_file(&PathBuf::from("readme.md")));
    }
}
