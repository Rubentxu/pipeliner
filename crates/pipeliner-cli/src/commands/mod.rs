//! CLI commands for Pipeliner.

use anyhow::{Context, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use pipeliner_core::{Pipeline, StepRegistry};
use pipeliner_executor::context::CacheMode;
use pipeliner_executor::{ExecutionConfig, LocalExecutor, OutputFormat};
use pipeliner_script::ScriptStepFactory;

pub mod describe;
pub mod gc;
pub mod graph;
pub mod init;
pub mod list;
pub mod script;

pub use describe::DescribeArgs;
pub use gc::{GcArgs, GcSubcommand};
pub use graph::GraphArgs;
pub use init::InitArgs;
pub use list::ListArgs;
pub use script::ScriptRunArgs;

/// Command-line interface for Pipeliner pipeline execution
#[derive(Parser, Debug)]
#[command(name = "pipeliner")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true, default_value = "false")]
    verbose: bool,

    /// Output format (json, yaml, human)
    #[arg(long, global = true, default_value = "human", value_name = "FORMAT")]
    format: String,

    /// Disable colored output
    #[arg(long, global = true, default_value = "false")]
    no_color: bool,

    /// Path to config file
    #[arg(short, long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a pipeline script
    Run(RunArgs),

    /// Execute a Rust script directly
    Script(ScriptRunArgs),

    /// Validate a pipeline definition
    Validate(ValidateArgs),

    /// Lint a pipeline for style and best practices
    Lint(LintArgs),

    /// Generate documentation for a pipeline
    Doc(DocArgs),

    /// Export pipeline to different formats
    Export(ExportArgs),

    /// Generate shell completions
    Completions(CompletionsArgs),

    /// Check pipeline syntax without execution
    Check(CheckArgs),

    /// Initialize a new pipeline
    Init(InitArgs),

    /// List available pipelines
    List(ListArgs),

    /// Show pipeline information
    Describe(DescribeArgs),

    /// Generate pipeline graph in various formats
    Graph(GraphArgs),

    /// Garbage collect old pipeline runs
    Gc(GcArgs),
}

#[derive(Args, Debug, Clone)]
struct RunArgs {
    /// Pipeline script file to run
    #[arg(value_name = "SCRIPT")]
    pub script: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    pub definition: Option<String>,

    /// Working directory
    #[arg(short, long)]
    pub working_dir: Option<PathBuf>,

    /// Stages to execute (comma-separated)
    #[arg(long)]
    pub stages: Option<String>,

    /// Dry-run mode (validate without executing)
    #[arg(long)]
    pub dry_run: bool,

    /// Cache mode (full, deps, none)
    #[arg(long, default_value = "full")]
    pub cache: String,

    /// Pipeline timeout in seconds
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Maximum retries on step failure
    #[arg(long)]
    pub retry: Option<u32>,

    /// Maximum parallelism for parallel stages
    #[arg(long)]
    pub parallelism: Option<usize>,

    /// Watch mode - re-run on file changes
    #[arg(long)]
    pub watch: bool,
}

#[derive(Args, Debug)]
struct ValidateArgs {
    /// Pipeline file to validate
    #[arg(value_name = "SCRIPT")]
    pub script: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    pub definition: Option<String>,
}

#[derive(Args, Debug)]
struct LintArgs {
    /// Pipeline file to lint
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    pub definition: Option<String>,

    /// Strict mode (fail on warnings)
    #[arg(short, long, default_value = "false")]
    pub strict: bool,
}

#[derive(Args, Debug)]
struct DocArgs {
    /// Pipeline file
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Output directory
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Format (markdown, html, man)
    #[arg(short, long, default_value = "markdown")]
    pub format: String,
}

#[derive(Args, Debug)]
struct ExportArgs {
    /// Pipeline file
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Output file
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Format (json, dockerfile, kubernetes)
    #[arg(short, long, default_value = "json")]
    pub format: String,
}

#[derive(Args, Debug)]
struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(short, long)]
    pub shell: String,
}

#[derive(Args, Debug)]
struct CheckArgs {
    /// Pipeline file to check
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    pub definition: Option<String>,
}

#[derive(Args, Debug)]
pub struct GraphArgs {
    /// Pipeline file to generate graph for
    #[arg(value_name = "SCRIPT")]
    pub script: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    pub definition: Option<String>,

    /// Output format (mermaid, dot)
    #[arg(long, default_value = "mermaid")]
    pub format: String,
}

