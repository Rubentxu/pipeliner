//! CLI tools for rustline
//!
//! Provides utilities that wrap rust-script functionality:
//! - `check`: Validate pipeline syntax via rust-script
//! - `lint`: Analyze pipelines for best practices
//! - `doc`: Generate documentation from pipeline comments
//! - `export`: Convert pipelines to CI/CD formats
//! - `completions`: Generate shell completions
//! - `run`: Execute pipelines via rust-script

pub mod check;
pub mod completions;
pub mod doc;
pub mod export;
pub mod lint;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// CLI arguments for rustline
#[derive(Parser, Debug)]
#[command(name = "rustline")]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Validate pipeline syntax using rust-script
    Check {
        /// Pipeline file to validate
        file: PathBuf,
        /// Show cargo build output
        #[arg(short, long)]
        cargo_output: bool,
    },

    /// Analyze pipeline for best practices
    Lint {
        /// Pipeline file to lint
        file: PathBuf,
        /// Output format
        #[arg(short, long, value_enum)]
        format: Option<LintFormat>,
        /// Minimum severity to show
        #[arg(short, long, value_enum)]
        severity: Option<LintSeverityArg>,
        /// Show suggestions
        #[arg(long)]
        suggestions: bool,
    },

    /// Generate documentation from pipeline comments
    Doc {
        /// Pipeline file to document
        file: PathBuf,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Output format
        #[arg(short, long, value_enum)]
        format: Option<DocFormatArg>,
    },

    /// Export pipeline to CI/CD formats
    Export {
        /// Pipeline file to export
        file: PathBuf,
        /// Export format
        #[arg(short, long, value_enum)]
        format: ExportFormatArg,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Pipeline name (defaults to filename)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Generate shell completions
    Completions {
        /// Shell type
        #[arg(value_enum)]
        shell: ShellArg,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum LintFormat {
    Text,
    Json,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum LintSeverityArg {
    Info,
    Warning,
    Error,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum DocFormatArg {
    Markdown,
    Json,
    Html,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum ExportFormatArg {
    GitHub,
    Gitlab,
    Jenkins,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum ShellArg {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

/// Build the CLI command for completion generation
pub fn build_cli() -> clap::Command {
    clap::Command::new("rustline")
        .version(env!("CARGO_PKG_VERSION"))
        .about("CLI tools for rustline pipelines")
}

/// Parse and execute CLI arguments
pub fn run() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Check { file, cargo_output } => {
            check::check_pipeline(&file, cargo_output)?;
        }
        Command::Lint {
            file,
            format,
            severity,
            suggestions,
        } => {
            let config = lint::LintConfig {
                min_severity: match severity {
                    Some(LintSeverityArg::Info) => lint::LintSeverity::Info,
                    Some(LintSeverityArg::Warning) => lint::LintSeverity::Warning,
                    Some(LintSeverityArg::Error) => lint::LintSeverity::Error,
                    None => lint::LintSeverity::Info,
                },
                show_suggestions: suggestions,
                format: match format {
                    Some(LintFormat::Json) => lint::OutputFormat::Json,
                    Some(LintFormat::Text) | None => lint::OutputFormat::Text,
                },
            };

            let messages = lint::lint_pipeline(&file, &config)?;
            let output = lint::format_lint_messages(&messages, config.format);
            println!("{}", output);
        }
        Command::Doc {
            file,
            output,
            format,
        } => {
            let doc_format = match format {
                Some(DocFormatArg::Json) => doc::DocFormat::Json,
                Some(DocFormatArg::Html) => doc::DocFormat::Html,
                Some(DocFormatArg::Markdown) | None => doc::DocFormat::Markdown,
            };

            let documentation = doc::generate_doc(&file, doc_format)?;

            if let Some(output_path) = output {
                doc::save_doc(&documentation, &output_path)?;
            } else {
                println!("{}", documentation);
            }
        }
        Command::Export {
            file,
            format,
            output,
            name,
        } => {
            let export_format = match format {
                ExportFormatArg::GitHub => export::ExportFormat::GitHubActions,
                ExportFormatArg::Gitlab => export::ExportFormat::GitLabCI,
                ExportFormatArg::Jenkins => export::ExportFormat::Jenkinsfile,
            };

            let config = export::ExportConfig {
                format: export_format,
                output: None,
                name: name.unwrap_or_else(|| {
                    file.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("pipeline")
                        .to_string()
                }),
            };

            let exported = export::export_pipeline(&file, &config)?;

            if let Some(output_path) = output {
                export::save_export(&exported, &output_path)?;
            } else {
                println!("{}", exported);
            }
        }
        Command::Completions { shell, output } => {
            use clap_complete::Shell;

            let shell_enum = match shell {
                ShellArg::Bash => Shell::Bash,
                ShellArg::Zsh => Shell::Zsh,
                ShellArg::Fish => Shell::Fish,
                ShellArg::PowerShell => Shell::PowerShell,
            };

            let completions = completions::generate_completions(shell_enum)?;

            if let Some(output_path) = output {
                completions::save_completions(&completions, &output_path)?;
            } else {
                println!("{}", completions);
            }
        }
    }

    Ok(())
}
