# Pipeliner → Bastion Worker Integration: Change Document

> **Date**: 2026-05-12
> **Status**: Implementation-ready
> **Target**: pipeliner (rustline) workspace
> **Consumer**: bastion-worker (`crates/bastion-worker/`) with feature `pipeline`

---

## 1. Executive Summary

Pipeliner needs **minimal changes** to integrate with Bastion's `bastion-worker`. The workspace already has the key infrastructure:

- `pipeliner-events` crate with `PipelineEvent` enum, `EventBus` trait, `LocalEventBus`, `EventEnvelope`
- `pipeliner-core` crate with `PipelineRuntime`, `LifecyclePhase`, `PipelineRunResult`
- `pipeliner-worker` crate (exists, can be adapted)

**The missing pieces are:**

1. Enrich `PipelineEvent::Created` to carry full pipeline structure (→ `pipeline_decl`)
2. Wire event emission into `LocalExecutor` execution flow
3. Add `PipelineExecutor::execute_with_events()` method (or builder pattern)
4. Ensure clean lib-only dependency path (bastion-worker uses only the library)

**Critical constraint:** The standalone CLI (`rustline` binary) MUST NOT break or gain Bastion dependencies. All event emission is through existing abstractions (`EventBus` trait).

---

## 2. Current Architecture

### 2.1 Workspace structure

```
pipeliner/
├── crates/
│   ├── pipeliner-core/         # Pipeline, Stage, Step, AgentType, Environment, Validate, PipelineRuntime
│   ├── pipeliner-events/       # PipelineEvent, EventBus, LocalEventBus, EventEnvelope, StageMarker
│   ├── pipeliner-executor/     # Executor implementations
│   ├── pipeliner-worker/       # Worker crate (exists)
│   ├── pipeliner-api/          # API layer
│   ├── pipeliner-cli/          # CLI binary
│   ├── pipeliner-library/      # Library loading
│   ├── pipeliner-infrastructure/ # Container runtimes (Docker, Podman, K8s)
│   ├── pipeliner-macros/       # Procedural macros
│   └── pipeliner-steps-*/      # Step implementations (artifact, git, maven, etc.)
├── src/                        # Root binary (rustline CLI) + legacy executor code
└── Cargo.toml                  # Workspace root
```

### 2.2 Existing event types (`pipeliner-events`)

```rust
// crates/pipeliner-events/src/types/base.rs
pub enum PipelineEvent {
    Created { pipeline_id: Uuid, name: String },
    Started { pipeline_id: Uuid, execution_id: Uuid, stage: String },
    StageStarted { pipeline_id: Uuid, execution_id: Uuid, stage_name: String },
    StageCompleted { pipeline_id: Uuid, execution_id: Uuid, stage_name: String, result: String },
    StepStarted { pipeline_id: Uuid, execution_id: Uuid, stage_name: String, step_name: String },
    StepCompleted { pipeline_id: Uuid, execution_id: Uuid, stage_name: String, step_name: String, output: Option<String> },
    Completed { pipeline_id: Uuid, execution_id: Uuid, result: String },
    Failed { pipeline_id: Uuid, execution_id: Uuid, error: String },
    Cancelled { pipeline_id: Uuid, execution_id: Uuid, reason: String },
}
```

### 2.3 Existing EventBus (`pipeliner-events`)

```rust
// crates/pipeliner-events/src/event_bus/mod.rs
#[async_trait]
pub trait EventBus: Send + Sync {
    type Error: std::fmt::Debug + std::fmt::Display;
    async fn publish(&self, event: EventEnvelope) -> Result<(), Self::Error>;
    async fn subscribe(&self, handler: Arc<dyn EventHandler>) -> Result<(), Self::Error>;
    async fn unsubscribe(&self, handler_id: &Uuid) -> Result<(), Self::Error>;
}

pub struct LocalEventBus { /* broadcast channel + DashMap handlers */ }
```

### 2.4 Current LocalExecutor (`src/executor/local.rs`)

