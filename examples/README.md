# Pipeliner — Ejemplos de Pipelines en Rust DSL

> ⚠️ **IMPORTANTE**: Pipeliner usa **Rust DSL** para definir pipelines. Los archivos `.json` son solo para serialización. NO uses YAML para definir pipelines.

## Cómo Definir un Pipeline

Los pipelines son **código Rust** que usa las structs de `pipeliner-core`:

```rust
use pipeliner_core::{Pipeline, Stage, Step};
use pipeliner_macros::{sh, echo};

let pipeline = Pipeline::new()
    .with_name("Mi Pipeline")
    .with_stage(
        Stage::new("Build")
            .with_step(sh!("cargo build --release"))
            .with_step(echo!("Build done!"))
    );
```

## Ejemplos Disponibles

### 1. Build + Test CI
**Archivo**: `pipelines/build_and_test.rs`

Pipeline CI típico que compila, testea y verifica el binary.

```bash
cargo run --example build_and_test
```

### 2. Git Clone + Build + Test
**Archivo**: `pipelines/git_clone_build.rs`

Clona un repositorio, lo compila y ejecuta tests.

```bash
REPO_URL=https://github.com/spring-projects/spring-petclinic.git \
  cargo run --example git_clone_build
```

### 3. Agent Pipeline
**Archivo**: `crates/pipeliner-agent/examples/agent-pipeline.rs`

Usa LLM como step del pipeline.

```bash
cargo run -p pipeliner-agent --example agent-pipeline
```

## Estructura de un Pipeline

```rust
Pipeline::new()
    .with_name("Nombre")
    .with_stage(Stage::new("Nombre Stage")
        .with_step(Step::shell("comando"))
        .with_step(Step::echo("mensaje"))
    )
```

## Macros Disponibles

| Macro | Descripción |
|-------|-------------|
| `sh!("comando")` | Ejecuta comando shell |
| `echo!("msg")` | Imprime mensaje |
| `stage!("nombre", vec![...])` | Crea stage con steps |

## Pasos Disponibles

| Tipo | Constructor |
|------|-------------|
| Shell | `Step::shell("cmd")` o `sh!("cmd")` |
| Echo | `Step::echo("msg")` o `echo!("msg")` |
| Agent | `Step::agent(config)` |

## Serialización

Los pipelines se pueden serializar a JSON:

```rust
let json = serde_json::to_string_pretty(&pipeline).unwrap();
```
