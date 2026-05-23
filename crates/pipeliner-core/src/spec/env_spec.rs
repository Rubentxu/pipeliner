//! Environment specification types for declarative pipeline definitions.
//!
//! This module defines the environment variable specification structure
//! used to pass environment variables to stages and steps.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Specification for environment variables.
///
/// This struct holds a collection of environment variable key-value pairs
/// that can be applied to stages or steps.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EnvSpec {
    /// Environment variables as key-value pairs
    vars: HashMap<String, String>,
}

impl EnvSpec {
    /// Creates a new empty environment specification.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a variable to the environment specification.
    ///
    /// # Arguments
    ///
    /// * `name` - The variable name
    /// * `value` - The variable value
    ///
    /// # Example
    ///
    /// ```
    /// use pipeliner_core::spec::EnvSpec;
    ///
    /// let env = EnvSpec::new()
    ///     .with_var("RUST_BACKTRACE", "1")
    ///     .with_var("LOG_LEVEL", "debug");
    /// ```
    #[must_use]
    pub fn with_var(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(name.into(), value.into());
        self
    }

    /// Gets a variable value by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The variable name to look up
    ///
    /// # Returns
    ///
    /// The value if found, None otherwise.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }

    /// Returns an iterator over the environment variables.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Returns the number of environment variables.
    #[must_use]
    pub fn len(&self) -> usize {
        self.vars.len()
    }

    /// Returns true if there are no environment variables.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_spec_creation() {
        let env = EnvSpec::new();
        assert!(env.is_empty());
    }

    #[test]
    fn test_env_spec_with_var() {
        let env = EnvSpec::new()
            .with_var("RUST_BACKTRACE", "1")
            .with_var("LOG_LEVEL", "debug");

        assert_eq!(env.len(), 2);
        assert_eq!(env.get("RUST_BACKTRACE"), Some("1"));
        assert_eq!(env.get("LOG_LEVEL"), Some("debug"));
    }

    #[test]
    fn test_env_spec_iter() {
        let env = EnvSpec::new()
            .with_var("KEY1", "value1")
            .with_var("KEY2", "value2");

        let vars: Vec<_> = env.iter().collect();
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn test_env_spec_overwrite() {
        let env = EnvSpec::new()
            .with_var("KEY", "value1")
            .with_var("KEY", "value2");

        assert_eq!(env.get("KEY"), Some("value2"));
        assert_eq!(env.len(), 1);
    }

    #[test]
    fn test_env_spec_serialization() {
        let env = EnvSpec::new()
            .with_var("FOO", "bar")
            .with_var("BAZ", "qux");

        let json = serde_json::to_string(&env).unwrap();
        let parsed: EnvSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.get("FOO"), Some("bar"));
        assert_eq!(parsed.get("BAZ"), Some("qux"));
    }

    #[test]
    fn test_env_spec_json_roundtrip() {
        let original = EnvSpec::new()
            .with_var("TEST", "value")
            .with_var("NUM", "42");

        let json = serde_json::to_string(&original).unwrap();
        let parsed: EnvSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.len(), original.len());
        assert_eq!(parsed.get("TEST"), original.get("TEST"));
        assert_eq!(parsed.get("NUM"), original.get("NUM"));
    }
}