The executor uses `tracing::info!` for logging but does NOT emit `PipelineEvent`s through the `EventBus`. This is the primary gap.

### 2.5 Existing PipelineRuntime (`pipeliner-core`)

```rust
pub enum LifecyclePhase {
    Init, LoadLibraries, SetupEngine, LoadSourceCode, BindSteps, Execute, Completed, Failed,
}

pub struct PipelineRunResult {
    pub success: bool,
    pub duration_ms: u64,
    pub stages_executed: usize,
    pub steps_executed: usize,
    pub error: Option<String>,
}
```

---

## 3. Required Changes

### 3.1 [ADDITIVE] Enrich `PipelineEvent::Created` → `PipelineDecl`

**File**: `crates/pipeliner-events/src/types/base.rs`

**Why**: Bastion dashboard needs the FULL pipeline structure upfront to draw the graph before execution starts. Current `Created` only has `name`.

**Change**: Add a new variant `PipelineDecl` that carries the complete pipeline structure.

```rust
// ADD to PipelineEvent enum:

/// Pipeline structure declaration for external visualization.
/// Emitted BEFORE execution starts so consumers can project the graph.
PipelineDecl {
    pipeline_id: Uuid,
    execution_id: Uuid,
    /// Pipeline name
    name: String,
    /// Serialized pipeline structure (stages, steps, conditions, DAG)
    /// as JSON for forward-compatible consumption.
    structure: PipelineStructure,
},

// ADD new struct:

/// Serializable pipeline structure for external consumers (dashboard, Bastion).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStructure {
    pub stages: Vec<StageStructure>,
}

/// Serializable stage structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageStructure {
    pub name: String,
    pub steps: Vec<StepStructure>,
    pub has_parallel: bool,
    pub has_matrix: bool,
    pub when_condition: Option<String>, // Human-readable description
}

/// Serializable step structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStructure {
    pub name: Option<String>,
    pub step_type: String,           // "shell", "echo", "retry", etc.
    pub command: Option<String>,     // For shell steps
}
```

Update `event_type()` match:

```rust
Self::PipelineDecl { .. } => "PipelineDecl",
```

Update `aggregate_id()` match:

```rust
Self::PipelineDecl { pipeline_id, .. } => pipeline_id,
```

**Impact**: [ADDITIVE] No breaking change. New enum variant. Existing code doesn't match exhaustively.

---

### 3.2 [ADDITIVE] Add `pipeline_structure()` method to `Pipeline`

**File**: `crates/pipeliner-core/src/pipeline/` (wherever `Pipeline` is defined)

**Why**: Need a clean way to extract pipeline structure for the `PipelineDecl` event.

```rust
impl Pipeline {
    /// Export the pipeline structure for external visualization.
    /// Used to emit `PipelineDecl` events before execution starts.
    pub fn structure(&self) -> PipelineStructure {
        PipelineStructure {
            stages: self.stages.iter().map(|stage| StageStructure {
                name: stage.name.clone(),
                steps: stage.steps.iter().map(|step| StepStructure {
                    name: step.name.clone(),
                    step_type: match &step.step_type {
                        StepType::Shell { .. } => "shell",
                        StepType::Echo { .. } => "echo",
                        StepType::Retry { .. } => "retry",
                        StepType::Timeout { .. } => "timeout",
                        StepType::Stash { .. } => "stash",
                        StepType::Unstash { .. } => "unstash",
                        StepType::Input { .. } => "input",
                        StepType::Dir { .. } => "dir",
                        StepType::Wait { .. } => "wait",
                    }.to_string(),
                    command: match &step.step_type {
                        StepType::Shell { command } => Some(command.clone()),
                        _ => None,
                    },
                }).collect(),
                has_parallel: !stage.parallel.is_empty(),
                has_matrix: stage.matrix.is_some(),
                when_condition: stage.when.as_ref().map(|w| format!("{:?}", w)),
            }).collect(),
        }
    }
}
```

**Impact**: [ADDITIVE] New method on existing type. No breaking change.

