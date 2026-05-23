//! Environment variable credential provider.

use std::env;

use crate::{Credential, CredentialProvider, ProviderError};

/// A credential provider that reads from environment variables.
///
/// This provider looks for environment variables with a specific prefix
/// and exposes them as credentials.
#[derive(Debug, Clone)]
pub struct EnvProvider {
    /// Prefix for environment variables to consider
    prefix: String,
}

impl Default for EnvProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvProvider {
    /// Creates a new environment variable provider with no prefix.
    ///
    /// This will expose all environment variables as credentials.
    #[must_use]
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
        }
    }

    /// Creates a new environment variable provider with the given prefix.
    ///
    /// Only environment variables starting with the prefix will be exposed.
    #[must_use]
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// Gets the environment variable value for the given key.
    fn get_env(&self, id: &str) -> Option<String> {
        let key = if self.prefix.is_empty() {
            id.to_string()
        } else {
            format!("{}_{}", self.prefix, id)
        };

        env::var(&key).ok()
    }

    /// Gets all environment variable keys that match this provider's prefix.
    fn matching_keys(&self) -> Vec<String> {
        env::vars()
            .filter(|(key, _)| {
                if self.prefix.is_empty() {
                    true
                } else {
                    key.starts_with(&self.prefix)
                }
            })
            .map(|(key, _)| {
                if self.prefix.is_empty() {
                    key
                } else {
                    key.strip_prefix(&format!("{}_", self.prefix))
                        .unwrap_or(&key)
                        .to_string()
                }
            })
            .collect()
    }
}

impl CredentialProvider for EnvProvider {
    fn get(&self, id: &str) -> Result<Credential, ProviderError> {
        self.get_env(id)
            .map(|value| Credential::new(id, value))
            .ok_or_else(|| ProviderError::NotFound(id.to_string()))
    }

    fn list(&self) -> Vec<String> {
        self.matching_keys()
    }

    fn put(&self, id: &str, credential: Credential) -> Result<(), ProviderError> {
        let key = if self.prefix.is_empty() {
            id.to_string()
        } else {
            format!("{}_{}", self.prefix, id)
        };

        // SAFETY: In a single-threaded CLI context, setting env vars is acceptable
        // This is consistent with how the rest of the codebase handles env vars
        unsafe {
            env::set_var(&key, credential.value);
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "env"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to safely set env var in tests
    unsafe fn set_test_var(key: &str, value: &str) {
        env::set_var(key, value);
    }

    // Helper function to safely remove env var in tests
    unsafe fn remove_test_var(key: &str) {
        env::remove_var(key);
    }

    #[test]
    fn test_env_provider_get_existing() {
        // SAFETY: Test-only environment manipulation
        unsafe {
            set_test_var("TEST_VAR", "test_value");
        }
        let provider = EnvProvider::new();

        let result = provider.get("TEST_VAR").unwrap();
        assert_eq!(result.value, "test_value");
        assert!(result.is_secret);

        unsafe { remove_test_var("TEST_VAR"); }
    }

    #[test]
    fn test_env_provider_get_not_found() {
        let provider = EnvProvider::new();
        let result = provider.get("NONEXISTENT_VAR_12345");
        assert!(result.is_err());
    }

    #[test]
    fn test_env_provider_list() {
        // SAFETY: Test-only environment manipulation
        unsafe {
            set_test_var("LIST_VAR_A", "value_a");
            set_test_var("LIST_VAR_B", "value_b");
        }

        let provider = EnvProvider::new();
        let vars = provider.list();

        assert!(vars.contains(&"LIST_VAR_A".to_string()));
        assert!(vars.contains(&"LIST_VAR_B".to_string()));

        unsafe {
            remove_test_var("LIST_VAR_A");
            remove_test_var("LIST_VAR_B");
        }
    }

    #[test]
    fn test_env_provider_with_prefix() {
        // SAFETY: Test-only environment manipulation
        unsafe {
            set_test_var("PIPELINER_API_KEY", "secret_key");
        }
        let provider = EnvProvider::with_prefix("PIPELINER");

        // With prefix, API_KEY maps to PIPELINER_API_KEY
        let result = provider.get("API_KEY").unwrap();
        assert_eq!(result.value, "secret_key");

        // List should return the stripped keys
        let keys = provider.list();
        assert!(keys.contains(&"API_KEY".to_string()));
        assert!(!keys.contains(&"PIPELINER_API_KEY".to_string()));

        unsafe { remove_test_var("PIPELINER_API_KEY"); }
    }

    #[test]
    fn test_env_provider_put() {
        let provider = EnvProvider::with_prefix("TEST");

        provider.put("NEW_VAR", Credential::new("NEW_VAR", "new_value")).unwrap();

        let value = env::var("TEST_NEW_VAR").unwrap();
        assert_eq!(value, "new_value");

        unsafe { remove_test_var("TEST_NEW_VAR"); }
    }
}
