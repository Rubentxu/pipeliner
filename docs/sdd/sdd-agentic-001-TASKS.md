# SDD-001: Tasks - Agentic Pipeline Support

## Batch 1: Core Types (pipeliner-agent)

### T1.1: Create pipeliner-agent crate
```bash
mkdir -p crates/pipeliner-agent/src
```
Create `Cargo.toml` with Rig dependencies.

### T1.2: Define AgentConfig in pipeliner-core
Add to `pipeliner-core/src/pipeline/mod.rs`:
- `AgentConfig` struct
- `ModelTool` struct (in pipeliner-agent)

### T1.3: Add StepType::Agent variant
Extend `StepType` enum in pipeliner-core.

### T1.4: Implement AgentExecutor
Create `pipeliner-agent/src/executor.rs` with Rig integration.

### T1.5: Implement Tool resolution
Create `pipeliner-agent/src/tools.rs`.

### T1.6: Implement Skill loading
Create `pipeliner-agent/src/skill.rs`.

---

## Batch 2: Executor Integration (pipeliner-executor)

### T2.1: Add execute_agent handler
In `pipeliner-executor/src/runtime.rs`, add:
```rust
StepType::Agent { config } => self.execute_agent(config, context).await
```

### T2.2: Add AgentExecutor to ExecutionContext
Update context to hold `AgentExecutor` reference.

---

## Batch 3: DSL Macros (pipeliner-macros)

### T3.1: Extend Cargo.toml
Add `pipeliner-agent` and `proc-macro2` dependencies.

### T3.2: Implement steps! macro
Parse block of steps into `Vec<Step>`.

### T3.3: Implement stage! macro
Parse `stage!("name") { steps { ... } }`.

### T3.4: Implement agent! macro
Parse `agent(model: "...") { prompt = "...", tools = [...], skill = "..." }`.

### T3.5: Implement post! macro
Parse `post { always { ... }, success { ... } }`.

### T3.6: Update pipeline! macro
Support new syntax with `stages { }` and `post { }`.

---

## Batch 4: Testing

### T4.1: Unit tests for AgentConfig
Serialization roundtrip tests.

### T4.2: Unit tests for macros
Macro expansion tests.

### T4.3: Integration test
Pipeline with `agent!` step compiles and runs.

---

## Batch 5: Documentation

### T5.1: Update AGENTS.md
Document AgentStep usage.

### T5.2: Add examples
Create `examples/agent-pipeline.rs`.

---

## Estimated Effort

| Batch | Tasks | Effort |
|-------|-------|--------|
| 1 | 6 | High |
| 2 | 2 | Medium |
| 3 | 6 | High |
| 4 | 3 | Medium |
| 5 | 2 | Low |

**Total**: ~19 tasks
