//! Rig integration for LLM execution
//!
//! This module provides integration with the Rig framework for LLM calls.

#[cfg(feature = "rig")]
pub mod client {
    use anyhow::{Context, Result};
    
    /// Create a Rig agent client based on model name
    pub fn create_agent(
        model: &str,
        _system_prompt: &str,
    ) -> Result<impl rig::agent::Agent> {
        use rig::providers::openai::Client;
        
        // Parse model name to determine provider
        let model_name = if model.contains("claude") {
            model.to_string()
        } else if model.contains("gpt") || model.contains("o1") {
            model.to_string()
        } else if model.contains("gemini") {
            model.to_string()
        } else {
            model.to_string() // default
        };
        
        // Default to OpenAI-compatible client
        let client = Client::from_env();
        Ok(client.agent(&model_name).build())
    }
    
    /// Execute a prompt with the agent
    pub async fn execute_prompt(
        agent: &impl rig::agent::Agent,
        prompt: &str,
    ) -> Result<String> {
        agent
            .prompt(prompt)
            .await
            .context("Failed to execute prompt")
    }
}

#[cfg(not(feature = "rig"))]
pub mod client {
    use anyhow::Result;
    
    /// Stub when Rig is not enabled
    pub fn create_agent(
        _model: &str,
        _system_prompt: &str,
    ) -> Result<StubAgent> {
        Ok(StubAgent)
    }
    
    /// Stub agent for non-Rig builds
    pub struct StubAgent;
    
    /// Execute prompt stub
    pub async fn execute_prompt(
        _agent: &StubAgent,
        prompt: &str,
    ) -> Result<String> {
        Ok(format!("[Stub] Prompt: {} chars", prompt.len()))
    }
}
