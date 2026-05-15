# Investigación: Pipeliner Agentic-First + MCP Pipeline Builder

> Estado: Draft | Fecha: 2025-05-15

---

## Resumen Ejecutivo

Esta investigación explora cómo transformar **Pipeliner** en un sistema **agentic-first** donde:
- Los pipelines pueden invocar agentes LLM como steps
- El MCP server se convierte en un **Pipeline Builder** que permite a agentes construir pipelines
- El sistema tiene capacidades de **self-healing** y **auto-optimización**

---

## 1. Estado Actual de Pipeliner

### 1.1 Arquitectura de Crates

```
pipeliner/
├── pipeliner-core/        # Domain types, Pipeline DSL
│   ├── Pipeline, Stage, Step, StepType
│   ├── StepRegistry       # Extension point para custom steps
│   └── Runner             # PipelineRunner con callbacks
├── pipeliner-executor/    # Execution engine
│   ├── StepExecutor       # Ejecuta steps
│   ├── ExecutionContext   # Estado durante ejecución
│   ├── PipelineObserver   # Hooks de eventos
│   └── Observers          # JsonCollector, LoggingObserver
├── pipeliner-events/      # Event sourcing
│   ├── EventBus           # Pub/sub trait
│   ├── EventHandler       # Handler trait
│   └── EventEnvelope      # AnyEvent (Pipeline, Worker, Infra)
├── pipeliner-cli/         # CLI commands
├── pipeliner-api/         # gRPC + REST
└── pipeliner-events/      # Event types
```

### 1.2 Extension Points Actuales

| Pattern | Ubicación | Uso |
|---------|-----------|-----|
| `StepFactory` | `pipeliner-core/registry.rs` | Registrar steps custom |
| `StepRegistry` | `pipeliner-core/registry.rs` | Gestionar factories |
| `PipelineObserver` | `pipeliner-executor/observers.rs` | Hooks de ejecución |
| `EventBus` | `pipeliner-events/event_bus/mod.rs` | Pub/sub async |
| `EventHandler` | `pipeliner-events/event_bus/mod.rs` | Consumir eventos |

### 1.3 StepType Enum Actual

```rust
pub enum StepType {
    Shell { command: String },      // ✅ Ejecuta comando
    Echo { message: String },      // ✅ Output
    Retry { count, step },         // ✅ Retry wrapper
    Timeout { duration, step },    // ✅ Timeout wrapper
    Script { content },            // ✅ Script inline
    Custom { name, config },       // ⚠️ Extension point
    When { condition, steps },     // ✅ Conditional
    Dir { path, steps },          // ✅ Change dir
    // ... 10+ más
}
```

**Gap**: No existe `Agent` variant.

---

## 2. Ideas Propuestas y Análisis de Viabilidad

### 2.1 AgentStep: Agente como Step

#### Concepto
```yaml
stages:
  - name: review
    steps:
      - type: agent
        agent: code-reviewer
        prompt: "Review PR #123 for security issues"
        model: claude-3-5-sonnet
        tools: [read_file, grep, bash]
```

#### Análisis de Código Existente

**StepType**: Necesita nuevo variant:
```rust
// En pipeliner-core/src/pipeline/mod.rs
Agent {
    agent_id: String,
    prompt: String,
    model: Option<String>,
    tools: Vec<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    output_format: AgentOutputFormat,
}
```

**StepExecutor**: Necesita handler:
```rust
// En pipeliner-executor/src/runtime.rs
StepType::Agent { agent_id, prompt, model, tools } => {
    self.execute_agent(agent_id, prompt, model, tools, step, context).await
}
```

#### Impacto y Complejidad

| Aspecto | Estimación |
|--------|-----------|
| Líneas de código | ~200-400 |
| Nuevos crates | 1 (`pipeliner-agent`) |
| Dependencias | LLM provider (anthropic, openai) |
| Complejidad | Media |
| Testing | Medio |

#### Plan de Implementación

