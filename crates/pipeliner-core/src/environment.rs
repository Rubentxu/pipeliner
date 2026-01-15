//! Environment variable handling for pipelines.
//!
//! This module provides types for managing environment variables
//! in pipelines, including simple values, secrets, and resolution strategies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Environment variable collection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Environment(pub HashMap<String, EnvVarValue>);

impl Environment {
    /// Creates a new empty environment
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Creates an environment from a hash map
    #[must_use]
    pub fn from_map(map: HashMap<String, String>) -> Self {
        Self(
            map.into_iter()
                .map(|(k, v)| (k, EnvVarValue::Value(v)))
                .collect(),
        )
    }

    /// Adds a simple value
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.insert(key.into(), EnvVarValue::Value(value.into()));
    }

    /// Adds a secret value
    pub fn insert_secret(&mut self, key: impl Into<String>, secret: impl Into<String>) {
        self.0.insert(
            key.into(),
            EnvVarValue::Secret(SecretValue {
                value: secret.into(),
                masked: true,
            }),
        );
    }

    /// Adds a credentials reference
    pub fn insert_credentials(
        &mut self,
        key: impl Into<String>,
        credentials_id: impl Into<String>,
        key_field: impl Into<String>,
    ) {
        self.0.insert(
            key.into(),
            EnvVarValue::Credentials(CredentialsValue {
                credentials_id: credentials_id.into(),
                key_field: key_field.into(),
            }),
        );
    }

    /// Gets a value by key
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&EnvVarValue> {
        self.0.get(key)
    }

    /// Returns an iterator over all environment variables
    pub fn iter(&self) -> impl Iterator<Item = (&str, &EnvVarValue)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Returns the number of variables
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the environment is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (key, value) in &self.0 {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{}: {}", key, value)?;
        }
        write!(f, "}}")
    }
}

/// Environment variable value types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EnvVarValue {
    /// Simple string value
    Value(String),

    /// Secret value (masked in logs)
    Secret(SecretValue),

    /// Credentials reference
    Credentials(CredentialsValue),

    /// Expression to evaluate
    Expression(ExpressionValue),
}

/// Secret value configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretValue {
    /// The secret value
    pub value: String,
    /// Whether to mask the value in logs
    #[serde(default = "default_masked")]
    pub masked: bool,
}

fn default_masked() -> bool {
    true
}

/// Credentials reference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsValue {
    /// Credentials ID to look up
    pub credentials_id: String,
    /// Field to extract from credentials
    pub key_field: String,
}

/// Expression value for dynamic resolution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionValue {
    /// The expression to evaluate
    pub expression: String,
}

/// Variable resolver trait for interpolating environment variables
pub trait VariableResolver {
    /// Resolves a variable value
    fn resolve(&self, name: &str) -> Option<String>;
}

impl VariableResolver for Environment {
    fn resolve(&self, name: &str) -> Option<String> {
        self.get(name).map(|v| v.to_string())
    }
}

impl VariableResolver for HashMap<String, String> {
    fn resolve(&self, name: &str) -> Option<String> {
        self.get(name).cloned()
    }
}

impl fmt::Display for EnvVarValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvVarValue::Value(v) => write!(f, "{}", v),
            EnvVarValue::Secret(s) => {
                if s.masked {
                    write!(f, "***")
                } else {
                    write!(f, "{}", s.value)
                }
            }
            EnvVarValue::Credentials(c) => write!(
                f,
                "${{credentialsId='{}', field='{}'}}",
                c.credentials_id, c.key_field
            ),
            EnvVarValue::Expression(e) => write!(f, "${{{}}}", e.expression),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_creation() {
        let env = Environment::new();
        assert!(env.is_empty());
    }

    #[test]
    fn test_environment_insert() {
        let mut env = Environment::new();
        env.insert("FOO", "bar");
        assert_eq!(env.get("FOO"), Some(&EnvVarValue::Value("bar".to_string())));
    }

    #[test]
    fn test_environment_from_map() {
        let env = Environment::from_map(HashMap::from([
            ("FOO".to_string(), "bar".to_string()),
            ("BAZ".to_string(), "qux".to_string()),
        ]));
        assert_eq!(env.get("FOO"), Some(&EnvVarValue::Value("bar".to_string())));
        assert_eq!(env.get("BAZ"), Some(&EnvVarValue::Value("qux".to_string())));
    }

    #[test]
    fn test_environment_secret() {
        let mut env = Environment::new();
        env.insert_secret("SECRET", "my-secret-value");
        if let EnvVarValue::Secret(s) = env.get("SECRET").unwrap() {
            assert_eq!(s.value, "my-secret-value");
            assert!(s.masked);
        } else {
            panic!("Expected Secret value");
        }
    }

    #[test]
    fn test_environment_credentials() {
        let mut env = Environment::new();
        env.insert_credentials("API_TOKEN", "github-token", "password");
        if let EnvVarValue::Credentials(c) = env.get("API_TOKEN").unwrap() {
            assert_eq!(c.credentials_id, "github-token");
            assert_eq!(c.key_field, "password");
        } else {
            panic!("Expected Credentials value");
        }
    }

    #[test]
    fn test_environment_display() {
        let mut env = Environment::new();
        env.insert("FOO", "bar");
        assert!(format!("{}", env).contains("FOO"));
    }
}
