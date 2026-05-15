# SDD-001: Pipeliner Agentic-First + DSL Macros

**Change**: Agentic pipeline support with Jenkins-style DSL macros
**Date**: 2025-05-15
**Status**: Implementado

---

## Executive Summary

Implementar soporte para pipelines agentic-first donde `AgentStep` invoca LLMs via Rig, con un DSL estilo Jenkinsfile.

---

## Decisions (from grilling)

### 1. Terminology

| Concept | Name |
|---------|------|
| LLM provider interface | `rig_client.rs` |
| Step type | `Step::agent()` |
| Executor | `AgentExecutor` |
| Config | `LlmAgentConfig` |
| Tool definition | `ModelTool` |
| Skill | Markdown file (`.md`) |

### 2. DSL Syntax (Jenkinsfile-style)

```rust
// Builder pattern (IMPLEMENTADO)
Stage::new("Build")
    .with_steps(vec![
        sh!("cargo build"),
        echo!("Build complete!")
    ])

// Macro sh!, echo! (IMPLEMENTADO)
sh!("cargo build")
echo!("Done!")
```

### 3. AgentStep Behavior

- **Single-shot**: Un LLM call por step (no tool loop)
- Tool loop puede ser `AgentWorkflow` futuro

### 4. Skill Format

- Markdown file (`.md`)
- Path resuelto desde pipeline working directory

### 5. Stack

- **Rig**: `rig-core`, `rig-mcp`, `rig-resources`
- **No custom LLM providers**: Delegar a Rig

---

## Requirements (IMPLEMENTADO)

### REQ-1: DSL Macros

| Macro | Status |
|-------|---------|
| `sh!("command")` | ✅ Implementado |
| `echo!("message") | ✅ Implementado |
| `stage!` | ⚠️ Parser complejo |
| `agent!` | ⚠️ Parser complejo |
| `pipeline!` | ⚠️ Parser complejo |

### REQ-2: AgentStep Type

```rust
pub struct LlmAgentConfig {
    pub model: String,
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Vec<String>,
    pub skill: Option<String>,
}

Step::agent(config)
```

### REQ-3: Integration with Rig

- Feature flag `rig` para LLM real
- Stub mode para testing

### REQ-4: MCP Pipeline Builder

| Tool | Status |
|------|---------|
| `pipeliner_list_pipelines` | ⚠️ Skeleton |
| `pipeliner_create_pipeline` | ⚠️ Skeleton |
| `pipeliner_run_pipeline` | ⚠️ Skeleton |

---

## Non-Goals

- No custom LLM providers (usar Rig)
- No tool loop autonomy (single-shot)
- No multi-agent orchestration todavía

---

## Artifacts

- `crates/pipeliner-agent/` - Agent step execution
- `crates/pipeliner-macros/` - DSL macros
- `crates/pipeliner-mcp/` - MCP server
- `docs/sdd/sdd-agentic-001-*` - SDD docs

---

## Next Steps

1. Parser macros `pipeline!`, `stage!`, `agent!`
2. Rig integration real (`--features rig`)
3. MCP server JSON-RPC completo
4. Self-healing observer
5. Plan + Execute pattern
