//! MCP Server implementation

use anyhow::Result;
use tracing::info;

/// MCP tool definition
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl Tool {
    pub fn new(name: &str, description: &str, input_schema: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        }
    }
}

/// Get all available MCP tools
pub fn get_tools() -> Vec<Tool> {
    vec![
        // List pipelines
        Tool::new(
            "pipeliner_list_pipelines",
            "List all available pipeline files in the pipelines directory",
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        // Create pipeline
        Tool::new(
            "pipeliner_create_pipeline",
            "Create a new pipeline from YAML or JSON definition",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Pipeline name" },
                    "yaml": { "type": "string", "description": "Pipeline definition in YAML format" }
                },
                "required": ["name", "yaml"]
            }),
        ),
        // Run pipeline
        Tool::new(
            "pipeliner_run_pipeline",
            "Execute a pipeline by name",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Pipeline name to execute" },
                    "params": { "type": "object", "description": "Pipeline parameters", "additionalProperties": true }
                },
                "required": ["name"]
            }),
        ),
        // Validate pipeline
        Tool::new(
            "pipeliner_validate_pipeline",
            "Validate a pipeline YAML/JSON definition",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "yaml": { "type": "string", "description": "Pipeline definition to validate" }
                },
                "required": ["yaml"]
            }),
        ),
        // Build from natural language
        Tool::new(
            "pipeliner_build_from_nl",
            "Build a pipeline from natural language description (requires LLM)",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "description": { "type": "string", "description": "Natural language description" }
                },
                "required": ["description"]
            }),
        ),
    ]
}

/// Run the MCP server
pub async fn run() -> Result<()> {
    let tools = get_tools();
    info!("Starting Pipeliner MCP Server with {} tools", tools.len());

    for tool in &tools {
        info!("  - {}: {}", tool.name, tool.description);
    }

    println!("\n=== Pipeliner MCP Server ===");
    println!("Tools available:");
    for tool in &tools {
        println!("  - {}", tool.name);
    }
    println!("\nNote: Full MCP server with JSON-RPC pending.");

    Ok(())
}
