//! In-memory credential provider.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::{Credential, CredentialProvider, ProviderError};

/// An in-memory credential provider.
///
/// This provider stores credentials in a HashMap and is useful for
/// testing or for temporary credentials during pipeline execution.
#[derive(Debug, Clone)]
pub struct MemoryProvider {
    store: Arc<RwLock<HashMap<String, Credential>>>,
}

impl Default for MemoryProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryProvider {
    /// Creates a new empty in-memory provider.
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new in-memory provider with the given credentials.
    #[must_use]
    pub fn with_credentials(credentials: HashMap<String, Credential>) -> Self {
        Self {
            store: Arc::new(RwLock::new(credentials)),
        }
    }

    /// Creates a new independent in-memory provider (not sharing state with clone).
    ///
    /// Unlike `new()` which creates a provider that shares state with its clones,
    /// this creates a completely independent provider.
    #[must_use]
    pub fn independent() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Clears all credentials from the provider.
    pub fn clear(&self) {
        self.store.write().clear();
    }

    /// Returns the number of credentials stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.store.read().len()
    }

    /// Returns true if the provider is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.store.read().is_empty()
    }
}

impl CredentialProvider for MemoryProvider {
    fn get(&self, id: &str) -> Result<Credential, ProviderError> {
        self.store
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| ProviderError::NotFound(id.to_string()))
    }

    fn list(&self) -> Vec<String> {
        self.store.read().keys().cloned().collect()
    }

    fn put(&self, id: &str, credential: Credential) -> Result<(), ProviderError> {
        self.store.write().insert(id.to_string(), credential);
        Ok(())
    }

    fn name(&self) -> &str {
        "memory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_provider_default() {
        let provider = MemoryProvider::default();
        assert!(provider.is_empty());
        assert_eq!(provider.len(), 0);
    }

    #[test]
    fn test_memory_provider_put_and_get() {
        let provider = MemoryProvider::default();
        let cred = Credential::new("API_KEY", "secret123");

        provider.put("api", cred.clone()).unwrap();
        assert!(!provider.is_empty());
        assert_eq!(provider.len(), 1);

        let retrieved = provider.get("api").unwrap();
        assert_eq!(retrieved.name, "API_KEY");
        assert_eq!(retrieved.value, "secret123");
    }

    #[test]
    fn test_memory_provider_get_not_found() {
        let provider = MemoryProvider::default();
        let result = provider.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_provider_list() {
        let provider = MemoryProvider::default();
        provider.put("a", Credential::new("A", "1")).unwrap();
        provider.put("b", Credential::new("B", "2")).unwrap();

        let ids = provider.list();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"a".to_string()));
        assert!(ids.contains(&"b".to_string()));
    }

    #[test]
    fn test_memory_provider_clear() {
        let provider = MemoryProvider::default();
        provider.put("a", Credential::new("A", "1")).unwrap();
        assert!(!provider.is_empty());

        provider.clear();
        assert!(provider.is_empty());
        assert_eq!(provider.len(), 0);
    }

    #[test]
    fn test_memory_provider_overwrite() {
        let provider = MemoryProvider::default();
        provider.put("key", Credential::new("KEY", "value1")).unwrap();
        provider.put("key", Credential::new("KEY", "value2")).unwrap();

        assert_eq!(provider.len(), 1);
        let retrieved = provider.get("key").unwrap();
        assert_eq!(retrieved.value, "value2");
    }

    #[test]
    fn test_memory_provider_clone_shares_state() {
        // MemoryProvider uses Arc internally so clones share the same store
        let provider = MemoryProvider::default();
        provider.put("key", Credential::new("KEY", "value")).unwrap();

        let cloned = provider.clone();
        cloned.put("key2", Credential::new("KEY2", "value2")).unwrap();

        // Original should now have two credentials (shared state)
        assert_eq!(provider.len(), 2);
        assert_eq!(cloned.len(), 2);
    }

    #[test]
    fn test_memory_provider_independent() {
        // MemoryProvider::independent() creates truly independent providers
        let provider = MemoryProvider::independent();
        provider.put("key", Credential::new("KEY", "value")).unwrap();

        let independent = MemoryProvider::independent();
        independent.put("key2", Credential::new("KEY2", "value2")).unwrap();

        // Original should still only have one credential
        assert_eq!(provider.len(), 1);
        // Independent should have one
        assert_eq!(independent.len(), 1);
    }
}
