//! # Step Factory Module
//!
//! Implements `ScriptStepFactory` for registering Rust script execution
//! as a pipeline step type.
//!
//! ## Usage
//!
//! ```ignore
//! use pipeliner_script::ScriptStepFactory;
//! use pipeliner_core::registry::StepRegistry;
//! use std::sync::Arc;
//!
//! let mut registry = StepRegistry::new();
//! registry.register(Arc::new(ScriptStepFactory::new()));
//! ```
//!
//! ## DSL Integration
//!
//! In the pipeline DSL, scripts are referenced by path:
//!
//! ```ignore
//! pipeline! {
//!     stage!("Build", steps![
//!         script!("scripts/build.rs", deps=["serde", "tokio"]),
//!     ]);
//! }
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;
use tokio::task::block_in_place;

use crate::cache::ScriptCache;
use crate::compiler::ScriptCompiler;
use crate::manifest::Manifest;
use crate::runner::{PipelineContext, ScriptConfig, ScriptRunner};

/// Arguments for script step creation.
#[derive(Debug, Clone)]
pub struct ScriptStepArgs {
    /// Path to the script file
    pub path: String,
    /// Inline dependencies to add
    pub deps: Vec<String>,
    /// Working directory
    pub workdir: Option<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
    /// Command-line arguments to pass to script
    pub args: Vec<String>,
}

impl ScriptStepArgs {
    /// Parses script arguments from JSON values.
    ///
    /// Expected formats:
    /// - Single string: "script.rs" or "script.rs --arg1 --arg2"
    /// - Array: ["script.rs", "--arg1"] or ["script.rs", "--arg1", "--arg2"]
    /// - Object: { "path": "script.rs", "deps": [...], "workdir": "...", "args": [...] }
    pub fn from_json(args: &[JsonValue]) -> Result<Self, StepError> {
        if args.is_empty() {
            return Err(StepError::InvalidArgs {
                message: "Script step requires at least one argument (script path)".to_string(),
            });
        }

        let first = &args[0];

        // Case 1: Object with path
        if let Some(obj) = first.as_object() {
            let path = obj
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| StepError::InvalidArgs {
                    message: "Script step object requires 'path' field".to_string(),
                })?
                .to_string();

            let deps = obj
                .get("deps")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let workdir = obj
                .get("workdir")
                .and_then(|v| v.as_str())
                .map(String::from);

            let env = obj
                .get("env")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| {
                            v.as_str().map(|s| (k.clone(), s.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default();

            let args = obj
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            return Ok(Self {
                path,
                deps,
                workdir,
                env,
                args,
            });
        }

        // Case 2: String path with optional inline args
        let path: String;
        let mut inline_args: Vec<String>;

        if let Some(s) = first.as_str() {
            // Check for space-separated args in the string
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.len() > 1 {
                path = parts[0].to_string();
                inline_args = parts[1..].iter().map(|p| p.to_string()).collect();
            } else {
                path = s.to_string();
                inline_args = Vec::new();
            }
        } else {
            return Err(StepError::InvalidArgs {
                message: "Script path must be a string or object".to_string(),
            });
        };

        // Collect remaining args (from array, skip first which is path)
        let remaining_args: Vec<String> = args.iter()
            .skip(1)
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        // Parse deps, workdir, env from remaining args AND inline_args
        // We combine them and process to extract special flags
        let mut all_args = inline_args.clone();
        all_args.extend(remaining_args);

        let mut deps: Vec<String> = Vec::new();
        let mut workdir: Option<String> = None;
        let mut env: Vec<(String, String)> = Vec::new();
        let mut script_args: Vec<String> = Vec::new();

        let mut i = 0;
        while i < all_args.len() {
            let arg = &all_args[i];
            if arg == "-d" {
                // -d as separate element, next element is the dep
                if i + 1 < all_args.len() {
                    deps.push(all_args[i + 1].clone());
                    i += 2;
                    continue;
                }
            } else if arg.starts_with("-d") {
                // -ddep or -d dep
                let dep = arg.trim_start_matches("-d").trim();
                if !dep.is_empty() {
                    deps.push(dep.to_string());
                }
                i += 1;
                continue;
            } else if arg.starts_with("--workdir=") {
                workdir = Some(arg.trim_start_matches("--workdir=").to_string());
                i += 1;
                continue;
            } else if arg == "--workdir" {
                if i + 1 < all_args.len() {
                    workdir = Some(all_args[i + 1].clone());
                    i += 2;
                    continue;
                }
            } else if arg.starts_with("--env=") {
                let kv = arg.trim_start_matches("--env=");
                if let Some((k, v)) = kv.split_once('=') {
                    env.push((k.to_string(), v.to_string()));
                }
                i += 1;
                continue;
            } else if arg == "--env" {
                if i + 1 < all_args.len() {
                    let kv = &all_args[i + 1];
                    if let Some((k, v)) = kv.split_once('=') {
                        env.push((k.to_string(), v.to_string()));
                    }
                    i += 2;
                    continue;
                }
            } else {
                script_args.push(arg.clone());
                i += 1;
                continue;
            }
            i += 1;
        }

        Ok(Self {
            path,
            deps,
            workdir,
            env,
            args: script_args,
        })
    }
}

/// Factory for creating script execution steps.
#[derive(Debug, Clone)]
pub struct ScriptStepFactory {
    /// Script cache
    cache: Arc<ScriptCache>,
    /// Script compiler
    compiler: Arc<ScriptCompiler>,
    /// Script runner
    runner: Arc<ScriptRunner>,
}

impl ScriptStepFactory {
    /// Creates a new script step factory.
    #[must_use]
    pub fn new() -> Self {
        Self::with_cache_dir(std::env::temp_dir().join("pipeliner-script-cache"))
    }

    /// Creates a factory with a custom cache directory.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        let cache = ScriptCache::new(cache_dir).unwrap_or_else(|e| {
            tracing::warn!("Failed to create script cache: {}, using default", e);
            ScriptCache::new(std::env::temp_dir().join("pipeliner-script-cache"))
                .expect("Default cache should work")
        });

