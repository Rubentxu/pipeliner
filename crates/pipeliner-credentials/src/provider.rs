//! Credential provider trait and error types.

use std::sync::Arc;
use thiserror::Error;

use crate::Credential;

/// Errors that can occur when providing credentials.
#[derive(Debug, Clone, Error)]
pub enum ProviderError {
    /// Credential with the given ID was not found
    #[error("Credential not found: {0}")]
    NotFound(String),

    /// Provider encountered an error
    #[error("Provider error: {0}")]
    ProviderError(String),

    /// Permission denied when accessing credential
    #[error("Permission denied for credential: {0}")]
    PermissionDenied(String),
}

/// Trait for credential providers.
///
/// Implementors of this trait can provide credentials from various
/// sources such as environment variables, files, or external secret stores.
///
/// # Example
///
/// ```
/// use pipeliner_credentials::{CredentialProvider, Credential, ProviderError};
/// use std::collections::HashMap;
/// use parking_lot::RwLock;
///
/// struct MyProvider {
///     credentials: RwLock<HashMap<String, Credential>>,
/// }
///
/// impl MyProvider {
///     fn new() -> Self {
///         Self { credentials: RwLock::new(HashMap::new()) }
///     }
/// }
///
/// impl CredentialProvider for MyProvider {
///     fn get(&self, id: &str) -> Result<Credential, ProviderError> {
///         self.credentials
///             .read()
///             .get(id)
///             .cloned()
///             .ok_or_else(|| ProviderError::NotFound(id.to_string()))
///     }
///
///     fn list(&self) -> Vec<String> {
///         self.credentials.read().keys().cloned().collect()
///     }
///
///     fn put(&self, id: &str, credential: Credential) -> Result<(), ProviderError> {
///         self.credentials.write().insert(id.to_string(), credential);
///         Ok(())
///     }
/// }
/// ```
pub trait CredentialProvider: Send + Sync {
    /// Gets a credential by its identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for the credential
    ///
    /// # Returns
    ///
    /// Returns the credential if found, or an error if not found or
    /// if the provider encounters an error.
    fn get(&self, id: &str) -> Result<Credential, ProviderError>;

    /// Lists all credential IDs available from this provider.
    fn list(&self) -> Vec<String>;

    /// Puts a credential into the provider.
    ///
    /// Some providers may not support write operations.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for the credential
    /// * `credential` - The credential to store
    fn put(&self, id: &str, credential: Credential) -> Result<(), ProviderError>;

    /// Returns the name of this provider.
    fn name(&self) -> &str {
        "unknown"
    }

    /// Checks if a credential exists.
    fn contains(&self, id: &str) -> bool {
        self.list().contains(&id.to_string())
    }
}

/// A type-erased credential provider that can be stored in collections.
pub type DynCredentialProvider = Arc<dyn CredentialProvider>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_error_not_found_display() {
        let err = ProviderError::NotFound("API_KEY".to_string());
        assert_eq!(err.to_string(), "Credential not found: API_KEY");
    }

    #[test]
    fn test_provider_error_provider_error_display() {
        let err = ProviderError::ProviderError("Connection failed".to_string());
        assert_eq!(err.to_string(), "Provider error: Connection failed");
    }

    #[test]
    fn test_provider_error_permission_denied_display() {
        let err = ProviderError::PermissionDenied("SECRET".to_string());
        assert_eq!(err.to_string(), "Permission denied for credential: SECRET");
    }

    #[test]
    fn test_provider_error_clone() {
        let err = ProviderError::NotFound("test".to_string());
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }
}