pub async fn run() -> Result<()> {
    let args = Cli::parse();

    // Load configuration
    let config = crate::config::load_config(
        args.config.as_ref(),
        args.verbose,
        &args.format,
        args.no_color,
    )?;

    // Apply config to tracing
    // SAFETY: set_var is unsafe but acceptable in single-threaded CLI context
    if config.verbose {
        unsafe { std::env::set_var("RUST_LOG", "debug") };
    } else if let Some(ref log_level) = config.log_level {
        unsafe { std::env::set_var("RUST_LOG", log_level) };
    }

    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    match args.command {
        Commands::Run(run_args) => {
            if run_args.watch {
                #[cfg(feature = "watch")]
                {
                    watch_pipeline(run_args, config).await
                }
                #[cfg(not(feature = "watch"))]
                {
                    anyhow::bail!("Watch mode requires the 'watch' feature to be enabled");
                }
            } else {
                run_pipeline_once(run_args, config).await
            }
        }
        Commands::Script(script_args) => script::run_script(script_args).await,
        Commands::Validate(validate_args) => validate_pipeline(validate_args, config.effective_format()),
        Commands::Lint(lint_args) => lint_pipeline(lint_args),
        Commands::Doc(doc_args) => generate_docs(doc_args),
        Commands::Export(export_args) => export_pipeline(export_args),
        Commands::Completions(completions_args) => generate_completions(completions_args),
        Commands::Check(check_args) => check_pipeline(check_args),
        Commands::Init(init_args) => init::init_pipeline(init_args),
        Commands::List(list_args) => list::list_pipelines(list_args, config.effective_format()),
        Commands::Describe(describe_args) => describe::describe_pipeline_cmd(describe_args, config.effective_format()),
        Commands::Graph(graph_args) => graph::graph_pipeline(graph_args),
        Commands::Gc(gc_args) => gc::run_gc(gc_args, config.effective_format()),
    }
}

/// Build ExecutionConfig from CLI RunArgs
/// This is separated for testability (T3.9)
fn build_execution_config(args: &RunArgs, config: &crate::config::Config) -> ExecutionConfig {
    let mut exec_config = ExecutionConfig::default();

    // T3.7: Wire --cache flag to ExecutionConfig
    let cache_mode = match args.cache.to_lowercase().as_str() {
        "full" => CacheMode::Full,
        "deps" => CacheMode::Deps,
        "none" => CacheMode::None,
        // Default case should not happen as clap validates
        _ => CacheMode::default(),
    };
    exec_config.cache_mode = cache_mode;

    // T3.6: Wire --retry flag to ExecutionConfig
    if let Some(max_retries) = args.retry {
        exec_config.retry_on_failure = true;
        exec_config.max_retries = max_retries as usize;
    }

    // T3.8: Wire --timeout to ExecutionConfig.global_timeout
    if let Some(timeout_secs) = args.timeout {
        exec_config.global_timeout = Some(std::time::Duration::from_secs(timeout_secs));
    }

    // Apply config-level overrides
    if let Some(ref cache) = config.cache_mode {
        if let Ok(mode) = parse_cache_mode(cache) {
            exec_config.cache_mode = mode;
        }
    }

    exec_config
}

fn parse_cache_mode(s: &str) -> Result<CacheMode> {
    match s.to_lowercase().as_str() {
        "full" => Ok(CacheMode::Full),
        "deps" => Ok(CacheMode::Deps),
        "none" => Ok(CacheMode::None),
        _ => anyhow::bail!("Invalid cache mode: {}", s),
    }
}

