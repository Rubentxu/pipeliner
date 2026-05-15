//! MCP Tools for Pipeliner


use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub pipelines_dir: PathBuf,
    pub execution_history: Arc<RwLock<Vec<ExecutionRecord>>>,
}

/// Record of a pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub pipeline_name: String,
    pub status: String,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn ok<T: Serialize>(data: T) -> Self {
        Self {
            success: true,
            data: Some(serde_json::to_value(data).unwrap_or_default()),
            error: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

/// Pipeline metadata for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInfo {
    pub name: String,
    pub path: String,
    pub stages: Vec<String>,
    pub description: Option<String>,
}

/// Pipeline creation request
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePipelineRequest {
    pub name: String,
    pub yaml: String,
}

/// Pipeline execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub pipeline_name: String,
    pub status: String,
    pub stages_executed: usize,
    pub steps_executed: usize,
    pub duration_ms: u64,
    pub output: Option<String>,
}

/// Natural language pipeline request
#[derive(Debug, Clone, Deserialize)]
pub struct BuildFromNlRequest {
    pub description: String,
    pub model: Option<String>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            pipelines_dir: PathBuf::from("pipelines"),
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl ToolContext {
    pub fn new(pipelines_dir: impl Into<PathBuf>) -> Self {
        Self {
            pipelines_dir: pipelines_dir.into(),
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

/// Tool implementation for MCP
#[derive(Debug, Clone)]
pub struct PipelineTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl PipelineTool {
    pub fn new(name: &str, description: &str, input_schema: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_ok() {
        let result = ToolResult::ok(vec!["a", "b"]);
        assert!(result.success);
        assert!(result.data.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_err() {
        let result = ToolResult::err("Something went wrong");
        assert!(!result.success);
        assert!(result.data.is_none());
        assert_eq!(result.error.unwrap(), "Something went wrong");
    }

    #[test]
    fn test_tool_context_default() {
        let ctx = ToolContext::default();
        assert_eq!(ctx.pipelines_dir, PathBuf::from("pipelines"));
    }
}
