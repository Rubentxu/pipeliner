# SDD-001: Design - Agentic Pipeline Support

## 1. Crate Structure

```
crates/
├── pipeliner-agent/          # NEW: Agent step execution
│   ├── src/
│   │   ├── lib.rs
│   │   ├── config.rs        # AgentConfig, ModelConfig, ModelTool
│   │   ├── executor.rs      # AgentExecutor
│   │   ├── provider.rs      # ModelProvider trait
│   │   ├── tools.rs         # Tool resolution
│   │   └── skill.rs         # Skill loading from .md
│   └── Cargo.toml
│
├── pipeliner-macros/        # EXTEND: DSL macros
│   ├── src/
│   │   ├── lib.rs           # Macros: pipeline!, stage!, steps!, etc.
│   │   ├── agent.rs         # agent! macro
│   │   ├── steps.rs          # sh!, echo!, etc.
│   │   └── post.rs           # post!, always!, etc.
│   └── Cargo.toml
```

## 2. Dependencies

### pipeliner-agent/Cargo.toml
```toml
[package]
name = "pipeliner-agent"
version = "0.1.0"
edition = "2021"

[dependencies]
pipeliner-core = { path = "../pipeliner-core" }
rig-core = "0.37"
rig-mcp = "0.1"
rig-resources = "0.1"
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tokio = { version = "1.35", features = ["full"] }
tracing = "0.1"
anyhow = "1.0"
```

### pipeliner-macros/Cargo.toml (extend)
```toml
[package]
name = "pipeliner-macros"
version = "0.1.0"
edition = "2021"

[dependencies]
proc-macro2 = "1.0"
quote = "1.0"
syn = { version = "2.0", features = ["full", "parsing"] }
pipeliner-core = { path = "../pipeliner-core" }
pipeliner-agent = { path = "../pipeliner-agent" }
```

## 3. Type Definitions

### AgentConfig (pipeliner-core)

```rust
// In: pipeliner-core/src/pipeline/mod.rs

/// Agent step configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Model to use (e.g., "claude-3-5-sonnet")
    pub model: String,
    
    /// System prompt for the agent
    #[serde(default)]
    pub prompt: String,
    
    /// Maximum tokens in response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    
    /// Temperature (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    
    /// Tools to make available
    #[serde(default)]
    pub tools: Vec<String>,
    
    /// Path to skill file (.md)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
}
```

### StepType::Agent (pipeliner-core)

```rust
// In: pipeliner-core/src/pipeline/mod.rs

pub enum StepType {
    // ... existing variants ...
    
    /// Agent execution step using LLM
    Agent {
        /// Agent configuration
        config: AgentConfig,
    },
}
```

### ModelTool (pipeliner-agent)

```rust
// In: pipeliner-agent/src/config.rs

/// Tool definition for LLM
#[derive(Debug, Clone)]
pub struct ModelTool {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,  // JSON Schema
}
```

## 4. Macro Design

### pipeline! Macro

```rust
// pipeline! { ... } generates:
Pipeline::new()
    .with_agent(AgentType::Any)
    .with_stages(vec![
        Stage::new("Build")
            .with_steps(vec![
                Step::shell("make build")
            ])
    ])
```

### stage! Macro

```rust
// stage!("Build") { steps { ... } } generates:
Stage::new("Build")
    .with_steps(vec![ /* steps */ ])
```

### steps! Macro

```rust
// steps { sh!("make") } generates:
vec![
    Step::shell("make")
]
```

### agent! Macro

```rust
// agent(model: "claude") { prompt = "..." } generates:
Step::agent(AgentConfig {
    model: "claude".to_string(),
    prompt: "...".to_string(),
    max_tokens: None,
    temperature: None,
    tools: vec![],
    skill: None,
})
```

### post! Macro

```rust
// post { always { ... } } generates:
PostCondition {
    always: vec![ /* steps */ ],
    success: vec![],
    failure: vec![],
    // ...
}
```

## 5. Executor Integration

### AgentExecutor (pipeliner-agent)

```rust
// In: pipeliner-agent/src/executor.rs

pub struct AgentExecutor {
    rig_agent: rig::Agent,
    tools: ToolRegistry,
}

impl AgentExecutor {
    pub async fn execute(
        &self,
        config: &AgentConfig,
        context: &ExecutionContext,
    ) -> Result<ExecutionStatus, AgentError> {
        // 1. Load skill if specified
        let skill_prompt = self.load_skill(&config.skill)?;
        
        // 2. Resolve tools
        let resolved_tools = self.resolve_tools(&config.tools)?;
        
        // 3. Build prompt with skill
        let full_prompt = format!("{}\n\nSkill:\n{}", config.prompt, skill_prompt);
        
        // 4. Execute via Rig
        let response = self.rig_agent
            .prompt(&full_prompt)
            .await?;
        
        // 5. Return status
        Ok(ExecutionStatus::Success)
    }
}
```

### StepExecutor Extension (pipeliner-executor)

```rust
// In: pipeliner-executor/src/runtime.rs

impl StepExecutor {
    async fn execute_agent(
        &self,
        config: &AgentConfig,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let agent_executor = context.agent_executor()?;
        agent_executor.execute(config, context).await
    }
}
```

## 6. Tool Resolution

```rust
// In: pipeliner-agent/src/tools.rs

pub enum ToolSource {
    BuiltIn,
    StepRegistry,
    Mcp(String),  // MCP server name
}

pub struct ToolRegistry {
    builtins: HashMap<String, BuiltinTool>,
    registry: Arc<dyn StepRegistry>,
    mcp_servers: HashMap<String, McpClient>,
}

impl ToolRegistry {
    pub fn resolve(&self, name: &str) -> Option<ModelTool> {
        // Check builtins first
        if let Some(tool) = self.builtins.get(name) {
            return Some(tool.to_model_tool());
        }
        
        // Check StepRegistry
        if let Some(factory) = self.registry.get(name) {
            return Some(factory.to_model_tool());
        }
        
        // Check MCP
        for server in self.mcp_servers.values() {
            if let Some(tool) = server.get_tool(name) {
                return Some(tool);
            }
        }
        
        None
    }
}
```

## 7. Skill Loading

```rust
// In: pipeliner-agent/src/skill.rs

pub fn load_skill(skill_path: &Option<String>) -> Result<String, AgentError> {
    match skill_path {
        Some(path) => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| AgentError::SkillLoad { path: path.clone(), source: e })?;
            Ok(content)
        }
        None => Ok(String::new()),
    }
}
```

## 8. File Map

```
Changed:
  crates/pipeliner-core/src/pipeline/mod.rs     # + AgentConfig, StepType::Agent
  crates/pipeliner-executor/src/runtime.rs      # + execute_agent handler
  crates/pipeliner-macros/src/lib.rs           # + pipeline!, stage!, steps!, post!
  crates/pipeliner-macros/Cargo.toml           # + dependencies

Created:
  crates/pipeliner-agent/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── config.rs      # AgentConfig, ModelTool
        ├── executor.rs    # AgentExecutor
        ├── provider.rs    # ModelProvider trait
        ├── tools.rs      # Tool resolution
        └── skill.rs      # Skill loading
```

## 9. Testing Strategy

### Unit Tests
- `AgentConfig` serialization/deserialization
- `load_skill` with valid/invalid paths
- Tool resolution precedence
- Macro expansion correctness

### Integration Tests
- Pipeline with `agent!` step compiles
- Pipeline with skill file executes
- Tool from StepRegistry accessible

### E2E Tests
- Real LLM call (with mock for CI)
- MCP tool integration
