//! Built-in credential providers.

pub mod env;
pub mod file;
pub mod memory;

pub use env::EnvProvider;
pub use file::FileProvider;
pub use memory::MemoryProvider;

use std::sync::Arc;

use crate::{Credential, CredentialProvider, ProviderError};

/// A provider chain that tries multiple providers in sequence.
///
/// The chain tries each provider in order until one returns the
/// requested credential.
#[derive(Clone)]
pub struct ProviderChain {
    providers: Vec<Arc<dyn CredentialProvider>>,
}

impl std::fmt::Debug for ProviderChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderChain")
            .field("providers", &self.providers.len())
            .finish()
    }
}

impl ProviderChain {
    /// Creates a new empty provider chain.
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Creates a new provider chain with the given providers.
    #[must_use]
    pub fn with_providers(providers: Vec<Arc<dyn CredentialProvider>>) -> Self {
        Self { providers }
    }

    /// Adds a provider to the end of the chain.
    pub fn add_provider(mut self, provider: Arc<dyn CredentialProvider>) -> Self {
        self.providers.push(provider);
        self
    }
}

impl Default for ProviderChain {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialProvider for ProviderChain {
    fn get(&self, id: &str) -> Result<Credential, ProviderError> {
        for provider in &self.providers {
            if provider.contains(id) {
                return provider.get(id);
            }
        }
        Err(ProviderError::NotFound(id.to_string()))
    }

    fn list(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for provider in &self.providers {
            ids.extend(provider.list());
        }
        ids.sort();
        ids.dedup();
        ids
    }

    fn put(&self, id: &str, credential: Credential) -> Result<(), ProviderError> {
        // Try to put to each provider, return first success or last error
        let mut last_error = ProviderError::ProviderError("No providers in chain".to_string());
        for provider in &self.providers {
            if provider.contains(id) || self.list().contains(&id.to_string()) {
                match provider.put(id, credential.clone()) {
                    Ok(()) => return Ok(()),
                    Err(e) => last_error = e,
                }
            }
        }
        Err(last_error)
    }

    fn name(&self) -> &str {
        "chain"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_chain_empty() {
        let chain = ProviderChain::new();
        let result = chain.get("test");
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_chain_with_memory_provider() {
        let memory = MemoryProvider::default();
        memory.put("TEST", Credential::new("TEST", "value")).unwrap();

        let chain = ProviderChain::new()
            .add_provider(Arc::new(memory));

        let result = chain.get("TEST").unwrap();
        assert_eq!(result.value, "value");
    }

    #[test]
    fn test_provider_chain_not_found() {
        let memory = MemoryProvider::default();
        let chain = ProviderChain::new()
            .add_provider(Arc::new(memory));

        let result = chain.get("NONEXISTENT");
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_chain_list() {
        let memory = MemoryProvider::default();
        memory.put("A", Credential::new("A", "1")).unwrap();
        memory.put("B", Credential::new("B", "2")).unwrap();

        let chain = ProviderChain::new()
            .add_provider(Arc::new(memory));

        let ids = chain.list();
        assert!(ids.contains(&"A".to_string()));
        assert!(ids.contains(&"B".to_string()));
    }
}
