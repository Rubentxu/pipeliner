//! Tool resolution and registry

use crate::config::ModelTool;
use std::collections::HashMap;
use std::sync::Arc;

/// Built-in tool definitions
pub struct BuiltinTools;

impl BuiltinTools {
    /// Get all built-in tools
    pub fn all() -> HashMap<String, ModelTool> {
        let mut tools = HashMap::new();

        // File reading
        tools.insert(
            "read_file".to_string(),
            ModelTool::new(
                "read_file",
                "Read the contents of a file. Takes a file path as input.",
            )
            .with_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            })),
        );

        // Grep
        tools.insert(
            "grep".to_string(),
            ModelTool::new(
                "grep",
                "Search for patterns in files. Takes a pattern and optional file paths.",
            )
            .with_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional path to search in"
                    }
                },
                "required": ["pattern"]
            })),
        );

        // Bash/Shell
        tools.insert(
            "bash".to_string(),
            ModelTool::new(
                "bash",
                "Execute a shell command. Returns command output.",
            )
            .with_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    }
                },
                "required": ["command"]
            })),
        );

        tools
    }

    /// Get a specific built-in tool
    pub fn get(name: &str) -> Option<ModelTool> {
        Self::all().get(name).cloned()
    }
}

/// Tool registry for resolving tool names to ModelTool definitions
pub struct ToolRegistry {
    builtins: HashMap<String, ModelTool>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            builtins: BuiltinTools::all(),
        }
    }

    /// Resolve a tool name to its definition
    pub fn resolve(&self, name: &str) -> Option<ModelTool> {
        self.builtins.get(name).cloned()
    }

    /// Resolve multiple tool names
    pub fn resolve_all(&self, names: &[String]) -> Vec<ModelTool> {
        names
            .iter()
            .filter_map(|name| self.resolve(name))
            .collect()
    }

    /// List all available tool names
    pub fn list(&self) -> Vec<String> {
        self.builtins.keys().cloned().collect()
    }

    /// Check if a tool exists
    pub fn contains(&self, name: &str) -> bool {
        self.builtins.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_tools() {
        let tools = BuiltinTools::all();
        assert!(tools.contains_key("read_file"));
        assert!(tools.contains_key("grep"));
        assert!(tools.contains_key("bash"));
    }

    #[test]
    fn test_resolve() {
        let registry = ToolRegistry::new();
        
        let tool = registry.resolve("read_file");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name, "read_file");
    }

    #[test]
    fn test_resolve_all() {
        let registry = ToolRegistry::new();
        
        let tools = registry.resolve_all(&["read_file".to_string(), "grep".to_string()]);
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn test_resolve_unknown() {
        let registry = ToolRegistry::new();
        
        let tool = registry.resolve("unknown_tool");
        assert!(tool.is_none());
    }
}
