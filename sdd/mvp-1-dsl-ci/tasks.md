# Tasks: MVP-1 DSL CI Features

## Phase 1: Foundation

### Batch 1: EnvSpec Type & Macro Parsing
- [ ] 1.1 Create `crates/pipeliner-core/src/spec/env_spec.rs` with `EnvSpec` struct (HashMap<String,String>, `vars` field, `new()`, `with_var()`, `merge()` methods)
- [ ] 1.2 Add `EnvSpec` export to `crates/pipeliner-core/src/spec/mod.rs`
- [ ] 1.3 Modify `PipelineSpec` in `pipeline_spec.rs` — add `env: Option<EnvSpec>` field with `#[serde(skip_serializing_if = "Option::is_none")]`
- [ ] 1.4 Modify `StageSpec` in `stage_spec.rs` — add `env: Option<EnvSpec>` field with same serde annotation
- [ ] 1.5 Add `parse_env_block()` function to `pipeline_parsing.rs` — parse `env { KEY = "value"; }` syntax
- [ ] 1.6 Modify `parse_pipeline()` to handle optional `env` block before `stages`

### Batch 2: StepSpec Variants
- [ ] 2.1 Create `DirStepSpec` struct in `step_spec.rs` — fields: `path: String`, `steps: Vec<StepSpec>`
- [ ] 2.2 Create `WithEnvStepSpec` struct in `step_spec.rs` — fields: `env: EnvSpec`, `steps: Vec<StepSpec>`
- [ ] 2.3 Create `LetOutputStepSpec` struct in `step_spec.rs` — fields: `var_name: String`, `inner: Box<StepSpec>`
- [ ] 2.4 Add `Dir`, `WithEnv`, `LetOutput` variants to `StepSpec` enum with `#[serde(tag = "type")]` — ensure JSON: `{"type":"dir",...}`
- [ ] 2.5 Add `type_name()`, `label()`, `allow_failure()` to `StepSpecExt` for new variants

### Batch 3: OptionsSpec
- [ ] 3.1 Create `crates/pipeliner-core/src/spec/options_spec.rs` — `Duration` struct (`minutes: u64`, `seconds: u64`), `Duration::minutes()`, `Duration::seconds()`
- [ ] 3.2 Create `OptionsSpec` struct — fields: `timeout: Option<Duration>`, `retry: u32` with defaults
- [ ] 3.3 Add `options: Option<OptionsSpec>` to `StageSpec` with serde annotation
- [ ] 3.4 Export `OptionsSpec`, `Duration` from `spec/mod.rs`

## Phase 2: Macro Parsing (pipeliner-macros)

### Batch 4: New Step Parsing
- [ ] 4.1 Extend `StepDef` enum — add `Dir(String, Vec<StepDef>)`, `WithEnv(Vec<(String,String)>, Vec<StepDef>)`, `LetOutput(String, Box<StepDef>)`
- [ ] 4.2 Add `parse_dir_step()` function — parse `dir "path" { steps }`
- [ ] 4.3 Add `parse_with_env_step()` function — parse `with_env { KEY = "value"; } { steps }`
- [ ] 4.4 Add `parse_let_output_step()` function — parse `let_output VAR = step`
- [ ] 4.5 Update `parse_steps()` to dispatch to new step parsers

### Batch 5: Extended sh Syntax
- [ ] 5.1 Add `parse_shell_block()` function — parse extended `sh { script "cmd"; label "x"; capture_stdout true; }`
- [ ] 5.2 Add `parse_shell_raw()` function — parse `sh raw "command"` syntax
- [ ] 5.3 Update `parse_step()` to handle extended sh block and raw syntax

### Batch 6: Code Generation
- [ ] 6.1 Update `generate_step_spec()` to handle `Dir`, `WithEnv`, `LetOutput` variants
- [ ] 6.2 Update `build_pipeline_spec()` to include `env` field in generated PipelineSpec
- [ ] 6.3 Update `generate_stage_execution()` to include `env` and `options` fields

## Phase 3: Executor (pipeliner-runtime)

### Batch 7: EnvContext & Interpolation
- [ ] 7.1 Create `EnvContext` struct in `local_executor.rs` — `HashMap<String,String>` wrapper with `new()`, `with_vars()`, `get()`, `merge()` methods
- [ ] 7.2 Implement `interpolate(script: &str, env: &EnvContext) -> String` — replace `${VAR}` and `$VAR`, handle `$$` escape, undefined vars → empty string
- [ ] 7.3 Add `env_context: EnvContext` field to `LocalExecutor` struct
- [ ] 7.4 Modify `execute()` to merge `spec.env` into `env_context` before stage execution

