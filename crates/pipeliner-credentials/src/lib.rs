//! # Pipeliner Credentials
//!
//! Credential management for Pipeliner pipelines.
//!
//! This crate provides credential providers for securely managing secrets
//! and sensitive data in pipeline executions.

#![warn(missing_docs)]
#![warn(unused)]
#![warn(clippy::pedantic)]

pub mod credential;
pub mod provider;
pub mod providers;
pub mod masking;

pub use credential::Credential;
pub use provider::{CredentialProvider, ProviderError};
pub use masking::SecretMasker;
pub use providers::{MemoryProvider, EnvProvider, FileProvider, ProviderChain};
