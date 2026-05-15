//! Agent executor using Rig

use anyhow::Result;
use tracing::{debug, info};

/// Execution status for agent steps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Success,
    Failure,
    Skipped,
}

/// Result of agent execution
#[derive(Debug, Clone)]
pub struct AgentResult {
    pub status: AgentStatus,
    pub output: Option<String>,
    pub error: Option<String>,
}

impl AgentResult {
    /// Create a success result
    pub fn success(output: Option<String>) -> Self {
        Self {
            status: AgentStatus::Success,
            output,
            error: None,
        }
    }

    /// Create a failure result
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            status: AgentStatus::Failure,
            output: None,
            error: Some(error.into()),
        }
    }
}

/// Agent executor for running LLM-powered steps
pub struct AgentExecutor {
    tool_registry: std::sync::Arc<crate::tools::ToolRegistry>,
}

impl Default for AgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentExecutor {
    /// Create a new agent executor
    pub fn new() -> Self {
        Self {
            tool_registry: std::sync::Arc::new(crate::tools::ToolRegistry::new()),
        }
    }

    /// Create with custom tool registry
    #[must_use]
    pub fn with_tool_registry(registry: crate::tools::ToolRegistry) -> Self {
        Self {
            tool_registry: std::sync::Arc::new(registry),
        }
    }

    /// Execute an agent step with the given configuration
    pub async fn execute(&self, config: &pipeliner_core::LlmAgentConfig) -> Result<AgentResult> {
        info!(
            model = %config.model,
            prompt_len = config.prompt.len(),
            tools = ?config.tools,
            skill = ?config.skill,
            "Executing agent step"
        );

        // 1. Load skill content if specified
        let skill_content = crate::skill::load_skill(&config.skill)?;
        if !skill_content.is_empty() {
            debug!(skill_len = skill_content.len(), "Loaded skill content");
        }

        // 2. Resolve tools
        let tools = self.tool_registry.resolve_all(&config.tools);
        debug!(tool_count = tools.len(), "Resolved tools");

        // 3. Build prompt with skill
        let full_prompt = if skill_content.is_empty() {
            config.prompt.clone()
        } else {
            format!(
                "{}\n\n## Skill Context\n{}\n\n## Task\nUse the skill context above to:",
                config.prompt, skill_content
            )
        };

        // 4. Execute via Rig or stub
        let result = self.execute_with_client(&config.model, &full_prompt, &tools).await?;

        info!(
            status = ?result.status,
            output_len = result.output.as_ref().map(|s| s.len()).unwrap_or(0),
            "Agent step completed"
        );

        Ok(result)
    }

    /// Execute with the configured LLM client
    async fn execute_with_client(
        &self,
        model: &str,
        prompt: &str,
        _tools: &[crate::config::ModelTool],
    ) -> Result<AgentResult> {
        #[cfg(feature = "rig")]
        {
            let agent = crate::rig_client::create_agent(model, "")
                .map_err(|e| anyhow::anyhow!("Failed to create agent: {}", e))?;

            match crate::rig_client::execute_prompt(&agent, prompt).await {
                Ok(response) => Ok(AgentResult::success(Some(response))),
                Err(e) => Ok(AgentResult::failure(format!("LLM error: {}", e))),
            }
        }

        #[cfg(not(feature = "rig"))]
        {
            let agent = crate::stub_client::create_agent(model, "")?;
            let response = crate::stub_client::execute_prompt(&agent, prompt).await?;
            Ok(AgentResult::success(Some(response)))
        }
    }

    /// Get the tool registry
    #[must_use]
    pub fn tool_registry(&self) -> &crate::tools::ToolRegistry {
        &self.tool_registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_with_prompt() {
        let executor = AgentExecutor::new();

        let config = pipeliner_core::LlmAgentConfig::new("gpt-4")
            .with_prompt("Say hello");

        let result = executor.execute(&config).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.status, AgentStatus::Success);
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_execute_with_skill() {
        let executor = AgentExecutor::new();

        // Create a temp skill file
        let temp_dir = tempfile::TempDir::new().unwrap();
        let skill_path = temp_dir.path().join("test.md");
        std::fs::write(&skill_path, "# Test Skill\n\nBe helpful.").unwrap();

        let config = pipeliner_core::LlmAgentConfig::new("gpt-4")
            .with_prompt("Greet user")
            .with_skill(skill_path.to_string_lossy().as_ref());

        let result = executor.execute(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_tools() {
        let executor = AgentExecutor::new();

        let config = pipeliner_core::LlmAgentConfig::new("gpt-4")
            .with_prompt("Count files")
            .with_tools(vec!["bash".to_string(), "read_file".to_string()]);

        let result = executor.execute(&config).await;
        assert!(result.is_ok());
    }
}
