//! Stub LLM client
//!
//! This module provides a stub when Rig is not enabled.

use anyhow::Result;

/// Stub agent
pub struct StubAgent;

/// Create a stub agent
pub fn create_agent(_model: &str, _system_prompt: &str) -> Result<StubAgent> {
    Ok(StubAgent)
}

/// Execute prompt stub
pub async fn execute_prompt(_agent: &StubAgent, prompt: &str) -> Result<String> {
    Ok(format!("[Stub] Prompt: {} chars", prompt.len()))
}
