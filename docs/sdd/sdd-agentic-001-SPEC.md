# SDD-001: Pipeliner Agentic-First + DSL Macros

**Change**: Agentic pipeline support with Jenkins-style DSL macros
**Date**: 2025-05-15
**Status**: Active

---

## Executive Summary

Implementar soporte para pipelines agentic-first donde `AgentStep` invoca LLMs via Rig, con un DSL estilo Jenkinsfile Declarativo usando macros Rust.

---

## Decisions (from grilling)

### 1. Terminology

| Concept | Name |
|---------|------|
| LLM provider interface | `ModelProvider` |
| Step type | `AgentStep` |
| Executor | `AgentExecutor` |
| Config | `AgentConfig` |
| Tool definition | `ModelTool` |

### 2. DSL Syntax (Jenkinsfile-style)

```rust
pipeline! {
    agent(any)
    
    stages {
        stage("Build") {
            steps {
                sh!("make build")
                echo!("Done")
            }
        }
        
        stage("Review") {
            steps {
                agent(model: "claude-3-5-sonnet") {
                    prompt = "Review PR #123 for security issues"
                    tools = ["read_file", "grep"]
                    skill = "code-review"
                }
            }
        }
    }
    
    post {
        always {
            echo!("Cleanup")
        }
        success {
            echo!("Build succeeded!")
        }
    }
}
```

### 3. AgentStep Behavior
- **Single-shot**: One LLM call per step (no tool loop)
- Tool loop can be wrapped in a future `AgentWorkflow`

### 4. Skill Format
- Markdown file (`.md`)
- Path resolved from pipeline working directory

### 5. Stack
- **Rig**: `rig-core`, `rig-compose`, `rig-mcp`, `rig-resources`
- **No custom LLM providers**: delegate to Rig

---

## Requirements

### REQ-1: DSL Macros

| Macro | Description |
|-------|-------------|
| `pipeline!` | Define complete pipeline |
| `stage!` | Single stage with name |
| `steps!` | Block containing steps |
| `agent!` | Agent step with config block |
| `sh!` | Shell command |
| `echo!` | Echo message |
| `post!` | Post-conditions block |
| `always!`, `success!`, `failure!` | Post conditions |

### REQ-2: AgentStep Type

```rust
pub struct AgentConfig {
    pub model: String,
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Vec<String>,      // Tool names
    pub skill: Option<String>,   // Path to .md file
}
```

### REQ-3: ModelConfig for Rig

```rust
pub struct ModelConfig {
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Vec<ModelTool>,
    pub skill: Option<String>,
}

pub struct ModelTool {
    pub name: String,
    pub description: String,
    pub schema: Value,  // JSON Schema
}
```

### REQ-4: Integration with Rig

- Use `rig-core` for LLM providers
- Use `rig-mcp` for MCP tool integration
- Use `rig-resources` for skill loading
- `AgentExecutor` wraps Rig Agent

### REQ-5: Tool Resolution

| Source | Example |
|--------|---------|
| Built-in | `sh`, `echo`, `read_file`, `grep` |
| StepRegistry | Custom steps registered |
| MCP | Via `rig-mcp` |

---

## Non-Goals

- No custom LLM providers (use Rig)
- No tool loop autonomy (single-shot only)
- No multi-agent orchestration yet

---

## Artifacts

- `crates/pipeliner-agent/` - New crate for AgentStep
- `crates/pipeliner-macros/` - Extended DSL macros
- `docs/sdd/sdd-agentic-001-DESIGN.md` - Architecture