**Dependency**: `PipelineStructure`, `StageStructure`, `StepStructure` from `pipeliner-events` (or define in `pipeliner-core` and re-export).

---

### 3.3 [ADDITIVE] Wire event emission into `LocalExecutor`

**File**: `src/executor/local.rs` (or `crates/pipeliner-executor/` if executor lives there)

**Why**: The executor must emit events through the `EventBus` at each lifecycle point.

**Approach**: Add optional `EventBus` to `LocalExecutor` via builder pattern.

```rust
use std::sync::Arc;
use pipeliner_events::{EventBus, EventEnvelope, EventMetadata, AnyEvent};
use pipeliner_events::types::{PipelineEvent, PipelineStructure};

pub struct LocalExecutor {
    config: ExecutorConfig,
    /// Optional event bus for structured event emission.
    /// When None, only tracing logs are emitted (current behavior).
    event_bus: Option<Arc<dyn EventBus<Error = pipeliner_events::event_bus::EventBusError>>>,
}

impl LocalExecutor {
    /// Sets the event bus for structured event emission.
    /// Without this, the executor only uses tracing (backward compatible).
    pub fn with_event_bus(
        mut self,
        bus: Arc<dyn EventBus<Error = pipeliner_events::event_bus::EventBusError>>,
    ) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Helper: emit event if bus is configured.
    async fn emit(&self, event: PipelineEvent) {
        if let Some(bus) = &self.event_bus {
            let envelope = EventEnvelope::new(
                AnyEvent::Pipeline(event),
                EventMetadata::new("local-executor"),
            );
            let _ = bus.publish(envelope).await;
        }
    }
}
```

**Insertion points in `execute()` method:**

```rust
impl PipelineExecutor for LocalExecutor {
    fn execute(&self, pipeline: &Pipeline) -> Result<StageResult, PipelineError> {
        let pipeline_id = uuid::Uuid::new_v4();
        let execution_id = uuid::Uuid::new_v4();

        // 1. Emit PipelineDecl (structure for dashboard)
        // NOTE: emit() is async but execute() is sync.
        // See Section 3.5 for async resolution.
        self.emit_sync(PipelineEvent::PipelineDecl {
            pipeline_id,
            execution_id,
            name: pipeline.name.clone().unwrap_or_default(),
            structure: pipeline.structure(),
        });

        // 2. Emit PipelineStarted
        self.emit_sync(PipelineEvent::Started {
            pipeline_id,
            execution_id,
            stage: "all".to_string(),
        });

        // 3. Execute stages (with events)
        for stage in &pipeline.stages {
            let stage_name = stage.name.clone();

            self.emit_sync(PipelineEvent::StageStarted {
                pipeline_id,
                execution_id,
                stage_name: stage_name.clone(),
            });

            let start = Instant::now();
            let result = self.execute_stage(stage, &context)?;
            let duration = start.elapsed();

            // 4. Emit StageCompleted
            self.emit_sync(PipelineEvent::StageCompleted {
                pipeline_id,
                execution_id,
                stage_name: stage_name.clone(),
                result: match result {
                    StageResult::Success => "SUCCESS".to_string(),
                    StageResult::Failure => "FAILURE".to_string(),
                    _ => format!("{:?}", result),
                },
            });

            context.record_stage_result(&stage_name, result);
            // ... failure handling ...
        }

        // 5. Emit PipelineCompleted
        self.emit_sync(PipelineEvent::Completed {
            pipeline_id,
            execution_id,
            result: "SUCCESS".to_string(),
        });

        Ok(StageResult::Success)
    }
}
```

**Similar instrumentation needed in `execute_step()`:**

```rust
fn execute_step(&self, step: &Step, context: &PipelineContext) -> Result<(), PipelineError> {
    // Before: emit StepStarted
    // After: emit StepCompleted with output/duration
    // On error: emit via StageMarker::Error pattern
}
```

**Impact**: [ADDITIVE] New optional field + method. When `event_bus` is `None`, behavior is identical to current code.

---

### 3.4 [INTERNAL] Resolve async/sync mismatch

