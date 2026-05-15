//! Configuration types for AgentStep
//!
//! Note: AgentConfig is defined in pipeliner-core as it's part of the domain model.
//! This module provides ModelTool for LLM integration.

use serde::{Deserialize, Serialize};

/// Tool definition for LLM integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTool {
    /// Tool name
    pub name: String,
    /// Tool description for the LLM
    pub description: String,
    /// JSON Schema for tool parameters
    #[serde(default)]
    pub schema: serde_json::Value,
}

impl ModelTool {
    /// Create a new ModelTool
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    /// Create with custom schema
    #[must_use]
    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.schema = schema;
        self
    }
}
