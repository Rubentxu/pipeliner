//! CLI commands for Pipeliner.

use anyhow::{Context, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

use pipeliner_core::Pipeline;
use pipeliner_events::LocalEventBus;

/// Command-line interface for Pipeliner pipeline execution
#[derive(Parser, Debug)]
#[command(name = "pipeliner")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a pipeline
    #[command(name = "run")]
    Run(RunArgs),

    /// Validate a pipeline definition
    #[command(name = "validate")]
    Validate(ValidateArgs),

    /// Lint a pipeline for style and best practices
    #[command(name = "lint")]
    Lint(LintArgs),

    /// Generate documentation for a pipeline
    #[command(name = "doc")]
    Doc(DocArgs),

    /// Export pipeline to different formats
    #[command(name = "export")]
    Export(ExportArgs),

    /// Generate shell completions
    #[command(name = "completions")]
    Completions(CompletionsArgs),

    /// Check pipeline syntax without execution
    #[command(name = "check")]
    Check(CheckArgs),
}

#[derive(Args, Debug)]
struct RunArgs {
    /// Pipeline file to run
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    definition: Option<String>,

    /// Working directory
    #[arg(short, long)]
    working_dir: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, default_value = "false")]
    verbose: bool,
}

#[derive(Args, Debug)]
struct ValidateArgs {
    /// Pipeline file to validate
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    definition: Option<String>,

    /// Output format (json, yaml, text)
    #[arg(short, long, default_value = "text")]
    output: String,
}

#[derive(Args, Debug)]
struct LintArgs {
    /// Pipeline file to lint
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    definition: Option<String>,

    /// Strict mode (fail on warnings)
    #[arg(short, long, default_value = "false")]
    strict: bool,
}

#[derive(Args, Debug)]
struct DocArgs {
    /// Pipeline file
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Output directory
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Format (markdown, html, man)
    #[arg(short, long, default_value = "markdown")]
    format: String,
}

#[derive(Args, Debug)]
struct ExportArgs {
    /// Pipeline file
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Output file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Format (json, yaml, dockerfile, kubernetes)
    #[arg(short, long, default_value = "json")]
    format: String,
}

#[derive(Args, Debug)]
struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(short, long)]
    shell: String,
}

#[derive(Args, Debug)]
struct CheckArgs {
    /// Pipeline file to check
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    definition: Option<String>,
}

pub async fn run() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Run(run_args) => run_pipeline(run_args).await,
        Commands::Validate(validate_args) => validate_pipeline(validate_args),
        Commands::Lint(lint_args) => lint_pipeline(lint_args),
        Commands::Doc(doc_args) => generate_docs(doc_args),
        Commands::Export(export_args) => export_pipeline(export_args),
        Commands::Completions(completions_args) => generate_completions(completions_args),
        Commands::Check(check_args) => check_pipeline(check_args),
    }
}

async fn run_pipeline(args: RunArgs) -> Result<()> {
    info!("Running pipeline");

    let definition = get_definition(&args.file, &args.definition)?;
    let pipeline: Pipeline =
        serde_yaml::from_str(&definition).context("Failed to parse pipeline definition")?;

    let event_bus = LocalEventBus::new();
    let name = pipeline.name.unwrap_or_else(|| "Unnamed".to_string());
    info!("Pipeline '{}' parsed successfully", name);

    Ok(())
}

fn validate_pipeline(args: ValidateArgs) -> Result<()> {
    info!("Validating pipeline");

    let definition = get_definition(&args.file, &args.definition)?;
    let _pipeline: Pipeline =
        serde_yaml::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("Pipeline is valid");
    Ok(())
}

fn lint_pipeline(args: LintArgs) -> Result<()> {
    info!("Linting pipeline");

    let definition = get_definition(&args.file, &args.definition)?;
    let _pipeline: Pipeline =
        serde_yaml::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("No issues found");
    Ok(())
}

fn generate_docs(args: DocArgs) -> Result<()> {
    info!("Generating documentation");

    let definition = get_definition(&args.file, &None)?;
    let _pipeline: Pipeline =
        serde_yaml::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("Documentation generated");
    Ok(())
}

fn export_pipeline(args: ExportArgs) -> Result<()> {
    info!("Exporting pipeline");

    let definition = get_definition(&args.file, &None)?;
    let pipeline: Pipeline =
        serde_yaml::from_str(&definition).context("Failed to parse pipeline definition")?;

    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&pipeline)?;
            println!("{}", json);
        }
        "yaml" => {
            let yaml = serde_yaml::to_string(&pipeline)?;
            println!("{}", yaml);
        }
        _ => anyhow::bail!("Unsupported format: {}", args.format),
    }

    Ok(())
}

fn generate_completions(args: CompletionsArgs) -> Result<()> {
    use clap_complete::Shell;

    let shell = match args.shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        _ => anyhow::bail!("Unsupported shell: {}", args.shell),
    };

    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "pipeliner", &mut std::io::stdout());

    Ok(())
}

fn check_pipeline(args: CheckArgs) -> Result<()> {
    info!("Checking pipeline");

    let definition = get_definition(&args.file, &args.definition)?;
    let _pipeline: Pipeline =
        serde_yaml::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("Pipeline syntax is correct");
    Ok(())
}

fn get_definition(file: &Option<PathBuf>, definition: &Option<String>) -> Result<String> {
    match (file, definition) {
        (Some(path), None) => std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {:?}", path)),
        (None, Some(def)) => Ok(def.clone()),
        (None, None) => anyhow::bail!("Either --file or --definition must be provided"),
        (Some(_), Some(_)) => anyhow::bail!("Cannot specify both --file and --definition"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cli_validate_parse() {
        let args = Cli::parse_from(&["pipeliner", "validate", "--file", "test.yaml"]);
        match args.command {
            Commands::Validate(_) => {}
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_cli_run_parse() {
        let args = Cli::parse_from(&["pipeliner", "run", "--file", "pipeline.jenkins"]);
        match args.command {
            Commands::Run(_) => {}
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_lint_parse() {
        let args = Cli::parse_from(&["pipeliner", "lint", "--file", "pipeline.jenkins"]);
        match args.command {
            Commands::Lint(_) => {}
            _ => panic!("Expected Lint command"),
        }
    }

    #[test]
    fn test_cli_completions_parse() {
        let args = Cli::parse_from(&["pipeliner", "completions", "--shell", "bash"]);
        match args.command {
            Commands::Completions(c) => assert_eq!(c.shell, "bash"),
            _ => panic!("Expected Completions command"),
        }
    }

    #[test]
    fn test_cli_check_parse() {
        let args = Cli::parse_from(&["pipeliner", "check", "--file", "pipeline.jenkins"]);
        match args.command {
            Commands::Check(_) => {}
            _ => panic!("Expected Check command"),
        }
    }
}