**Problem**: `EventBus::publish()` is `async`, but `PipelineExecutor::execute()` is synchronous.

**Options**:

**Option A: Make execute() async (RECOMMENDED)**

```rust
// pipeliner-events/src/types/base.rs or executor trait
#[async_trait]
pub trait PipelineExecutor: Send + Sync {
    async fn execute(&self, pipeline: &Pipeline) -> Result<StageResult, PipelineError>;
    async fn validate(&self, pipeline: &Pipeline) -> Result<(), ValidationError>;
    async fn dry_run(&self, pipeline: &Pipeline) -> Result<StageResult, PipelineError>;
    fn capabilities(&self) -> ExecutorCapabilities;
    fn health_check(&self) -> HealthStatus;
}
```

This is the cleanest approach but is [BREAKING] for the trait.

**Option B: Sync wrapper with tokio runtime**

```rust
fn emit_sync(&self, event: PipelineEvent) {
    if let Some(bus) = &self.event_bus {
        let rt = tokio::runtime::Handle::current();
        let envelope = EventEnvelope::new(
            AnyEvent::Pipeline(event),
            EventMetadata::new("local-executor"),
        );
        let _ = rt.block_on(bus.publish(envelope));
    }
}
```

This is [ADDITIVE] but requires a tokio runtime to be available (which it is in bastion-worker).

**Option C: Channel-based (decouple emit from publish)**

```rust
fn emit_sync(&self, event: PipelineEvent) {
    if let Some(tx) = &self.event_tx {
        let _ = tx.send(event); // mpsc::unbounded channel
    }
}
// Background task drains channel → bus.publish()
```

This is [ADDITIVE] and decouples sync emission from async publishing.

**Recommendation**: Option B for v1 (simplest, works in bastion-worker context). Option A for v2 (proper async trait).

---

### 3.5 [ADDITIVE] Add timing data to `StepCompleted` event

**File**: `crates/pipeliner-events/src/types/base.rs`

**Why**: Bastion dashboard needs duration per step for the timeline visualization.

**Change**: Add `duration_ms` to `StepCompleted`:

```rust
// BEFORE:
StepCompleted {
    pipeline_id: Uuid,
    execution_id: Uuid,
    stage_name: String,
    step_name: String,
    output: Option<String>,
},

// AFTER:
StepCompleted {
    pipeline_id: Uuid,
    execution_id: Uuid,
    stage_name: String,
    step_name: String,
    output: Option<String>,
    /// Duration in milliseconds. [ADDITIVE]
    duration_ms: u64,
    /// Exit code of the command. [ADDITIVE]
    exit_code: Option<i32>,
},
```

**Impact**: [BREAKING] for consumers that pattern-match `StepCompleted` exhaustively. Within the pipeliner workspace this is manageable since the only consumer is the event bus.

---

### 3.6 [ADDITIVE] Add `duration_ms` to `StageCompleted` event

**File**: `crates/pipeliner-events/src/types/base.rs`

```rust
// BEFORE:
StageCompleted {
    pipeline_id: Uuid,
    execution_id: Uuid,
    stage_name: String,
    result: String,
},

// AFTER:
StageCompleted {
    pipeline_id: Uuid,
    execution_id: Uuid,
    stage_name: String,
    result: String,
    /// Duration in milliseconds. [ADDITIVE]
    duration_ms: u64,
},
```

---

### 3.7 [ADDITIVE] Ensure clean library-only dependency

**File**: `Cargo.toml` (root) + `crates/pipeliner-core/Cargo.toml`

**Why**: bastion-worker should depend only on `pipeliner-core` + `pipeliner-events` (and optionally `pipeliner-executor`), NOT on CLI-only deps like `clap`.

**Verify**:
- `pipeliner-core` has no `clap` dependency ✅ (already true)
- `pipeliner-events` has no `clap` dependency ✅ (already true)
- `pipeliner-executor` has no `clap` dependency (verify)