### Batch 8: Step Executors
- [ ] 8.1 Add `execute_dir_step()` — save current dir, change to `spec.path`, execute inner steps, restore dir (even on error)
- [ ] 8.2 Add `execute_with_env_step()` — clone env_context, merge `spec.env`, execute inner steps
- [ ] 8.3 Add `execute_let_output_step()` — execute inner step, capture stdout if `capture_stdout: true`, store in env_context
- [ ] 8.4 Modify `execute_step()` match to handle `Dir`, `WithEnv`, `LetOutput` variants
- [ ] 8.5 Update `StepSpecExt` impl for new variants

### Batch 9: Shell Execution Updates
- [ ] 9.1 Modify `execute_shell_step()` — call `interpolate()` when `spec.interpolation == Pipeliner`, skip when `Raw`
- [ ] 9.2 Implement timeout in `execute_stage()` — use `tokio::time::timeout` with duration from `stage_spec.options.timeout`

### Batch 10: Retry Logic
- [ ] 10.1 Implement retry in `execute_stage()` — read `stage_spec.options.retry`, loop with exponential backoff (1s, 2s, 4s, ...)
- [ ] 10.2 Emit `StageRetry` event on each retry attempt

## Phase 4: Events & Reporting

### Batch 11: JsonlEventWriter
- [ ] 11.1 Create `JsonlEventWriter` struct in `events.rs` — implements `EventEmitter`, wraps `File`
- [ ] 11.2 Implement `emit()` — serialize `PipelineEvent` to JSON, write as line to file
- [ ] 11.3 Export `JsonlEventWriter` from `runtime/lib.rs`

### Batch 12: Enhanced Report
- [ ] 12.1 Add `env_snapshot: Option<EnvSpec>` field to `ExecutionReport` in `report.rs`
- [ ] 12.2 Add `output_length: Option<usize>` to `StepTiming` — for let_output steps
- [ ] 12.3 Add `label: Option<String>` to `StepTiming` — ensure step labels captured
- [ ] 12.4 Update `ExecutionReport::from_execution_result()` to capture environment snapshot

## Phase 5: Testing

### Batch 13: Unit Tests
- [ ] 13.1 Test `EnvSpec::merge()` — later vars override earlier
- [ ] 13.2 Test `interpolate()` — `${VAR}`, `$VAR`, `$$` escape, undefined vars
- [ ] 13.3 Test `StepSpec` JSON roundtrip for `Dir`, `WithEnv`, `LetOutput` variants
- [ ] 13.4 Test `Duration::minutes()` and `Duration::seconds()`
- [ ] 13.5 Test `OptionsSpec` serde serialization/deserialization

### Batch 14: Macro Parsing Tests (trybuild)
- [ ] 14.1 Test `env { KEY = "value"; }` parsing
- [ ] 14.2 Test `dir "path" { sh "cmd"; }` parsing
- [ ] 14.3 Test `with_env { K = "v"; } { steps }` parsing
- [ ] 14.4 Test `let_output VAR = sh "cmd"` parsing
- [ ] 14.5 Test `sh { script "cmd"; label "x"; }` extended syntax
- [ ] 14.6 Test `sh raw "cmd"` syntax

### Batch 15: Integration Tests
- [ ] 15.1 Test env propagation: pipeline → stage → step (with_env wins)
- [ ] 15.2 Test `dir` step changes working directory
- [ ] 15.3 Test `with_env` merges and restores environment
- [ ] 15.4 Test `let_output` captures and stores variable
- [ ] 15.5 Test timeout kills stage after duration
- [ ] 15.6 Test retry executes stage multiple times on failure
- [ ] 15.7 Test `sh raw` passes variables literally to shell
- [ ] 15.8 Test interpolation with captured variables

### Batch 16: E2E Tests
- [ ] 16.1 Full pipeline with `pipeline!` macro using all MVP-1 features — env, dir, with_env, let_output, options
- [ ] 16.2 Verify events.jsonl written correctly with all event types
- [ ] 16.3 Verify enhanced report.json contains environment snapshot

---

## File Changes Summary

| File | Action |
|------|--------|
| `pipeliner-core/src/spec/env_spec.rs` | Create |
| `pipeliner-core/src/spec/options_spec.rs` | Create |
| `pipeliner-core/src/spec/step_spec.rs` | Modify — add variants |
| `pipeliner-core/src/spec/stage_spec.rs` | Modify — add env/options |
| `pipeliner-core/src/spec/pipeline_spec.rs` | Modify — add env |
| `pipeliner-core/src/spec/mod.rs` | Modify — exports |
| `pipeliner-macros/src/pipeline_parsing.rs` | Modify — new parsers |
| `pipeliner-runtime/src/local_executor.rs` | Modify — EnvContext, step handlers |
| `pipeliner-runtime/src/events.rs` | Modify — JsonlEventWriter |
| `pipeliner-runtime/src/report.rs` | Modify — enhanced fields |
| `pipeliner-runtime/src/lib.rs` | Modify — exports |