1. Crear `pipeliner-agent` crate
2. Definir `AgentConfig` y `AgentProvider` trait
3. Agregar `StepType::Agent` variant
4. Implementar `AgentExecutor` en runtime
5. Agregar tests con mock LLM

---

### 2.2 MCP Pipeline Builder

#### Concepto

El MCP server de Pipeliner expone herramientas para que **agentes externos** puedan:
1. Listar pipelines disponibles
2. Crear nuevos pipelines
3. Ejecutar pipelines
4. Monitorear ejecución

```json
// Tools expuestas via MCP
{
  "tools": [
    {
      "name": "pipeliner_list",
      "description": "List all pipelines"
    },
    {
      "name": "pipeliner_create",
      "description": "Create a new pipeline from spec"
    },
    {
      "name": "pipeliner_run",
      "description": "Execute a pipeline"
    },
    {
      "name": "pipeliner_get_status",
      "description": "Get execution status"
    },
    {
      "name": "pipeliner_build_from_natural_language",
      "description": "Build pipeline from natural language description"
    }
  ]
}
```

#### Análisis de Código Existente

**API existente**: `pipeliner-api` tiene gRPC + REST
```rust
// pipeliner-api/src/grpc/mod.rs
pub mod pipeline_service {
    rpc ListPipelines(ListRequest) returns (ListResponse);
    rpc CreatePipeline(CreateRequest) returns (CreateResponse);
    rpc RunPipeline(RunRequest) returns (RunResponse);
}
```

**MCP actual**: Solo CogniCode (análisis de código)

#### Gap Identificado

| Funcionalidad | Existe | Via |
|--------------|--------|-----|
| Listar pipelines | ✅ | gRPC/REST |
| Crear pipeline | ✅ | gRPC/REST |
| Ejecutar pipeline | ✅ | gRPC/REST |
| Pipeline from NL | ❌ | **Nuevo** |
| MCP wrapper | ❌ | **Nuevo** |

#### Plan de Implementación

1. Crear `pipeliner-mcp` crate
2. Implementar MCP server con tool definitions
3. Integrar con `PipelineBuilder` para "build from NL"
4. Exponer via stdio o HTTP

---

### 2.3 Self-Healing Pipeline

#### Concepto

```
┌─────────────┐    Step Fails    ┌──────────────────┐
│   Pipeline  │ ───────────────► │  Agent Fixer     │
│   Runner    │                  │  (analyze error) │
└─────────────┘                  └────────┬─────────┘
       ▲                                  │
       │        Retry with Fix             │
       └──────────────────────────────────┘
```

#### Análisis de Código Existente

**PipelineObserver** tiene hooks:
```rust
pub trait PipelineObserver: Send + Sync {
    fn on_step_complete(&self, _ctx: &PipelineContext, _duration: Duration, _success: bool) {}
    fn on_error(&self, _ctx: &PipelineContext, _error: &str) {}
}
```

**StepExecutor** tiene retry:
```rust
StepType::Retry { count, step } => {
    self.execute_retry(inner, *count, ...).await
}
```

#### Gap Identificado

| Funcionalidad | Existe | Via |
|--------------|--------|-----|
| Retry on failure | ✅ | `StepType::Retry` |
| Observer hooks | ✅ | `PipelineObserver` |
| Agent-based fix | ❌ | **Nuevo** |
| Analyze error | ❌ | **Nuevo** |

#### Plan de Implementación

1. Crear `SelfHealingObserver`
2. Integrar con `AgentExecutor`
3. Policy config para max self-heal attempts
4. Telemetry para tracking

---

### 2.4 Plan + Execute Pattern

#### Concepto

```yaml
stages:
  - name: plan
    steps:
      - type: agent
        agent: planner
        prompt: "Create execution plan from: {{user_requirement}}"
        output_file: plan.json
        output_format: json

  - name: execute
    steps:
      - type: from_plan
        file: plan.json

  - name: verify
    steps:
      - type: agent
        agent: verifier
        prompt: "Verify execution matches: {{user_requirement}}"
```

