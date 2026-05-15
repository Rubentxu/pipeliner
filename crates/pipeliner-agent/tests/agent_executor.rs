//! Tests for pipeliner-agent

use pipeliner_agent::{AgentExecutor, AgentResult, AgentStatus, ModelTool, ToolRegistry};
use pipeliner_core::LlmAgentConfig;

#[tokio::test]
async fn test_agent_executor_creation() {
    let executor = AgentExecutor::new();
    // Just verify it can be created
    assert!(std::ptr::eq(executor.tool_registry(), executor.tool_registry()));
}

#[tokio::test]
async fn test_agent_executor_with_config() {
    let config = LlmAgentConfig::new("gpt-4").with_prompt("Say hello");
    
    let executor = AgentExecutor::new();
    
    let result = executor.execute(&config).await;
    assert!(result.is_ok());
    
    let result = result.unwrap();
    assert_eq!(result.status, AgentStatus::Success);
    assert!(result.output.is_some());
}

#[tokio::test]
async fn test_agent_result_success() {
    let result = AgentResult::success(Some("Hello, world!".to_string()));
    assert_eq!(result.status, AgentStatus::Success);
    assert_eq!(result.output, Some("Hello, world!".to_string()));
    assert!(result.error.is_none());
}

#[tokio::test]
async fn test_agent_result_failure() {
    let result = AgentResult::failure("Something went wrong");
    assert_eq!(result.status, AgentStatus::Failure);
    assert!(result.output.is_none());
    assert_eq!(result.error, Some("Something went wrong".to_string()));
}

#[test]
fn test_model_tool_creation() {
    let tool = ModelTool::new("read_file", "Read a file");
    
    assert_eq!(tool.name, "read_file");
    assert_eq!(tool.description, "Read a file");
    assert!(tool.schema.is_object());
}

#[test]
fn test_model_tool_with_schema() {
    use serde_json::json;
    
    let schema = json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string"
            }
        },
        "required": ["path"]
    });
    
    let tool = ModelTool::new("read_file", "Read a file").with_schema(schema.clone());
    
    assert_eq!(tool.schema, schema);
}

#[test]
fn test_tool_registry_default() {
    let registry = ToolRegistry::new();
    let tools = registry.list();
    
    assert!(!tools.is_empty());
    assert!(tools.contains(&"read_file".to_string()));
    assert!(tools.contains(&"grep".to_string()));
    assert!(tools.contains(&"bash".to_string()));
}

#[test]
fn test_tool_registry_resolve() {
    let registry = ToolRegistry::new();
    
    let tool = registry.resolve("read_file");
    assert!(tool.is_some());
    assert_eq!(tool.unwrap().name, "read_file");
}

#[test]
fn test_tool_registry_resolve_unknown() {
    let registry = ToolRegistry::new();
    
    let tool = registry.resolve("unknown_tool");
    assert!(tool.is_none());
}

#[test]
fn test_tool_registry_resolve_all() {
    let registry = ToolRegistry::new();
    
    let tools = registry.resolve_all(&[
        "read_file".to_string(),
        "grep".to_string(),
        "bash".to_string(),
    ]);
    
    assert_eq!(tools.len(), 3);
}

#[test]
fn test_tool_registry_contains() {
    let registry = ToolRegistry::new();
    
    assert!(registry.contains("read_file"));
    assert!(!registry.contains("unknown_tool"));
}

#[test]
fn test_tool_registry_read_file_schema() {
    let registry = ToolRegistry::new();
    
    let tool = registry.resolve("read_file").unwrap();
    
    // Verify the schema has path parameter
    let schema = &tool.schema;
    let props = schema.get("properties").unwrap();
    assert!(props.get("path").is_some());
}

#[test]
fn test_tool_registry_grep_schema() {
    let registry = ToolRegistry::new();
    
    let tool = registry.resolve("grep").unwrap();
    
    // Verify the schema has pattern parameter
    let schema = &tool.schema;
    let props = schema.get("properties").unwrap();
    assert!(props.get("pattern").is_some());
}

#[test]
fn test_tool_registry_bash_schema() {
    let registry = ToolRegistry::new();
    
    let tool = registry.resolve("bash").unwrap();
    
    // Verify the schema has command parameter
    let schema = &tool.schema;
    let props = schema.get("properties").unwrap();
    assert!(props.get("command").is_some());
}
