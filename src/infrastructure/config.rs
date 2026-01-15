//! Configuration management

use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default executor
    pub default_executor: String,
    /// Cache directory
    pub cache_dir: String,
    /// Log level
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_executor: "local".to_string(),
            cache_dir: ".rustline/cache".to_string(),
            log_level: "info".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.default_executor, "local");
        assert_eq!(config.log_level, "info");
    }
}
