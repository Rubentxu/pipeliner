//! File-based credential provider.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::{Credential, CredentialProvider, ProviderError};

/// A credential provider that reads from a file.
///
/// Supports `.env` files and JSON credential files.
#[derive(Debug, Clone)]
pub struct FileProvider {
    /// Path to the credentials file
    path: String,
    /// Cache of loaded credentials
    cache: Arc<parking_lot::RwLock<Option<HashMap<String, Credential>>>>,
}

impl FileProvider {
    /// Creates a new file provider for the given path.
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            cache: Arc::new(parking_lot::RwLock::new(None)),
        }
    }

    /// Creates a new file provider for the default credentials location.
    ///
    /// Looks for `~/.pipeliner/credentials.toml` or `~/.pipeliner/credentials.env`.
    #[must_use]
    pub fn default_location() -> Option<Self> {
        if let Some(home) = dirs::home_dir() {
            let creds_path = home.join(".pipeliner").join("credentials");
            // Try both .toml and .env extensions
            if creds_path.with_extension("toml").exists() {
                return Some(Self::new(creds_path.with_extension("toml").to_string_lossy().to_string()));
            }
            if creds_path.with_extension("env").exists() {
                return Some(Self::new(creds_path.with_extension("env").to_string_lossy().to_string()));
            }
        }
        None
    }

    /// Loads and parses the credentials file.
    fn load_credentials(&self) -> Result<HashMap<String, Credential>, ProviderError> {
        // Check cache first
        if let Some(cached) = self.cache.read().as_ref() {
            return Ok(cached.clone());
        }

        let path = Path::new(&self.path);
        if !path.exists() {
            return Err(ProviderError::NotFound(format!(
                "Credentials file not found: {}",
                self.path
            )));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| ProviderError::ProviderError(format!("Failed to read file: {}", e)))?;

        let credentials = if self.path.ends_with(".env") {
            self.parse_env_file(&content)?
        } else if self.path.ends_with(".toml") {
            self.parse_toml_file(&content)?
        } else {
            // Try to detect format
            if content.contains('=') {
                self.parse_env_file(&content)?
            } else {
                self.parse_toml_file(&content)?
            }
        };

        // Update cache
        *self.cache.write() = Some(credentials.clone());

        Ok(credentials)
    }

    /// Parses a .env file format.
    fn parse_env_file(&self, content: &str) -> Result<HashMap<String, Credential>, ProviderError> {
        let mut credentials = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                credentials.insert(
                    key.to_string(),
                    Credential::new(key, value),
                );
            }
        }

        Ok(credentials)
    }

    /// Parses a TOML file format.
    fn parse_toml_file(&self, content: &str) -> Result<HashMap<String, Credential>, ProviderError> {
        let mut credentials = HashMap::new();

        // Simple TOML parsing for [credentials] section
        let mut in_credentials_section = false;

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with('[') && line.ends_with(']') {
                in_credentials_section = line == "[credentials]";
                continue;
            }

            if in_credentials_section {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim().trim_matches('"').trim_matches('\'');
                    if !value.is_empty() {
                        credentials.insert(
                            key.to_string(),
                            Credential::new(key, value),
                        );
                    }
                }
            }
        }

        Ok(credentials)
    }

    /// Invalidates the cache, forcing a reload on next access.
    pub fn invalidate_cache(&self) {
        *self.cache.write() = None;
    }
}

impl CredentialProvider for FileProvider {
    fn get(&self, id: &str) -> Result<Credential, ProviderError> {
        let credentials = self.load_credentials()?;
        credentials
            .get(id)
            .cloned()
            .ok_or_else(|| ProviderError::NotFound(id.to_string()))
    }

    fn list(&self) -> Vec<String> {
        self.load_credentials()
            .map(|c| c.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn put(&self, _id: &str, _credential: Credential) -> Result<(), ProviderError> {
        Err(ProviderError::ProviderError(
            "FileProvider does not support write operations".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_env_file() {
        let provider = FileProvider::new("/nonexistent.env");
        let content = r#"
# Comment
API_KEY=secret123
USERNAME=admin
"#;
        let credentials = provider.parse_env_file(content).unwrap();

        assert_eq!(credentials.len(), 2);
        assert_eq!(credentials.get("API_KEY").unwrap().value, "secret123");
        assert_eq!(credentials.get("USERNAME").unwrap().value, "admin");
    }

    #[test]
    fn test_parse_env_file_with_quotes() {
        let provider = FileProvider::new("/nonexistent.env");
        let content = r#"
API_KEY="quoted_value"
USERNAME='single_quoted'
"#;
        let credentials = provider.parse_env_file(content).unwrap();

        assert_eq!(credentials.get("API_KEY").unwrap().value, "quoted_value");
        assert_eq!(credentials.get("USERNAME").unwrap().value, "single_quoted");
    }

    #[test]
    fn test_parse_toml_file() {
        let provider = FileProvider::new("/nonexistent.toml");
        let content = r#"
[credentials]
api_key = "secret456"
username = "admin"
"#;
        let credentials = provider.parse_toml_file(content).unwrap();

        assert_eq!(credentials.len(), 2);
        assert_eq!(credentials.get("api_key").unwrap().value, "secret456");
        assert_eq!(credentials.get("username").unwrap().value, "admin");
    }

    #[test]
    fn test_file_provider_not_found() {
        let provider = FileProvider::new("/truly/nonexistent/path/12345.env");
        let result = provider.get("ANYTHING");
        assert!(result.is_err());
    }

    #[test]
    fn test_file_provider_loads_env_file() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join("credentials.env");
        std::fs::write(&env_path, "TEST_KEY=test_value\n").unwrap();

        let provider = FileProvider::new(env_path.to_string_lossy().to_string());
        let result = provider.get("TEST_KEY").unwrap();

        assert_eq!(result.value, "test_value");
    }

    #[test]
    fn test_file_provider_list() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join("credentials.env");
        std::fs::write(&env_path, "KEY1=value1\nKEY2=value2\n").unwrap();

        let provider = FileProvider::new(env_path.to_string_lossy().to_string());
        let ids = provider.list();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"KEY1".to_string()));
        assert!(ids.contains(&"KEY2".to_string()));
    }

    #[test]
    fn test_file_provider_cache_invalidation() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join("credentials.env");
        std::fs::write(&env_path, "KEY=value\n").unwrap();

        let provider = FileProvider::new(env_path.to_string_lossy().to_string());

        // First access loads
        assert!(provider.get("KEY").is_ok());

        // Modify the file
        std::fs::write(&env_path, "KEY=new_value\n").unwrap();

        // Without invalidation, should still get cached value
        // (In real use, you'd want to invalidate)
        // For testing, we can directly check the cache
        provider.invalidate_cache();

        // After invalidation, should see new value
        let result = provider.get("KEY").unwrap();
        assert_eq!(result.value, "new_value");
    }
}