        Self {
            cache: Arc::new(cache),
            compiler: Arc::new(ScriptCompiler::new()),
            runner: Arc::new(ScriptRunner::new()),
        }
    }

    /// Returns the step name.
    const fn step_name() -> &'static str {
        "script"
    }

    /// Compiles a script and returns the binary path.
    ///
    /// Uses cache if available, otherwise compiles.
    async fn get_or_compile_script(
        &self,
        script_path: &Path,
        deps: &[String],
    ) -> Result<PathBuf, StepError> {
        // Read script content
        let content = std::fs::read_to_string(script_path)
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to read script '{}': {}", script_path.display(), e),
            })?;

        // Parse manifest
        let manifest = Manifest::parse(&content).map_err(|e| StepError::CreationFailed {
            message: format!("Failed to parse manifest: {}", e),
        })?;

        // Merge inline deps with manifest deps
        let mut all_deps = manifest.dependencies.clone();
        for dep in deps {
            if !all_deps.iter().any(|d| d.starts_with(dep.split_whitespace().next().unwrap_or(dep))) {
                all_deps.push(dep.clone());
            }
        }

        // Compute cache hash
        let hash = ScriptCache::compute_hash(&content, &all_deps, script_path);

        // Check cache
        if let Some(cached_binary) = self.cache.get(&hash) {
            tracing::debug!("Cache hit for script '{}'", script_path.display());
            return Ok(cached_binary);
        }

        // Compile
        tracing::debug!("Compiling script '{}'", script_path.display());
        let binary_path = self
            .compiler
            .compile_script(&content, &manifest, script_path)
            .await
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to compile script: {}", e),
            })?;

        // Store in cache
        self.cache
            .store(hash, &content, &binary_path, &all_deps)
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to store in cache: {}", e),
            })?;

        Ok(binary_path)
    }

    /// Runs a compiled script with the given context.
    async fn run_script(
        &self,
        binary_path: &Path,
        workdir: Option<&str>,
        env: &[(String, String)],
        args: &[String],
        context: PipelineContext,
    ) -> Result<CustomStep, StepError> {
        let config = ScriptConfig::new(binary_path)
            .with_workdir(workdir.unwrap_or("."))
            .with_args(args)
            .with_pipeline_context(context);

        let output = self
            .runner
            .run(config)
            .await
            .map_err(|e| StepError::CreationFailed {
                message: format!("Script execution failed: {}", e),
            })?;

        if output.is_success() {
            Ok(CustomStep::success(
                Self::step_name(),
                Some(output.stdout),
            ))
        } else if output.is_timeout() {
            Err(StepError::CreationFailed {
                message: "Script execution timed out".to_string(),
            })
        } else {
            Err(StepError::CreationFailed {
                message: format!(
                    "Script failed with exit code {:?}: {}",
                    output.exit_code, output.stderr
                ),
            })
        }
    }
}