#### Análisis de Código Existente

**StepType::Custom** permite extensibilidad:
```rust
StepType::Custom { name: "from_plan", config: {...} }
```

**StepRegistry** permite registrar factories:
```rust
registry.register(Arc::new(FromPlanFactory::new()));
```

#### Plan de Implementación

1. Definir `PlanStep` config
2. Crear `PlanExecutor` 
3. Integrar con `AgentStep` para generación de plan

---

### 2.5 Pipeline como Tool (MCP)

#### Concepto

Pipeliner se registra como tool en un agente externo:

```rust
// En un agente externo (VTCode, Claude Code, etc.)
{
  "name": "pipeliner",
  "description": "Execute CI/CD pipelines",
  "input_schema": {
    "type": "object",
    "properties": {
      "pipeline": { "type": "string" },
      "params": { "type": "object" }
    }
  }
}
```

#### Análisis de Código Existente

**StepRegistry** ya permite registering custom steps:
```rust
pub trait StepFactory: Send + Sync {
    fn name(&self) -> &str;
    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError>;
}
```

**pipeliner-cli** tiene commands:
```rust
// pipeliner-cli/src/commands/mod.rs
pub mod run;
pub mod list;
pub mod init;
```

#### Plan de Implementación

1. Crear `PipelineToolFactory`
2. Exponer via MCP como tool
3. Documentar tool schema

---

### 2.6 Multi-Agent Patterns

#### Conceptos

| Pattern | Descripción | Aplicación |
|---------|-------------|------------|
| **Router** | Agente decide rama | `when: agent_decision` |
| **Critic** | Agente valida output | Post-step verification |
| **Sequencer** | Pasa resultado | `{{previous_output}}` |
| **Parallel** | Voting | `stage.parallel: true` + multi-agent |
| **Debate** | Moderator decide | Stage con multiple agents |

#### Análisis de Código Existente

**Matrix execution** existe para parallel:
```rust
// pipeliner-core/src/matrix.rs
pub struct MatrixConfig {
    pub axes: Vec<MatrixAxis>,
    pub exclude: Vec<MatrixExclude>,
}
```

**When conditions**:
```rust
StepType::When { condition, steps }
```

#### Plan de Implementación

1. Extender `MatrixConfig` para multi-agent
2. Agregar `StepType::AgentRouter`
3. Implementar `DebateRunner`

---

## 3. Arquitectura Propuesta

### 3.1 Diagrama de Alto Nivel

```
┌─────────────────────────────────────────────────────────────────┐
│                        External Agent                            │
│                    (VTCode, Claude Code)                        │
└────────────────────────────┬────────────────────────────────────┘
                             │ MCP
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Pipeliner MCP Server                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Pipeline     │  │ Execution    │  │ Build from   │          │
│  │ Builder API  │  │ Manager      │  │ NL           │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└────────────────────────────┬────────────────────────────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
┌─────────────────┐ ┌─────────────┐ ┌─────────────────┐
│ pipeliner-core  │ │pipeliner-   │ │  pipeliner-     │
│                 │ │executor     │ │  agent          │
│ • Pipeline      │ │             │ │                 │
│ • Stage         │ │• Executor   │ │ • AgentProvider │
│ • Step          │ │• Observer   │ │ • AgentExecutor │
│ • StepRegistry  │ │• Context    │ │ • LLM adapters │
└─────────────────┘ └─────────────┘ └─────────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
┌─────────────────┐ ┌─────────────┐ ┌─────────────────┐
│ pipeliner-events│ │pipeliner-   │ │  External       │
│                 │ │api          │ │  LLM Providers  │
│ • EventBus     │ │             │ │                 │
│ • AnyEvent     │ │• gRPC       │ │ • OpenAI        │
│ • Subscribers  │ │• REST       │ │ • Anthropic     │
└─────────────────┘ └─────────────┘ └─────────────────┘
```

### 3.2 Nuevos Crates Propuestos

