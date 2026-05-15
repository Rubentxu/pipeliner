# Pipeliner Agent

LLM-powered step execution for Pipeliner using the Rig framework.

## Features

- **AgentStep**: Execute LLM-powered steps in pipelines
- **Tool Registry**: Built-in tools (read_file, grep, bash) + custom tools
- **Skill Loading**: Load skills from Markdown files
- **Rig Integration**: Uses Rig for LLM provider abstraction
- **Feature Flags**: Compile with or without Rig for testing

## Installation

```toml
[dependencies]
pipeliner-agent = { version = "0.1", features = ["rig"] }  # Real LLM
pipeliner-agent = "0.1"  # Stub mode (testing)
```

## Usage

### Basic Agent Step

```rust
use pipeliner_core::{LlmAgentConfig, Pipeline, Stage, Step};
use pipeliner_agent::AgentExecutor;

let config = LlmAgentConfig::new("claude-3-5-sonnet")
    .with_prompt("Review this code for bugs")
    .with_tools(vec!["read_file".to_string(), "grep".to_string()])
    .with_skill("skills/code-review.md");

let agent_step = Step::agent(config).with_name("code-reviewer");

let pipeline = Pipeline::new()
    .with_name("Agent Pipeline")
    .with_stage(Stage::new("Review").with_step(agent_step));

// Execute
let executor = AgentExecutor::new();
let result = executor.execute(&LlmAgentConfig::new("gpt-4")
    .with_prompt("Analyze"))
    .await;
```

### DSL Macros

```rust
use pipeliner_core::{LlmAgentConfig, Pipeline, Stage};
use pipeliner_macros::{sh, echo};

let config = LlmAgentConfig::new("claude")
    .with_prompt("Analyze this")
    .with_tools(vec!["bash".to_string()]);

let pipeline = Pipeline::new()
    .with_stage(Stage::new("Analyze")
        .with_step(sh!("echo 'Starting...'"))
        .with_step(Step::agent(config)));
```

## Configuration

### LlmAgentConfig

| Field | Type | Description |
|-------|------|-------------|
| `model` | String | LLM model (e.g., "claude-3-5-sonnet") |
| `prompt` | String | System prompt for the agent |
| `max_tokens` | Option<u32> | Maximum tokens in response |
| `temperature` | Option<f32> | Creativity (0.0-1.0) |
| `tools` | Vec<String> | Tool names to make available |
| `skill` | Option<String> | Path to skill .md file |

### Built-in Tools

| Tool | Description |
|------|-------------|
| `read_file` | Read file contents |
| `grep` | Search patterns in files |
| `bash` | Execute shell commands |

### Skill Files

Skills are Markdown files that provide additional context:

```markdown
# Code Review Skill

You are an expert code reviewer. Focus on:
- Security vulnerabilities
- Performance issues
- Code style
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `rig` (default) | Real LLM integration via Rig |
| (none) | Stub client for testing |

```bash
# Run with real LLM
cargo test -p pipeliner-agent --features "pipeliner-agent/rig"

# Run with stub (no API calls)
cargo test -p pipeliner-agent
```

## Running Examples

```bash
cd crates
cargo run -p pipeliner-agent --example agent-pipeline
```

## Architecture

```
pipeliner-agent/
├── src/
│   ├── config.rs        # ModelTool definition
│   ├── executor.rs      # AgentExecutor
│   ├── rig_client.rs    # Real Rig integration
│   ├── stub_client.rs   # Stub for testing
│   ├── skill.rs         # Skill loading
│   ├── tools.rs         # ToolRegistry
│   └── lib.rs          # Public API
├── Cargo.toml
├── README.md
└── examples/
    └── agent-pipeline.rs
```

## Dependencies

- `rig-core` - LLM providers (OpenAI, Anthropic, etc.)
- `rig-mcp` - MCP integration
- `rig-resources` - Skill loading
- `pipeliner-core` - Domain types
- `tokio` - Async runtime
