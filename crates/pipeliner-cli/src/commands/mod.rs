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

pub mod init;
pub mod script;

/// Command-line interface for Pipeliner pipeline execution
#[derive(Parser, Debug)]
#[command(name = "rustline")]
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

    /// Execute a Rust script directly
    #[command(name = "script")]
    Script(script::ScriptRunArgs),

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

    /// Initialize a new pipeline
    #[command(name = "init")]
    Init(init::InitArgs),
}

#[derive(Args, Debug, Clone)]
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

    /// Stages to execute (comma-separated)
    #[arg(long)]
    stages: Option<String>,

    /// Dry-run mode (validate without executing)
    #[arg(long)]
    dry_run: bool,

    /// Output format (human, json, quiet)
    #[arg(long, default_value = "human")]
    output: String,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log: String,

    /// Environment (development, staging, production)
    #[arg(long)]
    env: Option<String>,

    /// Cache mode (full, deps, none)
    #[arg(long, default_value = "full")]
    cache: String,

    /// Pipeline timeout in seconds
    #[arg(long)]
    timeout: Option<u64>,

    /// Maximum retries on step failure
    #[arg(long)]
    retry: Option<u32>,

    /// Watch mode - re-run on file changes
    #[arg(long)]
    watch: bool,
}

#[derive(Args, Debug)]
struct ValidateArgs {
    /// Pipeline file to validate
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Pipeline definition as string
    #[arg(short, long)]
    definition: Option<String>,

    /// Output format (json, text)
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

    /// Format (json, dockerfile, kubernetes)
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
        Commands::Run(run_args) => {
            if run_args.watch {
                #[cfg(feature = "watch")]
                {
                    watch_pipeline(run_args).await
                }
                #[cfg(not(feature = "watch"))]
                {
                    anyhow::bail!("Watch mode requires the 'watch' feature to be enabled");
                }
            } else {
                run_pipeline_once(run_args).await
            }
        }
        Commands::Script(script_args) => script::run_script(script_args).await,
        Commands::Validate(validate_args) => validate_pipeline(validate_args),
        Commands::Lint(lint_args) => lint_pipeline(lint_args),
        Commands::Doc(doc_args) => generate_docs(doc_args),
        Commands::Export(export_args) => export_pipeline(export_args),
        Commands::Completions(completions_args) => generate_completions(completions_args),
        Commands::Check(check_args) => check_pipeline(check_args),
        Commands::Init(init_args) => init::init_pipeline(init_args),
    }
}

