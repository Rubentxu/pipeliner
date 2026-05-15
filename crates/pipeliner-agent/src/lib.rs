//! Pipeliner Agent - LLM-powered step execution
//!
//! This crate provides `AgentExecutor` for executing LLM-powered steps
//! in Pipeliner pipelines using the Rig framework.
//!
//! Note: `LlmAgentConfig` is defined in `pipeliner-core` as part of the domain model.
//!
//! ## Features
//!
//! - `rig` (default): Use real LLM providers via Rig
//! - (none): Use stub client for testing
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pipeliner_agent::{AgentExecutor, ModelTool};
//! use pipeliner_core::LlmAgentConfig;
//!
//! let config = LlmAgentConfig::new("claude-3-5-sonnet")
//!     .with_prompt("Review this code");
//!
//! let executor = AgentExecutor::new();
//! let result = executor.execute(&config).await;
//! ```

pub mod config;
pub mod executor;
pub mod skill;
pub mod tools;

// Conditional compilation for LLM client
#[cfg(feature = "rig")]
pub mod rig_client;

#[cfg(not(feature = "rig"))]
pub mod stub_client;

pub mod rig_integration;

// Re-export LlmAgentConfig from pipeliner-core
pub use pipeliner_core::LlmAgentConfig;

// Export our types
pub use config::ModelTool;
pub use executor::{AgentExecutor, AgentResult, AgentStatus};
pub use tools::ToolRegistry;