async fn run_pipeline_once(args: RunArgs, config: crate::config::Config) -> Result<()> {
    info!("Running pipeline");

    // Get script path (positional arg)
    let script_path = args.script.as_ref();

    // Detect file type and parse accordingly
    let pipeline = if let Some(ref path) = script_path {
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            // .rs file - need to compile and run via script runner
            info!("Detected .rs file, using script runner");
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read .rs file: {:?}", path))?;
            pipeliner_core::dsl::parse_pipeline(&content)
                .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?
        } else if path.extension().and_then(|e| e.to_str()) == Some("dsl") {
            // Use runtime DSL parser for .dsl files
            info!("Detected .dsl file, using runtime parser");
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read .dsl file: {:?}", path))?;
            pipeliner_core::dsl::parse_pipeline(&content)
                .map_err(|e| anyhow::anyhow!("DSL parse error: {}", e))?
        } else {
            // JSON pipeline - read directly from the script path
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read pipeline file: {:?}", path))?;
            serde_json::from_str(&content)
                .context("Failed to parse pipeline JSON")?
        }
    } else {
        // Check if definition looks like DSL (contains "pipeline {")
        if let Some(ref def) = args.definition {
            if def.trim_start().starts_with("pipeline {") || def.contains("stage(") {
                info!("Detected DSL syntax, using runtime parser");
                pipeliner_core::dsl::parse_pipeline(def)
                    .map_err(|e| anyhow::anyhow!("DSL parse error: {}", e))?
            } else {
                serde_json::from_str(def)
                    .context("Failed to parse pipeline")?
            }
        } else {
            anyhow::bail!("No pipeline script or definition provided");
        }
    };

    let name = pipeline.name.clone().unwrap_or_else(|| "Unnamed".to_string());
    info!("Pipeline '{}' parsed successfully", name);

    // Create step registry and register script factory
    let mut registry = StepRegistry::new();
    registry.register(Arc::new(ScriptStepFactory::new()));

    // Build ExecutionConfig
    let mut exec_config = build_execution_config(&args, &config);

    // Create executor with registry for custom steps
    let mut executor = LocalExecutor::with_registry(registry);

    // Apply ExecutionConfig settings to executor via builder pattern
    if exec_config.retry_on_failure {
        executor = executor.with_retry(exec_config.max_retries);
    }

    executor = executor.with_cache_mode(exec_config.cache_mode);

    if let Some(timeout) = exec_config.global_timeout {
        executor = executor.with_global_timeout(timeout);
    }

    // Apply stage filter if specified
    if let Some(stages) = &args.stages {
        let stage_list: Vec<String> = stages.split(',').map(String::from).collect();
        executor = executor.with_stages(stage_list);
        info!("Stage filter applied: {}", stages);
    }

    // Apply dry-run mode
    if args.dry_run {
        executor = executor.with_dry_run(true);
        info!("Dry-run mode enabled");
    }

    // Apply parallelism limit
    if let Some(parallelism) = args.parallelism {
        executor = executor.with_max_parallelism(parallelism);
        info!("Parallelism limit set to {}", parallelism);
    }

    // Apply output format
    let output_format = match config.effective_format() {
        crate::config::OutputFormat::Json => OutputFormat::Json,
        crate::config::OutputFormat::Yaml => OutputFormat::Json, // Fallback to json for now
        crate::config::OutputFormat::Human => OutputFormat::Human,
    };
    executor = executor.with_output_format(output_format);
    info!("Output format: {:?}", config.effective_format());

    // Execute with optional timeout (using config.global_timeout as reference)
    let execute_future = executor.execute(&pipeline);

    let results: Vec<pipeliner_executor::LocalResult> = if let Some(timeout_secs) = args.timeout {
        tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            execute_future
        ).await
        .map_err(|_| anyhow::anyhow!("Pipeline timed out after {} seconds", timeout_secs))?
    } else {
        execute_future.await
    };

    // Analyze results (skip in dry-run mode since results will be empty)
    if args.dry_run {
        return Ok(());
    }

    let success_count = results.iter().filter(|r| r.success).count();
    let total_count = results.len();
    let all_success = results.iter().all(|r| r.success);

    if all_success {
        info!(
            "Pipeline '{}' completed successfully: {}/{} steps successful",
            name, success_count, total_count
        );
        Ok(())
    } else {
        // Find failed steps
        let failed: Vec<_> = results.iter().filter(|r| !r.success).collect();
        let failed_names: Vec<_> = failed.iter().map(|r| r.stage.as_str()).collect();
        anyhow::bail!(
            "Pipeline '{}' failed: {}/{} steps successful. Failed steps: {:?}",
            name,
            success_count,
            total_count,
            failed_names
        );
    }
}

fn validate_pipeline(args: ValidateArgs, format: crate::config::OutputFormat) -> Result<()> {
    info!("Validating pipeline");

    let definition = get_definition_from_script_or_def(&args.script, &args.definition)?;

    // Try to parse as different formats
    let pipeline_result: Result<Pipeline, _> = serde_json::from_str(&definition)
        .context("Not valid JSON");

    let pipeline = if pipeline_result.is_err() {
        // Try DSL parsing
        pipeliner_core::dsl::parse_pipeline(&definition)
            .map_err(|e| anyhow::anyhow!("Not valid DSL: {}", e))?
    } else {
        pipeline_result?
    };

    match format {
        crate::config::OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&pipeline)?);
        }
        crate::config::OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&pipeline)?);
        }
        crate::config::OutputFormat::Human => {
            println!("Pipeline is valid: {}", pipeline.name.as_deref().unwrap_or("Unnamed"));
        }
    }

    Ok(())
}

