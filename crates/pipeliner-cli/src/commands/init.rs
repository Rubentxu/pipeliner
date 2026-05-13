//! Init command - scaffold a new pipeline

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Pipeline name
    #[arg(long)]
    pub name: Option<String>,

    /// Output file path
    #[arg(short, long, default_value = "pipeline.json")]
    pub output: PathBuf,
}

pub fn init_pipeline(args: InitArgs) -> Result<()> {
    let name = args.name.unwrap_or_else(|| "my-pipeline".to_string());

    if args.output.exists() {
        anyhow::bail!(
            "File {:?} already exists. Remove it first or use --output for a different path.",
            args.output
        );
    }

    let template = format!(
        r#"{{
  "name": "{}",
  "stages": [
    {{
      "name": "build",
      "steps": [
        {{
          "step": {{
            "name": "compile",
            "shell": {{
              "command": "echo 'Building...'"
            }}
          }}
        }}
      ]
    }},
    {{
      "name": "test",
      "steps": [
        {{
          "step": {{
            "name": "run-tests",
            "shell": {{
              "command": "echo 'Running tests...'"
            }}
          }}
        }}
      ]
    }},
    {{
      "name": "deploy",
      "steps": [
        {{
          "step": {{
            "name": "deploy",
            "shell": {{
              "command": "echo 'Deploying...'"
            }}
          }}
        }}
      ]
    }}
  ]
}}"#,
        name
    );

    std::fs::write(&args.output, &template)
        .with_context(|| format!("Failed to write {:?}", args.output))?;

    println!("Created pipeline scaffold: {:?}", args.output);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_file() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("pipeline.json");

        let args = InitArgs {
            name: Some("test-pipeline".to_string()),
            output: output.clone(),
        };

        init_pipeline(args).unwrap();

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("test-pipeline"));
        assert!(content.contains("build"));
        assert!(content.contains("test"));
        assert!(content.contains("deploy"));
    }

    #[test]
    fn test_init_rejects_existing_file() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("pipeline.json");
        fs::write(&output, "existing").unwrap();

        let args = InitArgs {
            name: None,
            output: output.clone(),
        };

        assert!(init_pipeline(args).is_err());
    }
}
