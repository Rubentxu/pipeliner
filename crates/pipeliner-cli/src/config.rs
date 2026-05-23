//! Configuration module for Pipeliner CLI.
//!
//! Loads configuration from:
//! - `~/.pipeliner/config.toml` (global config)
//! - Environment variables (PIPELINER_* overrides)
//! - `--config` CLI flag (highest priority)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Global CLI configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// Verbose output
    #[serde(default)]
    pub verbose: bool,

    /// Output format (json, yaml, human)
    #[serde(default = "default_format")]
    pub format: OutputFormat,

    /// Disable colors
    #[serde(default)]
    pub no_color: bool,

    /// Config file path (if specified via CLI)
    #[serde(skip)]
    pub config_path: Option<PathBuf>,

    /// Default pipeline directory
    #[serde(default)]
    pub pipeline_dir: Option<PathBuf>,

    /// Default cache mode
    #[serde(default)]
    pub cache_mode: Option<String>,

    /// Default log level
    #[serde(default)]
    pub log_level: Option<String>,
}

fn default_format() -> OutputFormat {
    OutputFormat::Human
}

/// Output format for CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
    Yaml,
}

impl OutputFormat {
    /// Parse from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "yaml" | "yml" => OutputFormat::Yaml,
            _ => OutputFormat::Human,
        }
    }
}

impl Config {
    /// Load configuration from the default path `~/.pipeliner/config.toml`
    pub fn load_default() -> Result<Self> {
        Self::load_from_default_path().or_else(|_| Ok(Config::default()))
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;
        Self::parse(&content)
    }

    /// Load from the default config path
    fn load_from_default_path() -> Result<Self> {
        let config_path = Self::default_config_path()?;
        Self::load_from_path(&config_path)
    }

    /// Get the default config path `~/.pipeliner/config.toml`
    fn default_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Could not determine home directory")?;
        Ok(home.join(".pipeliner").join("config.toml"))
    }

    /// Parse configuration from TOML string
    fn parse(content: &str) -> Result<Self> {
        let mut config: Config = toml_edit::de::from_str(content)
            .context("Failed to parse config TOML")?;

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides
    ///
    /// Environment variables take precedence over config file:
    /// - `PIPELINER_VERBOSE` -> verbose
    /// - `PIPELINER_FORMAT` -> format
    /// - `PIPELINER_NO_COLOR` -> no_color
    /// - `PIPELINER_CONFIG` -> config_path (not really an override, used at load time)
    /// - `PIPELINER_PIPELINE_DIR` -> pipeline_dir
    /// - `PIPELINER_CACHE_MODE` -> cache_mode
    /// - `PIPELINER_LOG_LEVEL` -> log_level
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("PIPELINER_VERBOSE") {
            self.verbose = val.parse().unwrap_or(false);
        }

        if let Ok(val) = std::env::var("PIPELINER_FORMAT") {
            self.format = OutputFormat::parse(&val);
        }

        if let Ok(val) = std::env::var("PIPELINER_NO_COLOR") {
            self.no_color = val.parse().unwrap_or(false);
        }

        if let Ok(val) = std::env::var("PIPELINER_PIPELINE_DIR") {
            self.pipeline_dir = Some(PathBuf::from(val));
        }

        if let Ok(val) = std::env::var("PIPELINER_CACHE_MODE") {
            self.cache_mode = Some(val);
        }

        if let Ok(val) = std::env::var("PIPELINER_LOG_LEVEL") {
            self.log_level = Some(val);
        }
    }

    /// Merge with CLI args (CLI takes precedence)
    pub fn merge_with_cli(&mut self, verbose: bool, format: OutputFormat, no_color: bool, config_path: Option<PathBuf>) {
        if verbose {
            self.verbose = verbose;
        }
        self.format = format;
        self.no_color = no_color;
        if config_path.is_some() {
            self.config_path = config_path;
        }
    }

    /// Check if output should be colored
    pub fn use_color(&self) -> bool {
        !self.no_color && atty::is(atty::Stream::Stdout)
    }

    /// Get effective format (accounting for no_color)
    pub fn effective_format(&self) -> OutputFormat {
        if self.no_color && self.format == OutputFormat::Human {
            // When --no-color is set, still use human but stripped of ANSI
            OutputFormat::Human
        } else {
            self.format
        }
    }
}

/// Load configuration with priority: CLI arg > env > config file
pub fn load_config(
    config_path: Option<&PathBuf>,
    verbose: bool,
    format: &str,
    no_color: bool,
) -> Result<Config> {
    // Start with default config
    let mut config = Config::load_default().unwrap_or_default();

    // Apply environment variable overrides
    config.apply_env_overrides();

    // Load from explicit config path if provided
    if let Some(path) = config_path {
        let file_config = Config::load_from_path(path)?;
        config = file_config;
    }

    // Apply CLI arguments (highest priority)
    config.merge_with_cli(verbose, OutputFormat::parse(format), no_color, config_path.cloned());

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parse() {
        assert_eq!(OutputFormat::parse("json"), OutputFormat::Json);
        assert_eq!(OutputFormat::parse("JSON"), OutputFormat::Json);
        assert_eq!(OutputFormat::parse("yaml"), OutputFormat::Yaml);
        assert_eq!(OutputFormat::parse("yml"), OutputFormat::Yaml);
        assert_eq!(OutputFormat::parse("human"), OutputFormat::Human);
        assert_eq!(OutputFormat::parse("unknown"), OutputFormat::Human);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(!config.verbose);
        assert_eq!(config.format, OutputFormat::Human);
        assert!(!config.no_color);
        assert!(config.config_path.is_none());
    }

    #[test]
    fn test_config_merge_with_cli() {
        let mut config = Config::default();
        config.merge_with_cli(true, OutputFormat::Json, true, Some(PathBuf::from("/custom.toml")));

        assert!(config.verbose);
        assert_eq!(config.format, OutputFormat::Json);
        assert!(config.no_color);
        assert_eq!(config.config_path, Some(PathBuf::from("/custom.toml")));
    }

    #[test]
    fn test_config_parse_toml() {
        let toml = r#"
verbose = true
format = "json"
no_color = false
pipeline_dir = "/tmp/pipelines"
cache_mode = "full"
log_level = "debug"
"#;
        let config: Config = toml_edit::de::from_str(toml).unwrap();
        assert!(config.verbose);
        assert_eq!(config.format, OutputFormat::Json);
        assert!(!config.no_color);
        assert_eq!(config.pipeline_dir, Some(PathBuf::from("/tmp/pipelines")));
        assert_eq!(config.cache_mode, Some("full".to_string()));
        assert_eq!(config.log_level, Some("debug".to_string()));
    }

    #[test]
    fn test_env_overrides() {
        // SAFETY: Setting env vars in tests is safe as tests are isolated
        unsafe {
            std::env::set_var("PIPELINER_VERBOSE", "true");
            std::env::set_var("PIPELINER_FORMAT", "json");
            std::env::set_var("PIPELINER_NO_COLOR", "true");
        }

        let mut config = Config::default();
        config.apply_env_overrides();

        assert!(config.verbose);
        assert_eq!(config.format, OutputFormat::Json);
        assert!(config.no_color);

        // Clean up
        // SAFETY: Removing env vars in tests is safe as tests are isolated
        unsafe {
            std::env::remove_var("PIPELINER_VERBOSE");
            std::env::remove_var("PIPELINER_FORMAT");
            std::env::remove_var("PIPELINER_NO_COLOR");
        }
    }
}