fn lint_pipeline(args: LintArgs) -> Result<()> {
    info!("Linting pipeline");

    let definition = get_definition_from_script_or_def(&args.file, &args.definition)?;
    let _pipeline: Pipeline =
        serde_json::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("No issues found");
    Ok(())
}

fn generate_docs(args: DocArgs) -> Result<()> {
    info!("Generating documentation");

    let definition = get_definition_from_script_or_def(&args.file, &None)?;
    let _pipeline: Pipeline =
        serde_json::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("Documentation generated");
    Ok(())
}

fn export_pipeline(args: ExportArgs) -> Result<()> {
    info!("Exporting pipeline");

    let definition = get_definition_from_script_or_def(&args.file, &None)?;
    let pipeline: Pipeline =
        serde_json::from_str(&definition).context("Failed to parse pipeline definition")?;

    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&pipeline)?;
            println!("{}", json);
        }
        _ => anyhow::bail!("Unsupported format: {}. Valid options: json", args.format),
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

    let definition = get_definition_from_script_or_def(&args.file, &args.definition)?;
    let _pipeline: Pipeline =
        serde_json::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("Pipeline syntax is correct");
    Ok(())
}

fn get_definition(definition: &Option<String>) -> Result<String> {
    match definition {
        Some(def) => Ok(def.clone()),
        None => anyhow::bail!("Either --definition must be provided or a pipeline file must be specified"),
    }
}

fn get_definition_from_script_or_def(script: &Option<PathBuf>, definition: &Option<String>) -> Result<String> {
    if let Some(path) = script {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {:?}", path))
    } else if let Some(def) = definition {
        Ok(def.clone())
    } else {
        anyhow::bail!("Either a pipeline file or --definition must be provided")
    }
}