| Crate | Propósito | Dependencias |
|-------|-----------|--------------|
| `pipeliner-agent` | Agent step execution | anthropic, openai, async-trait |
| `pipeliner-mcp` | MCP server + pipeline builder | tokio, mcp-server, pipeliner-core |
| `pipeliner-self-healing` | Self-healing observer | pipeliner-agent, pipeliner-events |

### 3.3 Modificaciones a Crates Existentes

#### `pipeliner-core/src/pipeline/mod.rs`

```rust
// NUEVO: Agregar variant a StepType
pub enum StepType {
    // ... existentes ...
    
    /// Agent execution step
    Agent {
        agent_id: String,
        prompt: String,
        model: Option<String>,
        tools: Vec<String>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        output_format: AgentOutputFormat,
    },
    
    /// Plan from natural language
    Plan {
        requirement: String,
        output_file: PathBuf,
    },
    
    /// Execute from plan
    FromPlan {
        file: PathBuf,
    },
}

pub enum AgentOutputFormat {
    Text,
    Json,
    JsonSchema(serde_json::Value),
}
```

#### `pipeliner-executor/src/runtime.rs`

```rust
// NUEVO: Handler para Agent step
impl StepExecutor {
    async fn execute_agent(
        &self,
        agent_id: &str,
        prompt: &str,
        model: Option<&str>,
        tools: &[String],
        step: &Step,
        context: &mut ExecutionContext,
    ) -> ExecutorResult<ExecutionStatus> {
        let agent_executor = context.get_agent_executor()?;
        let result = agent_executor.execute(agent_id, prompt, model, tools).await?;
        
        // Guardar output en contexto
        context.set_variable("AGENT_OUTPUT", result);
        
        Ok(ExecutionStatus::Success)
    }
}
```

---

## 4. Plan de Implementación por Fases

### Fase 1: Fundamentos (Semanas 1-2)

1. **Crear `pipeliner-agent` crate**
   - Definir `AgentProvider` trait
   - Implementar `AnthropicProvider`, `OpenAIProvider`
   - Mock provider para testing

2. **Extender `StepType`**
   - Agregar `Agent` variant
   - Serialization/deserialization

3. **Implementar `AgentExecutor`**
   - Integración con providers
   - Tool execution loop

### Fase 2: MCP Pipeline Builder (Semanas 3-4)

1. **Crear `pipeliner-mcp` crate**
   - MCP server setup
   - Tool definitions

2. **Implementar tools**
   - `pipeliner_list`
   - `pipeliner_create`
   - `pipeliner_run`
   - `pipeliner_get_status`

3. **Build from NL** (opcional fase 3)
   - Integración con LLM
   - Prompt engineering

### Fase 3: Self-Healing (Semanas 5-6)

1. **Crear `SelfHealingObserver`**
   - Policy config
   - Max attempts tracking

2. **Integrar con AgentExecutor**
   - Error analysis
   - Fix generation

3. **Telemetry**
   - Success rate
   - Self-heal effectiveness

### Fase 4: Multi-Agent Patterns (Semanas 7-8)

1. **Extender Matrix para agents**
2. **Implementar Router pattern**
3. **Debate/Majority voting**

---

## 5. Consideraciones de Diseño

### 5.1 API Stability

- Mantener backward compatibility con `StepType::Custom`
- `AgentProvider` trait permite swapping providers
- Versionar tool schemas en MCP

### 5.2 Security

- Sandboxing de agent execution
- Tool allowlist configurable
- Rate limiting en MCP
- Credentials en environment, no hardcoded

### 5.3 Observability

- Exportar metrics a Prometheus
- Distributed tracing con OpenTelemetry
- Event logging para debugging

### 5.4 Testing Strategy

- Unit tests con mock providers
- Integration tests con testcontainers
- E2E con real LLM (skip in CI)

---

## 6. Comparativa con Proyectos Similares