impl Default for ScriptStepFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StepFactory for ScriptStepFactory {
    fn name(&self) -> &str {
        Self::step_name()
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // Parse arguments
        let step_args = ScriptStepArgs::from_json(args)?;

        // Get absolute path
        let script_path = if Path::new(&step_args.path).is_absolute() {
            PathBuf::from(&step_args.path)
        } else {
            // Resolve relative to current dir
            std::env::current_dir()
                .map_err(|e| StepError::CreationFailed {
                    message: format!("Failed to get current directory: {}", e),
                })?
                .join(&step_args.path)
        };

        // Synchronously check if script exists
        if !script_path.exists() {
            return Err(StepError::CreationFailed {
                message: format!("Script not found: {}", script_path.display()),
            });
        }

        // For now, we need to block on compilation since StepFactory::create is sync
        // In a real implementation, you'd want to compile async and return a future step
        // But that requires changing the StepFactory trait
        //
        // We use blocking to run the async compilation
        let cache = self.cache.clone();
        let compiler = self.compiler.clone();
        let path_str = script_path.to_string_lossy().to_string();
        let deps = step_args.deps.clone();

        let binary_path = block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let content = std::fs::read_to_string(&script_path)
                    .map_err(|e| StepError::CreationFailed {
                        message: format!("Failed to read script: {}", e),
                    })?;

                let manifest = Manifest::parse(&content).map_err(|e| StepError::CreationFailed {
                    message: format!("Failed to parse manifest: {}", e),
                })?;

                let mut all_deps = manifest.dependencies.clone();
                for dep in &deps {
                    if !all_deps.iter().any(|d| d.starts_with(dep.split_whitespace().next().unwrap_or(dep))) {
                        all_deps.push(dep.clone());
                    }
                }

                let hash = ScriptCache::compute_hash(&content, &all_deps, &script_path);

                if let Some(cached) = cache.get(&hash) {
                    return Ok::<_, StepError>(cached);
                }

                let binary_path = compiler
                    .compile_script(&content, &manifest, &script_path)
                    .await
                    .map_err(|e| StepError::CreationFailed {
                        message: format!("Failed to compile: {}", e),
                    })?;

                cache.store(hash, &content, &binary_path, &all_deps)
                    .map_err(|e| StepError::CreationFailed {
                        message: format!("Failed to cache: {}", e),
                    })?;

                Ok(binary_path)
            })
        }).map_err(|_| StepError::CreationFailed {
            message: "Compilation task panicked".to_string(),
        })?;

        // Build pipeline context
        let context = PipelineContext::new()
            .with_step_name(Self::step_name());

        // Run the script
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            self.run_script(
                &binary_path,
                step_args.workdir.as_deref(),
                &step_args.env,
                &step_args.args,
                context,
            )
            .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_step_args_from_json_string() {
        let args = vec![JsonValue::String("script.rs".to_string())];
        let parsed = ScriptStepArgs::from_json(&args).unwrap();

        assert_eq!(parsed.path, "script.rs");
        assert!(parsed.deps.is_empty());
        assert!(parsed.workdir.is_none());
    }

    #[test]
    fn test_script_step_args_from_json_string_with_args() {
        let args = vec![JsonValue::String("script.rs --verbose --flag".to_string())];
        let parsed = ScriptStepArgs::from_json(&args).unwrap();

        assert_eq!(parsed.path, "script.rs");
        assert_eq!(parsed.args, vec!["--verbose", "--flag"]);
    }

    #[test]
    fn test_script_step_args_from_json_array() {
        let args = vec![
            JsonValue::String("script.rs".to_string()),
            JsonValue::String("--verbose".to_string()),
            JsonValue::String("--flag".to_string()),
        ];
        let parsed = ScriptStepArgs::from_json(&args).unwrap();

        assert_eq!(parsed.path, "script.rs");
        assert_eq!(parsed.args, vec!["--verbose", "--flag"]);
    }

    #[test]
    fn test_script_step_args_from_json_object() {
        let args = vec![serde_json::json!({
            "path": "build.rs",
            "deps": ["serde", "tokio"],
            "workdir": "/workspace",
            "args": ["--release"]
        })];
        let parsed = ScriptStepArgs::from_json(&args).unwrap();

        assert_eq!(parsed.path, "build.rs");
        assert_eq!(parsed.deps, vec!["serde", "tokio"]);
        assert_eq!(parsed.workdir, Some("/workspace".to_string()));
        assert_eq!(parsed.args, vec!["--release"]);
    }

    #[test]
    fn test_script_step_args_from_json_empty() {
        let args: Vec<JsonValue> = vec![];
        let result = ScriptStepArgs::from_json(&args);

        assert!(result.is_err());
    }

    #[test]
    fn test_script_step_factory_name() {
        let factory = ScriptStepFactory::new();
        assert_eq!(factory.name(), "script");
    }

    #[test]
    fn test_script_step_factory_default() {
        let factory = ScriptStepFactory::default();
        assert_eq!(factory.name(), "script");
    }

    #[tokio::test]
    async fn test_script_step_factory_with_cache_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let factory = ScriptStepFactory::with_cache_dir(temp_dir.path().to_path_buf());
        assert_eq!(factory.name(), "script");
    }

    #[test]
    fn test_script_step_args_with_deps_flag() {
        let args = vec![
            JsonValue::String("script.rs".to_string()),
            JsonValue::String("-d serde".to_string()),
            JsonValue::String("-d tokio".to_string()),
        ];
        let parsed = ScriptStepArgs::from_json(&args).unwrap();

        assert_eq!(parsed.path, "script.rs");
        assert_eq!(parsed.deps, vec!["serde", "tokio"]);
    }

    #[test]
    fn test_script_step_args_with_workdir() {
        let args = vec![
            JsonValue::String("script.rs".to_string()),
            JsonValue::String("--workdir=/workspace".to_string()),
        ];
        let parsed = ScriptStepArgs::from_json(&args).unwrap();

        assert_eq!(parsed.workdir, Some("/workspace".to_string()));
    }

    #[test]
    fn test_script_step_args_with_env() {
        let args = vec![
            JsonValue::String("script.rs".to_string()),
            JsonValue::String("--env=RUST_LOG=debug".to_string()),
        ];
        let parsed = ScriptStepArgs::from_json(&args).unwrap();

        assert_eq!(parsed.env, vec![("RUST_LOG".to_string(), "debug".to_string())]);
    }
}