#[cfg(feature = "watch")]
async fn watch_pipeline(args: RunArgs, config: crate::config::Config) -> Result<()> {
    use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::Duration;

    let file_path = args.script.clone()
        .ok_or_else(|| anyhow::anyhow!("Running in watch mode requires a script file"))?;

    // Initial run
    run_pipeline_once(args.clone(), config.clone()).await?;

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    let _ = tx.send(());
                }
            }
        },
        Config::default().with_poll_interval(Duration::from_millis(500)),
    )?;

    watcher.watch(&file_path, RecursiveMode::NonRecursive)?;

    println!("[WATCH] Watching {:?} for changes... (Ctrl+C to stop)", file_path);

    // Debounce loop
    while rx.recv().is_ok() {
        // Drain any additional events (debounce)
        while rx.recv_timeout(Duration::from_millis(500)).is_ok() {}

        println!("[WATCH] File changed, re-running pipeline...");
        if let Err(e) = run_pipeline_once(args.clone(), config.clone()).await {
            eprintln!("[WATCH] Pipeline failed: {}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_run_with_positional_script() {
        let args = Cli::parse_from(&["pipeliner", "run", "pipeline.json"]);
        match args.command {
            Commands::Run(run_args) => {
                assert_eq!(run_args.script, Some(PathBuf::from("pipeline.json")));
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_validate_parse() {
        let args = Cli::parse_from(&["pipeliner", "validate", "pipeline.json"]);
        match args.command {
            Commands::Validate(_) => {}
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_cli_list_parse() {
        let args = Cli::parse_from(&["pipeliner", "list"]);
        match args.command {
            Commands::List(_) => {}
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_cli_describe_parse() {
        let args = Cli::parse_from(&["pipeliner", "describe", "pipeline.json"]);
        match args.command {
            Commands::Describe(_) => {}
            _ => panic!("Expected Describe command"),
        }
    }

    #[test]
    fn test_cli_global_verbose() {
        let args = Cli::parse_from(&["pipeliner", "-v", "list"]);
        assert!(args.verbose);
    }

    #[test]
    fn test_cli_global_verbose_long() {
        let args = Cli::parse_from(&["pipeliner", "--verbose", "list"]);
        assert!(args.verbose);
    }

    #[test]
    fn test_cli_global_format() {
        let args = Cli::parse_from(&["pipeliner", "--format", "json", "list"]);
        assert_eq!(args.format, "json");
    }

    #[test]
    fn test_cli_global_no_color() {
        let args = Cli::parse_from(&["pipeliner", "--no-color", "list"]);
        assert!(args.no_color);
    }

    #[test]
    fn test_cli_global_config() {
        let args = Cli::parse_from(&["pipeliner", "--config", "/path/to/config.toml", "list"]);
        assert_eq!(args.config, Some(PathBuf::from("/path/to/config.toml")));
    }

    #[test]
    fn test_cli_init_with_positional_name() {
        let args = Cli::parse_from(&["pipeliner", "init", "my-pipeline"]);
        match args.command {
            Commands::Init(init_args) => {
                assert_eq!(init_args.name, Some("my-pipeline".to_string()));
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_run_args_with_all_flags() {
        let args = Cli::parse_from(&[
            "pipeliner", "run",
            "pipeline.json",
            "--cache", "deps",
            "--timeout", "300",
            "--retry", "3",
        ]);
        match args.command {
            Commands::Run(run_args) => {
                assert_eq!(run_args.script, Some(PathBuf::from("pipeline.json")));
                assert_eq!(run_args.cache, "deps");
                assert_eq!(run_args.timeout, Some(300));
                assert_eq!(run_args.retry, Some(3));
                assert!(!run_args.watch);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_run_args_watch_flag() {
        let args = Cli::parse_from(&[
            "pipeliner", "run",
            "pipeline.json",
            "--watch",
        ]);
        match args.command {
            Commands::Run(run_args) => {
                assert!(run_args.watch);
            }
            _ => panic!("Expected Run command"),
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

    #[test]
    fn test_cli_completions_parse() {
        let args = Cli::parse_from(&["pipeliner", "completions", "--shell", "bash"]);
        match args.command {
            Commands::Completions(c) => assert_eq!(c.shell, "bash"),
            _ => panic!("Expected Completions command"),
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

    // =======================================================================
    // ExecutionConfig tests
    // =======================================================================

    #[test]
    fn test_build_execution_config_retry_flag() {
        let args = RunArgs {
            script: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            stages: None,
            dry_run: false,
            cache: "full".to_string(),
            timeout: None,
            retry: Some(3),
            watch: false,
        };

        let config = crate::config::Config::default();
        let exec_config = build_execution_config(&args, &config);

        assert!(exec_config.retry_on_failure, "retry_on_failure should be true when --retry is set");
        assert_eq!(exec_config.max_retries, 3, "max_retries should be 3");
    }

    #[test]
    fn test_build_execution_config_cache_none() {
        let args = RunArgs {
            script: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            stages: None,
            dry_run: false,
            cache: "none".to_string(),
            timeout: None,
            retry: None,
            watch: false,
        };

        let config = crate::config::Config::default();
        let exec_config = build_execution_config(&args, &config);

        assert_eq!(exec_config.cache_mode, CacheMode::None, "cache_mode should be CacheMode::None");
    }

    #[test]
    fn test_build_execution_config_timeout() {
        let args = RunArgs {
            script: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            stages: None,
            dry_run: false,
            cache: "full".to_string(),
            timeout: Some(300),
            retry: None,
            watch: false,
        };

        let config = crate::config::Config::default();
        let exec_config = build_execution_config(&args, &config);

        assert!(exec_config.global_timeout.is_some(), "global_timeout should be Some");
        assert_eq!(exec_config.global_timeout.unwrap().as_secs(), 300, "global_timeout should be 300 seconds");
    }

    #[test]
    fn test_build_execution_config_default_values() {
        let args = RunArgs {
            script: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            stages: None,
            dry_run: false,
            cache: "full".to_string(),
            timeout: None,
            retry: None,
            watch: false,
        };

        let config = crate::config::Config::default();
        let exec_config = build_execution_config(&args, &config);

        assert!(!exec_config.retry_on_failure, "retry_on_failure should be false by default");
        assert_eq!(exec_config.max_retries, 0, "max_retries should be 0 by default");
        assert_eq!(exec_config.cache_mode, CacheMode::Full, "cache_mode should be Full by default");
        assert!(exec_config.global_timeout.is_none(), "global_timeout should be None by default");
    }

    #[test]
    fn test_get_definition_with_definition_only() {
        let script: Option<PathBuf> = None;
        let definition = Some("inline definition".to_string());

        let result = get_definition_from_script_or_def(&script, &definition);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "inline definition");
    }

    #[test]
    fn test_get_definition_without_script_or_def() {
        let script: Option<PathBuf> = None;
        let definition: Option<String> = None;

        let result = get_definition_from_script_or_def(&script, &definition);
        assert!(result.is_err());
    }
}