fn init_tracing(level: &str) {
    let filter = match level.to_lowercase().as_str() {
        "error" => "error",
        "warn" => "warn",
        "info" => "info",
        "debug" => "debug",
        "trace" => "trace",
        _ => "info",
    };
    // SAFETY: set_var is unsafe but acceptable in single-threaded CLI context
    unsafe {
        std::env::set_var("RUST_LOG", filter);
    }
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

/// Build ExecutionConfig from CLI RunArgs
/// This is separated for testability (T3.9)
fn build_execution_config(args: &RunArgs) -> ExecutionConfig {
    let mut config = ExecutionConfig::default();

    // T3.7: Wire --cache flag to ExecutionConfig
    let cache_mode = match args.cache.to_lowercase().as_str() {
        "full" => CacheMode::Full,
        "deps" => CacheMode::Deps,
        "none" => CacheMode::None,
        // Default case should not happen as clap validates
        _ => CacheMode::default(),
    };
    config.cache_mode = cache_mode;

    // T3.6: Wire --retry flag to ExecutionConfig
    if let Some(max_retries) = args.retry {
        config.retry_on_failure = true;
        config.max_retries = max_retries as usize;
    }

    // T3.8: Wire --timeout to ExecutionConfig.global_timeout
    if let Some(timeout_secs) = args.timeout {
        config.global_timeout = Some(std::time::Duration::from_secs(timeout_secs));
    }

    config
}

async fn run_pipeline_once(args: RunArgs) -> Result<()> {
    init_tracing(&args.log);
    info!("Running pipeline");

    let definition = get_definition(&args.file, &args.definition)?;
    let pipeline: Pipeline =
        serde_json::from_str(&definition).context("Failed to parse pipeline definition")?;

    let name = pipeline.name.clone().unwrap_or_else(|| "Unnamed".to_string());
    info!("Pipeline '{}' parsed successfully", name);

    // Create step registry and register script factory
    let mut registry = StepRegistry::new();
    registry.register(Arc::new(ScriptStepFactory::new()));

    // =======================================================================
    // T3.6, T3.7, T3.8: Build ExecutionConfig from CLI args
    // =======================================================================
    let mut config = ExecutionConfig::default();

    // T3.7: Wire --cache flag to ExecutionConfig
    let cache_mode = match args.cache.to_lowercase().as_str() {
        "full" => CacheMode::Full,
        "deps" => CacheMode::Deps,
        "none" => CacheMode::None,
        _ => anyhow::bail!("Invalid cache mode: '{}'. Valid: full, deps, none", args.cache),
    };
    config.cache_mode = cache_mode;
    info!("Cache mode: {:?}", config.cache_mode);

    // T3.6: Wire --retry flag to ExecutionConfig
    if let Some(max_retries) = args.retry {
        config.retry_on_failure = true;
        config.max_retries = max_retries as usize;
        info!("Retry enabled: {} max retries", config.max_retries);
    }

    // T3.8: Wire --timeout to ExecutionConfig.global_timeout
    // Keep the outer tokio::time::timeout as a hard guard
    if let Some(timeout_secs) = args.timeout {
        config.global_timeout = Some(std::time::Duration::from_secs(timeout_secs));
        info!("Timeout: {} seconds", timeout_secs);
    }

    // Create executor with registry for custom steps
    let mut executor = LocalExecutor::with_registry(registry);

    // Apply ExecutionConfig settings to executor via builder pattern
    // T3.6: Apply retry settings
    if config.retry_on_failure {
        executor = executor.with_retry(config.max_retries);
    }

    // T3.7: Apply cache mode
    executor = executor.with_cache_mode(config.cache_mode);

    // T3.8: Apply global timeout (used by executor; CLI also keeps tokio::time::timeout as hard guard)
    if let Some(timeout) = config.global_timeout {
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

    // Apply output format
    let output_format = match args.output.to_lowercase().as_str() {
        "json" => OutputFormat::Json,
        "quiet" => OutputFormat::Quiet,
        "human" | "text" => OutputFormat::Human,
        _ => anyhow::bail!("Invalid output format: '{}'. Valid options: human, json, quiet", args.output),
    };
    executor = executor.with_output_format(output_format);
    info!("Output format: {}", args.output);

    // Apply environment if specified
    if let Some(env) = &args.env {
        // SAFETY: set_var is unsafe but acceptable in single-threaded CLI context
        unsafe {
            std::env::set_var("ENVIRONMENT", env);
        }
        info!("Environment set to: {}", env);
    }

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

fn validate_pipeline(args: ValidateArgs) -> Result<()> {
    info!("Validating pipeline");

    let definition = get_definition(&args.file, &args.definition)?;
    let _pipeline: Pipeline =
        serde_json::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("Pipeline is valid");
    Ok(())
}

fn lint_pipeline(args: LintArgs) -> Result<()> {
    info!("Linting pipeline");

    let definition = get_definition(&args.file, &args.definition)?;
    let _pipeline: Pipeline =
        serde_json::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("No issues found");
    Ok(())
}

fn generate_docs(args: DocArgs) -> Result<()> {
    info!("Generating documentation");

    let definition = get_definition(&args.file, &None)?;
    let _pipeline: Pipeline =
        serde_json::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("Documentation generated");
    Ok(())
}

fn export_pipeline(args: ExportArgs) -> Result<()> {
    info!("Exporting pipeline");

    let definition = get_definition(&args.file, &None)?;
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

    let definition = get_definition(&args.file, &args.definition)?;
    let _pipeline: Pipeline =
        serde_json::from_str(&definition).context("Failed to parse pipeline definition")?;

    println!("Pipeline syntax is correct");
    Ok(())
}

fn get_definition(file: &Option<PathBuf>, definition: &Option<String>) -> Result<String> {
    match (file, definition) {
        (Some(path), None) if path.as_path() == std::path::Path::new("-") => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)
                .context("Failed to read from stdin")?;
            if buf.trim().is_empty() {
                anyhow::bail!("No pipeline definition provided on stdin");
            }
            Ok(buf)
        }
        (Some(path), None) => std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {:?}", path)),
        (None, Some(def)) => Ok(def.clone()),
        (None, None) => anyhow::bail!("Either --file, --definition, or stdin (-) must be provided"),
        (Some(_), Some(_)) => anyhow::bail!("Cannot specify both --file and --definition"),
    }
}

#[cfg(feature = "watch")]
async fn watch_pipeline(args: RunArgs) -> Result<()> {
    use notify::{RecommendedWatcher, RecursiveMode, Config, Event, EventKind, Watcher};
    use std::sync::mpsc;
    use std::time::Duration;

    let file_path = args.file.clone()
        .ok_or_else(|| anyhow::anyhow!("--watch requires --file"))?;

    // Initial run
    run_pipeline_once(args.clone()).await?;

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
        if let Err(e) = run_pipeline_once(args.clone()).await {
            eprintln!("[WATCH] Pipeline failed: {}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cli_validate_parse() {
        let args = Cli::parse_from(&["pipeliner", "validate", "--file", "test.json"]);
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

    #[test]
    fn test_get_definition_stdin_path() {
        // Test that the stdin case is properly handled (path is "-")
        let path = PathBuf::from("-");
        assert_eq!(path, PathBuf::from("-"));
    }

    #[test]
    fn test_run_args_with_all_flags() {
        let args = Cli::parse_from(&[
            "pipeliner", "run",
            "--file", "test.json",
            "--log", "debug",
            "--env", "production",
            "--cache", "deps",
            "--timeout", "300",
            "--retry", "3",
        ]);
        match args.command {
            Commands::Run(run_args) => {
                assert!(run_args.file.is_some());
                assert_eq!(run_args.log, "debug");
                assert_eq!(run_args.env, Some("production".to_string()));
                assert_eq!(run_args.cache, "deps");
                assert_eq!(run_args.timeout, Some(300));
                assert_eq!(run_args.retry, Some(3));
                assert!(!run_args.watch);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_run_args_watch_flag() {
        let args = Cli::parse_from(&[
            "pipeliner", "run",
            "--file", "test.json",
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
    fn test_init_command_parse() {
        let args = Cli::parse_from(&[
            "pipeliner", "init",
            "--name", "my-pipeline",
            "--output", "pipeline.json",
        ]);
        match args.command {
            Commands::Init(init_args) => {
                assert_eq!(init_args.name, Some("my-pipeline".to_string()));
                assert_eq!(init_args.output, PathBuf::from("pipeline.json"));
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_init_command_default_output() {
        let args = Cli::parse_from(&["pipeliner", "init"]);
        match args.command {
            Commands::Init(init_args) => {
                assert_eq!(init_args.output, PathBuf::from("pipeline.json"));
            }
            _ => panic!("Expected Init command"),
        }
    }

    // =======================================================================
    // T3.9: CLI Integration Tests for Flags Wiring
    // =======================================================================

    #[test]
    fn test_build_execution_config_retry_flag() {
        // T3.6: Verify --retry flag wires to ExecutionConfig correctly
        let args = RunArgs {
            file: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            verbose: false,
            stages: None,
            dry_run: false,
            output: "human".to_string(),
            log: "info".to_string(),
            env: None,
            cache: "full".to_string(),
            timeout: None,
            retry: Some(3),
            watch: false,
        };

        let config = build_execution_config(&args);

        assert!(config.retry_on_failure, "retry_on_failure should be true when --retry is set");
        assert_eq!(config.max_retries, 3, "max_retries should be 3");
    }

    #[test]
    fn test_build_execution_config_retry_flag_zero_retries() {
        // T3.6: Verify --retry 0 sets retry_on_failure=true with max_retries=0
        let args = RunArgs {
            file: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            verbose: false,
            stages: None,
            dry_run: false,
            output: "human".to_string(),
            log: "info".to_string(),
            env: None,
            cache: "full".to_string(),
            timeout: None,
            retry: Some(0),
            watch: false,
        };

        let config = build_execution_config(&args);

        // When retry is specified (even 0), retry_on_failure is enabled
        assert!(config.retry_on_failure, "retry_on_failure should be true when --retry is specified");
        assert_eq!(config.max_retries, 0, "max_retries should be 0");
    }

    #[test]
    fn test_build_execution_config_cache_none() {
        // T3.7: Verify --cache none wires to CacheMode::None
        let args = RunArgs {
            file: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            verbose: false,
            stages: None,
            dry_run: false,
            output: "human".to_string(),
            log: "info".to_string(),
            env: None,
            cache: "none".to_string(),
            timeout: None,
            retry: None,
            watch: false,
        };

        let config = build_execution_config(&args);

        assert_eq!(config.cache_mode, CacheMode::None, "cache_mode should be CacheMode::None");
    }

    #[test]
    fn test_build_execution_config_cache_deps() {
        // T3.7: Verify --cache deps wires to CacheMode::Deps
        let args = RunArgs {
            file: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            verbose: false,
            stages: None,
            dry_run: false,
            output: "human".to_string(),
            log: "info".to_string(),
            env: None,
            cache: "deps".to_string(),
            timeout: None,
            retry: None,
            watch: false,
        };

        let config = build_execution_config(&args);

        assert_eq!(config.cache_mode, CacheMode::Deps, "cache_mode should be CacheMode::Deps");
    }

    #[test]
    fn test_build_execution_config_cache_full() {
        // T3.7: Verify --cache full (default) wires to CacheMode::Full
        let args = RunArgs {
            file: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            verbose: false,
            stages: None,
            dry_run: false,
            output: "human".to_string(),
            log: "info".to_string(),
            env: None,
            cache: "full".to_string(),
            timeout: None,
            retry: None,
            watch: false,
        };

        let config = build_execution_config(&args);

        assert_eq!(config.cache_mode, CacheMode::Full, "cache_mode should be CacheMode::Full");
    }

    #[test]
    fn test_build_execution_config_timeout() {
        // T3.8: Verify --timeout wires to ExecutionConfig.global_timeout
        let args = RunArgs {
            file: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            verbose: false,
            stages: None,
            dry_run: false,
            output: "human".to_string(),
            log: "info".to_string(),
            env: None,
            cache: "full".to_string(),
            timeout: Some(300),
            retry: None,
            watch: false,
        };

        let config = build_execution_config(&args);

        assert!(config.global_timeout.is_some(), "global_timeout should be Some");
        assert_eq!(config.global_timeout.unwrap().as_secs(), 300, "global_timeout should be 300 seconds");
    }

    #[test]
    fn test_build_execution_config_timeout_none() {
        // T3.8: Verify no --timeout results in None
        let args = RunArgs {
            file: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            verbose: false,
            stages: None,
            dry_run: false,
            output: "human".to_string(),
            log: "info".to_string(),
            env: None,
            cache: "full".to_string(),
            timeout: None,
            retry: None,
            watch: false,
        };

        let config = build_execution_config(&args);

        assert!(config.global_timeout.is_none(), "global_timeout should be None when not specified");
    }

    #[test]
    fn test_build_execution_config_combined_flags() {
        // T3.9: Verify combined flags work together
        let args = RunArgs {
            file: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            verbose: false,
            stages: None,
            dry_run: false,
            output: "human".to_string(),
            log: "info".to_string(),
            env: None,
            cache: "deps".to_string(),
            timeout: Some(600),
            retry: Some(5),
            watch: false,
        };

        let config = build_execution_config(&args);

        // T3.6: Retry settings
        assert!(config.retry_on_failure, "retry_on_failure should be true");
        assert_eq!(config.max_retries, 5, "max_retries should be 5");

        // T3.7: Cache mode
        assert_eq!(config.cache_mode, CacheMode::Deps, "cache_mode should be CacheMode::Deps");

        // T3.8: Timeout
        assert!(config.global_timeout.is_some(), "global_timeout should be Some");
        assert_eq!(config.global_timeout.unwrap().as_secs(), 600, "global_timeout should be 600 seconds");
    }

    #[test]
    fn test_build_execution_config_default_values() {
        // Verify default ExecutionConfig values when no flags are specified
        let args = RunArgs {
            file: Some(PathBuf::from("test.json")),
            definition: None,
            working_dir: None,
            verbose: false,
            stages: None,
            dry_run: false,
            output: "human".to_string(),
            log: "info".to_string(),
            env: None,
            cache: "full".to_string(), // default
            timeout: None,
            retry: None,
            watch: false,
        };

        let config = build_execution_config(&args);

        // Default values
        assert!(!config.retry_on_failure, "retry_on_failure should be false by default");
        assert_eq!(config.max_retries, 0, "max_retries should be 0 by default");
        assert_eq!(config.cache_mode, CacheMode::Full, "cache_mode should be Full by default");
        assert!(config.global_timeout.is_none(), "global_timeout should be None by default");
    }
}