**bastion-worker will depend on**:
```toml
[dependencies.pipeliner-core]
path = "../../../pipeliner/crates/pipeliner-core"

[dependencies.pipeliner-events]
path = "../../../pipeliner/crates/pipeliner-events"

[dependencies.pipeliner-executor]
path = "../../../pipeliner/crates/pipeliner-executor"
optional = true
```

**Impact**: [INTERNAL] No change to pipeliner itself, just documenting the dependency path.

---

## 4. Event Catalog — Mapping to Bastion Proto

| Pipeliner `PipelineEvent` | Bastion `PipelineEvent.event_type` | Notes |
|---|---|---|
| `PipelineDecl` | `"pipeline_decl"` | **NEW** — full structure for dashboard graph |
| `Started` | `"pipeline_started"` | Existing |
| `StageStarted` | `"stage_started"` | Existing |
| `StageCompleted` | `"stage_finished"` | Existing, add `duration_ms` |
| `StepStarted` | `"step_started"` | Existing |
| `StepCompleted` | `"step_finished"` | Existing, add `duration_ms`, `exit_code` |
| `Failed` | `"step_failed"` / `"pipeline_finished"` | Map based on context |
| `Completed` | `"pipeline_finished"` | Existing |
| `Cancelled` | `"pipeline_finished"` with cancelled status | Existing, bonus |

The bridge in bastion-worker converts:
```
EventEnvelope { event: AnyEvent::Pipeline(pipeline_event), metadata }
  → bastion_worker::PipelineEvent {
      event_type: pipeline_event.event_type().to_string(),
      pipeline_run_id: metadata.correlation_id.map(|id| id.to_string()).unwrap_or_default(),
      stage_name: extract_stage_name(&pipeline_event),
      step_name: extract_step_name(&pipeline_event),
      step_index: ...,
      payload: serde_json::to_vec(&pipeline_event).unwrap(),
      timestamp_ms: metadata.timestamp.timestamp_millis() as u64,
      labels: ...,
    }
```

---

## 5. Code Examples

### 5.1 How bastion-worker uses Pipeliner

```rust
// In bastion-worker, behind feature "pipeline"
use pipeliner_core::{Pipeline, Validate};
use pipeliner_events::{EventBus, EventEnvelope, AnyEvent, LocalEventBus};
use std::sync::Arc;

// 1. Create event bus
let bus = Arc::new(LocalEventBus::new());

// 2. Subscribe a handler that bridges to gRPC
let grpc_bridge = Arc::new(BastionGrpcBridge::new(grpc_sender.clone()));
bus.subscribe(grpc_bridge).await?;

// 3. Load pipeline from file
let pipeline = Pipeline::from_file("/workspace/pipeline.yaml")?;
pipeline.validate()?;

// 4. Create executor with event bus
let executor = LocalExecutor::new()
    .with_cwd("/workspace")
    .with_event_bus(bus);

// 5. Execute
let result = executor.execute(&pipeline)?;
```

### 5.2 BastionGrpcBridge (EventHandler impl)

```rust
/// Bridges pipeliner events to bastion-worker gRPC PipelineEvent messages.
struct BastionGrpcBridge {
    sender: mpsc::Sender<WorkerMessage>,
}

#[async_trait]
impl EventHandler for BastionGrpcBridge {
    async fn handle(&self, envelope: &EventEnvelope) {
        if let AnyEvent::Pipeline(pe) = &envelope.event {
            let msg = WorkerMessage {
                command_id: String::new(),
                payload: Some(Payload::PipelineEvent(PipelineEvent {
                    event_type: pe.event_type().to_string(),
                    pipeline_run_id: envelope.metadata.correlation_id
                        .map(|id| id.to_string())
                        .unwrap_or_default(),
                    stage_name: extract_stage(pe),
                    step_name: extract_step(pe),
                    step_index: 0,
                    payload: serde_json::to_vec(pe).unwrap_or_default(),
                    timestamp_ms: envelope.metadata.timestamp.timestamp_millis() as u64,
                    labels: HashMap::new(),
                })),
            };
            let _ = self.sender.send(msg).await;
        }
    }
}
```

### 5.3 Standalone CLI continues to work unchanged