| Proyecto | Agentic Pipeline | MCP Native | Self-Healing |
|----------|------------------|------------|--------------|
| **Pipeliner (propuesto)** | ✅ | ✅ | ✅ |
| Jenkins | ❌ | ❌ | ⚠️ Plugins |
| GitHub Actions | ❌ | ❌ | ❌ |
| Argo Workflows | ⚠️ Scripts | ❌ | ❌ |
| Temporal | ⚠️ Workers | ❌ | ❌ |
| AWS Step Functions | ❌ | ❌ | ❌ |

---

## 7. Risks y Mitigaciones

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| LLM provider instability | Medium | High | Retry logic, fallback providers |
| Prompt injection | Low | High | Input sanitization, tool allowlist |
| Circular pipeline (agent builds agent) | Medium | Medium | Max recursion depth |
| Cost explosion | Medium | Medium | Token budgets, caching |
| Context window limits | High | Low | Chunking, summarization |

---

## 8. Conclusiones

### 8.1 Viabilidad

✅ **Alta viabilidad** - El código actual tiene buenos extension points:
- `StepFactory` + `StepRegistry`
- `PipelineObserver`
- `EventBus`

### 8.2 Orden Recomendado

1. **`pipeliner-agent`** - Primero, es la base
2. **Extender `StepType`** - Minimal changes
3. **`pipeliner-mcp`** -价值 inmediata para usuarios
4. **Self-healing** - Nice to have, puede ser iterativo

### 8.3 Próximos Pasos

1. Revisar esta propuesta
2. Decidir alcance de fase 1
3. Crear RFC en el repo
4. Empezar implementación

---

## 9. DSL de Rust - Estilo Jenkinsfile

### Sintaxis con Macros

Pipeliner usa macros estilo Jenkinsfile Declarativo:

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
        
        stage("Test") {
            steps {
                sh!("make test")
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

// AgentStep también es un macro
stage!("Code Review") {
    steps {
        agent(model: "claude-3-5-sonnet") {
            prompt = "Review PR #123 for security issues"
            tools = ["read_file", "grep"]
            skill = "code-review"
        }
    }
}
```

### Macros Requeridas

| Macro | Descripción |
|-------|-------------|
| `pipeline!` | Define pipeline completo |
| `stage!` | Stage individual |
| `steps!` | Bloque de steps |
| `agent!` | Step de agente LLM |
| `sh!`, `echo!` | Steps built-in |
| `post!` | Post-conditions |
| `always!`, `success!`, `failure!` | Condiciones |

### Decision: Macros vs Builder
- **Elegido**: Macros (Opción A)
- **Razón**: Más declarativo, similar a Jenkinsfile, menos boilerplate

## 10. Stack Elegido: Rig + Rig-MCP

### Crates de Rig

| Crate | Uso en Pipeliner |
|-------|------------------|
| `rig-core` | LLM providers (OpenAI, Anthropic, Gemini, etc.) |
| `rig-compose` | Sistema de tools y registry |
| `rig-mcp` | Integración con MCP servers existentes |
| `rig-resources` | Skills (carga de archivos .md) |

### Lo que Pipeliner define

1. **`AgentStep`** — tipo de step que usa rig Agent
2. **`AgentConfig`** — configuración del step (prompt, model, skill path)
3. **DSL macros** — `agent! {...}` que genera el step
4. **Tool bindings** — conectar StepRegistry con rig tools
5. **Skill loading** — leer .md y pasar a rig

### Lo que Rig provee

- Providers de LLM (20+ proveedores)
- Sistema de tools (rig-compose)
- Integración MCP (rig-mcp)
- Skills system (rig-resources)

## 10. Referencias

- [Pipeliner Core](../crates/pipeliner-core/src/)
- [Pipeliner Executor](../crates/pipeliner-executor/src/)
- [MCP Specification](https://modelcontextprotocol.io)
- [Anthropic Messages API](https://docs.anthropic.com/en/api/messages)
- [OpenAI Chat API](https://platform.openai.com/docs/api-reference)
