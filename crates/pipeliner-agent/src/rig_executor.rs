//! Ejemplo de uso de Rig real
//!
//! ```rust
//! // Habilitar feature "rig" en Cargo.toml
//! // [features]
//! rig = []
//!
//! // Usar provider real
//! use pipeliner_agent::{AgentExecutor, LlmAgentConfig};
//! use pipeliner_core::LlmAgentConfig;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = LlmAgentConfig::new("claude")
//!         .with_prompt("Explain this code");
//!     let executor = AgentExecutor::new();
//!     let result = executor.execute(&config).await?;
//!     println!("{}", result.output);
//! }
//! ```
//!
//! Para usar con OpenAI u otro provider:
//! ```rust,ignore
//! // En rig_client.rs
//! use rig::providers::{openai, anthropic};
//! let client = openai::Client::from_env();
//! // o
//! let client = anthropic::Client::from_env();
//! ```

//! Implementación de rig_client.rs para producción
#[cfg(feature = "rig")]
pub mod rig_client {
    use anyhow::{Context, Result};
    
    //! Crear cliente según modelo
    pub fn create_client(model: &str) -> Result<impl rig::providers::Client> {
        let client = if model.contains("claude") {
            rig::providers::anthropic::Client::from_env()
        } else if model.contains("gemini") {
            rig::providers::gemini::Client::from_env()
        } else {
            rig::providers::openai::Client::from_env()
        };
        Ok(client)
    }
    
    //! Ejecutar prompt con cliente
    pub async fn run_prompt(
        client: &impl rig::providers::Client,
        model: &str,
        prompt: &str,
    ) -> Result<String> {
        let agent = client.agent(model);
        agent.prompt(prompt)
            .await
            .context("LLM call failed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rig_client() {
        // Solo corre si RIG_API_KEY está configurada
        if std::env::var("OPENAI_API_KEY").is_err() 
            && std::env::var("ANTHROPIC_API_KEY").is_err() {
            eprintln!("SKIP: No API key configured");
            return;
        }
        
        let result = create_client("gpt-4").await;
        assert!(result.is_ok());
    }
}