```rust
// src/main.rs (unchanged)
fn main() {
    let executor = LocalExecutor::new(); // No event_bus → only tracing logs
    let result = executor.execute(&pipeline);
    // Works exactly as before
}
```

---

## 6. Migration Path

### Order of implementation:

1. **`pipeliner-events`**: Add `PipelineDecl` variant + `PipelineStructure` types → [ADDITIVE]
2. **`pipeliner-core`**: Add `Pipeline::structure()` method → [ADDITIVE]
3. **`pipeliner-events`**: Add `duration_ms`/`exit_code` to `StepCompleted`, `StageCompleted` → [BREAKING but internal]
4. **`pipeliner-executor`** or `src/executor/`: Wire event emission into `LocalExecutor` → [ADDITIVE]
5. **Verify**: All existing tests pass, CLI works unchanged

### Testing strategy:

- Unit test: `Pipeline::structure()` returns correct structure
- Unit test: `LocalExecutor` with `LocalEventBus` emits correct event sequence
- Integration test: `PipelineDecl` → `Started` → `StageStarted` → `StepStarted` → `StepCompleted` → `StageCompleted` → `Completed`
- Regression test: `LocalExecutor` without event bus works identically to current behavior

---

## 7. Files Changed Summary

| File | Change | Type | Risk |
|---|---|---|---|
| `crates/pipeliner-events/src/types/base.rs` | Add `PipelineDecl` variant, `PipelineStructure`/`StageStructure`/`StepStructure` structs, add `duration_ms`/`exit_code` to `StepCompleted`, `duration_ms` to `StageCompleted` | ADDITIVE + BREAKING (internal) | Low |
| `crates/pipeliner-events/src/types/mod.rs` | Re-export new types | ADDITIVE | None |
| `crates/pipeliner-events/src/lib.rs` | Re-export new types | ADDITIVE | None |
| `crates/pipeliner-core/src/pipeline/*.rs` | Add `Pipeline::structure()` method | ADDITIVE | Low |
| `src/executor/local.rs` (or `crates/pipeliner-executor/`) | Add `event_bus` field, `with_event_bus()` builder, `emit_sync()` helper, instrument execute/stage/step methods | ADDITIVE | Medium |
| No changes to CLI, no new dependencies on Bastion | — | — | — |

**Total: ~5 files, mostly additive changes.**

---

## 8. `pipeliner-worker` crate — not discarded, not required for v1

The `pipeliner-worker` crate is a **job queue + worker pool + scheduler** — it is NOT a sandbox agent competing with `bastion-worker`. It's an internal orchestration layer within Pipeliner.

```
bastion-worker (sandbox process, gRPC, JNLP transport)
  └── feature "pipeline"
      └── pipeliner-executor (runs stages/steps)
          └── pipeliner-worker (scheduler + job queue) ← optional, for advanced scheduling
              └── pipeliner-core (Pipeline, Stage, Step)
              └── pipeliner-events (PipelineEvent, EventBus)
```

For v1 (sequential stage execution via `LocalExecutor`), `pipeliner-worker` is not needed.
If Pipeliner later needs priority scheduling, worker pools, or job queuing inside the sandbox,
`pipeliner-worker` becomes useful as a library dependency of the `pipeline` feature.

**Decision**: Keep the crate. Don't depend on it for v1 integration. Available if scheduling complexity grows.

---

## 9. What Does NOT Need to Change

- `PipelineExecutor` trait signature (can add `with_event_bus()` as builder method on `LocalExecutor`)
- `Pipeline` struct (just adding a method)
- `Stage`, `Step`, `StepType` (no changes needed)
- `PipelineContext` (no changes needed)
- CLI binary (`src/main.rs`, `crates/pipeliner-cli/`) (zero changes)
- `Cargo.toml` dependencies (no new external crates needed)
- `StageMarker` system (continues working alongside event bus)

---

*Document version: 1.0, 2026-05-12.*
*See also: Bastion strategy doc `docs/propuestas/007-bastion-pipeliner-integration-strategy.md`*
