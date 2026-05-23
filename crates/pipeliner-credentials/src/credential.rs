//! Credential types for Pipeliner.

use serde::{Deserialize, Serialize};

/// A credential that can be provided by a `CredentialProvider`.
///
/// Credentials contain sensitive data that should be masked when
/// displayed or logged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Credential {
    /// The name/identifier of the credential
    pub name: String,
    /// The actual secret value
    pub value: String,
    /// Whether this credential should be treated as a secret
    pub is_secret: bool,
}

impl Credential {
    /// Creates a new credential with the given name and value.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            is_secret: true,
        }
    }

    /// Creates a new non-secret credential.
    #[must_use]
    pub fn plain(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            is_secret: false,
        }
    }

    /// Returns the credential value, masked if it's a secret.
    #[must_use]
    pub fn masked_value(&self) -> String {
        if self.is_secret {
            "***".to_string()
        } else {
            self.value.clone()
        }
    }
}

impl std::fmt::Display for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.masked_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_new_is_secret() {
        let cred = Credential::new("API_KEY", "secret123");
        assert_eq!(cred.name, "API_KEY");
        assert_eq!(cred.value, "secret123");
        assert!(cred.is_secret);
    }

    #[test]
    fn test_credential_plain_not_secret() {
        let cred = Credential::plain("USERNAME", "admin");
        assert_eq!(cred.name, "USERNAME");
        assert_eq!(cred.value, "admin");
        assert!(!cred.is_secret);
    }

    #[test]
    fn test_credential_masked_value_secret() {
        let cred = Credential::new("API_KEY", "secret123");
        assert_eq!(cred.masked_value(), "***");
    }

    #[test]
    fn test_credential_masked_value_plain() {
        let cred = Credential::plain("USERNAME", "admin");
        assert_eq!(cred.masked_value(), "admin");
    }

    #[test]
    fn test_credential_display_secret() {
        let cred = Credential::new("API_KEY", "secret123");
        assert_eq!(cred.to_string(), "API_KEY: ***");
    }

    #[test]
    fn test_credential_display_plain() {
        let cred = Credential::plain("USERNAME", "admin");
        assert_eq!(cred.to_string(), "USERNAME: admin");
    }

    #[test]
    fn test_credential_clone_equality() {
        let cred1 = Credential::new("API_KEY", "secret123");
        let cred2 = cred1.clone();
        assert_eq!(cred1, cred2);
    }

    #[test]
    fn test_credential_serde_roundtrip() {
        let cred = Credential::new("API_KEY", "secret123");
        let json = serde_json::to_string(&cred).unwrap();
        let parsed: Credential = serde_json::from_str(&json).unwrap();
        assert_eq!(cred, parsed);
    }
}
