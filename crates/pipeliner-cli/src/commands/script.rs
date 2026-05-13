//! Script execution subcommand for `rustline run`.

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;
use tracing::info;

use pipeliner_script::{
    Manifest, ScriptCompiler, ScriptRunner,
};
use pipeliner_script::runner::{PipelineContext, ScriptConfig};

/// Arguments for the `rustline run` subcommand.
#[derive(Args, Debug)]
pub struct ScriptRunArgs {
    /// Path to the Rust script file to execute.
    #[arg(value_name = "SCRIPT")]
    pub script: PathBuf,

    /// Additional cargo dependencies to merge with manifest dependencies.
    /// Can be specified multiple times: `-d serde -d tokio`
    #[arg(short = 'd', long = "dep", value_name = "DEP")]
    pub deps: Vec<String>,

    /// Arguments to pass to the script.
    /// Use `--` to separate rustline arguments from script arguments.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub script_args: Vec<String>,
}

/// Run a Rust script directly.
pub async fn run_script(args: ScriptRunArgs) -> Result<()> {
    let script_path = &args.script;

    // Verify script file exists
    if !script_path.exists() {
        anyhow::bail!("Script not found: {}", script_path.display());
    }

    // Verify it's a .rs file
    if script_path.extension().and_then(|s| s.to_str()) != Some("rs") {
        anyhow::bail!("Script must have .rs extension: {}", script_path.display());
    }

    info!("Executing script '{}'", script_path.display());

    // Read the script file
    let content = std::fs::read_to_string(script_path)
        .with_context(|| format!("Failed to read script '{}'", script_path.display()))?;

    // Parse manifest from the script
    let manifest = Manifest::parse(&content)
        .with_context(|| "Failed to parse script manifest")?;

    // Merge inline deps with manifest deps
    let mut all_deps = manifest.dependencies.clone();
    for dep in &args.deps {
        if !all_deps.iter().any(|d| d.starts_with(dep.split_whitespace().next().unwrap_or(dep))) {
            all_deps.push(dep.clone());
        }
    }

    // Create compiler and runner
    let compiler = ScriptCompiler::new();
    let runner = ScriptRunner::new();

    // Compile the script (uses cache internally)
    let binary_path = compiler
        .compile_script(&content, &manifest, script_path)
        .await
        .with_context(|| "Failed to compile script")?;

    // Build pipeline context
    let context = PipelineContext::new()
        .with_pipeline_name("rustline")
        .with_step_name(script_path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("script"));

    // Build script config
    let mut config = ScriptConfig::new(&binary_path)
        .with_pipeline_context(context);

    if !args.script_args.is_empty() {
        config = config.with_args(&args.script_args);
    }

    // Run the script
    let output = runner.run(config)
        .await
        .with_context(|| "Script execution failed")?;

    // Print output
    if !output.stdout.is_empty() {
        println!("{}", output.stdout);
    }

    if !output.stderr.is_empty() {
        eprintln!("{}", output.stderr);
    }

    // Check exit status
    if output.is_success() {
        Ok(())
    } else if output.is_timeout() {
        anyhow::bail!("Script timed out")
    } else {
        let exit_code = output.exit_code.unwrap_or(-1);
        anyhow::bail!("Script exited with failure (exit code: {})", exit_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_args(script: &str, deps: Vec<&str>, script_args: Vec<&str>) -> ScriptRunArgs {
        ScriptRunArgs {
            script: PathBuf::from(script),
            deps: deps.into_iter().map(String::from).collect(),
            script_args: script_args.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_script_run_args_parsing() {
        let args = test_args("script.rs", vec!["serde"], vec!["arg1", "arg2"]);

        assert_eq!(args.script, PathBuf::from("script.rs"));
        assert_eq!(args.deps, vec!["serde"]);
        assert_eq!(args.script_args, vec!["arg1", "arg2"]);
    }

    #[test]
    fn test_script_run_args_with_deps() {
        let args = test_args("build.rs", vec!["tokio", "serde"], vec![]);

        assert_eq!(args.script, PathBuf::from("build.rs"));
        assert_eq!(args.deps, vec!["tokio", "serde"]);
        assert!(args.script_args.is_empty());
    }

    #[test]
    fn test_script_run_args_no_deps() {
        let args = test_args("hello.rs", vec![], vec![]);

        assert_eq!(args.script, PathBuf::from("hello.rs"));
        assert!(args.deps.is_empty());
        assert!(args.script_args.is_empty());
    }

    #[test]
    fn test_script_run_args_with_hyphen_args() {
        let args = test_args("script.rs", vec![], vec!["--verbose", "-f"]);

        assert_eq!(args.script_args, vec!["--verbose", "-f"]);
    }
}