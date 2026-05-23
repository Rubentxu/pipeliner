# Pipeliner - Plan de Reestructuración y Roadmap

> Basado en: `specs/deep-research.md` (entrevista de diseño de ~14,800 líneas)
> Fecha: 2026-05-16
> Estado: **DISEÑO CERRADO - IMPLEMENTACIÓN PENDIENTE**

---

## Resumen Ejecutivo

El diseño actual de Pipeliner diverge significativamente del diseño acordado en `deep-research.md`. Este documento establece:

1. **Análisis deGap**: Qué existe vs. qué debería existir
2. **Plan de Reestructuración**: Cómo reorganizar los crates y el código
3. **Roadmap de Implementación**: Fases MVP-0 a MVP-4

---

## PARTE 1: ANÁLISIS DE GAP

### 1.1 Estructura Actual de Crates

```
crates/
├── pipeliner-core/          # Tipos domain, Pipeline, Stage, Step (ACTUAL)
├── pipeliner-cli/            # CLI principal
├── pipeliner-macros/         # Macros pipeline! (SINTAXIS INCORRECTA)
├── pipeliner-executor/       # Ejecución de pipelines
├── pipeliner-worker/        # Worker para ejecución distribuida
├── pipeliner-events/         # Sistema de eventos
├── pipeliner-api/            # API REST/gRPC
├── pipeliner-infrastructure/ # Integración con Kubernetes, Docker, etc.
├── pipeliner-library/        # Librerías compartidas
├── pipeliner-script/         # Ejecución de scripts Rust (INCOMPLETO)
├── pipeliner-steps-core/     # Steps básicos
├── pipeliner-steps-git/      # Steps para git
├── pipeliner-steps-http/     # Steps para HTTP
├── pipeliner-steps-tooling/  # Steps para herramientas
├── pipeliner-steps-maven/    # Steps para Maven
├── pipeliner-steps-gradle/   # Steps para Gradle
├── pipeliner-steps-container/# Steps para contenedores
├── pipeliner-steps-helm/     # Steps para Helm
├── pipeliner-steps-scanner/  # Steps para escaneo
├── pipeliner-steps-policy/   # Steps para políticas
├── pipeliner-steps-artifact/ # Steps para artefactos
├── pipeliner-steps-notify/   # Steps para notificaciones
└── pipeliner-mcp/           # MCP server
```

### 1.2 Estructura Objetivo (según deep-research.md)

```
pipeliner/
├── pipeliner-cli/              # CLI: run, check, dry-run, graph, gc
├── pipeliner-core/            # PipelineSpec, StageSpec, StepSpec (TIPOS)
├── pipeliner-macros/          # pipeline! procedural macro (DSL)
├── pipeliner-runtime/         # Runtime local (NUEVO - separar de executor)
├── pipeliner-protocol/        # Serialización JSON, --describe protocol
├── pipeliner-script/          # Script runner (rust-script-like)
├── pipeliner-credentials/     # Sistema de credenciales (NUEVO)
└── pipeliner-steps/           # Steps built-in
    ├── steps-core/
    ├── steps-notify/
    └── ... (reorganizar steps existentes)
```

### 1.3 Gap Analysis Detallado

| Componente | Estado Actual | Estado Objetivo | Prioridad |
|------------|---------------|----------------|-----------|
| DSL `pipeline!` | Sintaxis `stage!("Name")`, `sh!("cmd")` | `stage "Name" { steps { sh "cmd" } }` | CRÍTICA |
| PipelineSpec | Mezclado con runtime | Serializable, separado | CRÍTICA |
| `--describe` | No existe | Protocolo stdout JSON | CRÍTICA |
| Cargo caching | Incompleto | rust-script-like cache | ALTA |
| `env` | Basic | Literales + secrets + with_env | ALTA |
| `parallel` | Parcial | Recursivo con fail_fast | MEDIA |
| `post` | Básico | always/success/failure/aborted/cleanup | MEDIA |
| `credentials` |分散 | Trait CredentialProvider unificado | MEDIA |
| Secret masking | No existe | Exact, URL-encoded, Base64 | MEDIA |
| `options` | Parcial | timeout, retry, fail_fast | BAJA |

---

## PARTE 2: DISEÑO CERRADO (de deep-research.md)

### 2.1 PipelineSpec Mínimo (MVP-0)

```rust
pub struct PipelineSpec {
    pub schema_version: String,      // "pipeliner.pipeline.v1"
    pub pipeliner_version: String,  // "0.1.0"
    pub stages: Vec<StageSpec>,
    pub post: Option<PostSpec>,
}

pub struct StageSpec {
    pub id: String,
    pub display_name: String,
    pub execution: StageExecution,
    pub post: Option<PostSpec>,
}

pub enum StageExecution {
    Steps { steps: Vec<StepSpec> },
    Parallel { stages: Vec<StageSpec> },
}

pub enum StepSpec {
    Shell(ShellStepSpec),
    Echo(EchoStepSpec),
}

pub struct ShellStepSpec {
    pub kind: ShellKind,
    pub script: String,
    pub label: Option<String>,
    pub interpolation: InterpolationMode,
    pub capture_stdout: bool,
    pub return_status: bool,
    pub fail_on_nonzero: bool,
}

pub enum ShellKind {
    Sh,
    PowerShell,
    Cmd,
}

pub enum InterpolationMode {
    Pipeliner,  // default
    Raw,        // para sh raw
}
```

### 2.2 DSL Objetivo

```rust
pipeline! {
    stages {
        stage "Build" {
            steps {
                sh "cargo build";
                echo "done";
            }
        }
    }
}
```

El DSL `pipeline!` genera automáticamente `fn main()` que:
1. Construye el `PipelineSpec`
2. Lo serializa a JSON
3. Lo emite por stdout con `--describe`

### 2.3 Flujo de Ejecución

```
pipeline.rs
    |
    v
[Macro pipeline! expande a fn main]
    |
    v
[Compilar + cachear estilo rust-script]
    |
    v
[Ejecutar: pipeline --describe]
    |
    v
[stdout = PipelineSpec JSON]
    |
    v
[CLI recibe, valida, ejecuta]
```

---

## PARTE 3: PLAN DE REESTRUCTURACIÓN

### 3.1 Fases de Reestructuración

```
Fase 0: Limpieza y preparación
  - Eliminar código huérfano
  - Consolidar crates steps
  - Definir Workspace Dependencies

Fase 1: Core + Spec
  - Reescribir PipelineSpec según diseño
  - Separar spec de runtime
  - Crear pipeliner-protocol

Fase 2: Macro DSL
  - Reescribir pipeline! macro
  - Implementar parser que genere PipelineSpec
  - Soportar sintaxis objetivo

Fase 3: Script Runner
  - Completar pipeliner-script
  - Implementar cargo cache
  - Protocolo --describe

Fase 4: Runtime
  - Reescribir pipeliner-runtime (desde executor)
  - Ejecución local
  - Eventos, report.json

Fase 5: Features CI
  - env, with_env, credentials
  - parallel, post
  - timeout, retry
  - archive, junit

Fase 6: Extensibilidad
  - Plugins como funciones IntoSteps
  - Credential providers
```

### 3.2 Red Lines del MVP (según diseño)

```
1. Sin agent/docker/kubernetes/ssh executor
2. Sin input/aprobaciones manuales
3. Sin matrix builds
4. Sin sandbox fuerte para pipeline.rs
5. Sin ejecución remota
6. Sin UI web
7. Sin executor plugins dinámicos
8. Sin closures Rust dinámicas serializadas en PipelineSpec
9. Sin estado unstable
10. Sin explain
11. Sin caché de PipelineSpec
12. Sin configuración avanzada de credential providers
```

### 3.3 Crates a Crear/Modificar/Eliminar

#### CREAR:
- `pipeliner-protocol` - Serialización, schemas, --describe
- `pipeliner-runtime` - Runtime central (separado de executor)
- `pipeliner-credentials` - Sistema de credenciales unificado

#### MODIFICAR PROFUNDAMENTE:
- `pipeliner-core` - Nuevos PipelineSpec, StageSpec, StepSpec
- `pipeliner-macros` - Reescribir pipeline! con nueva sintaxis
- `pipeliner-script` - Completar rust-script-like

#### SIMPLIFICAR/CONSOLIDAR:
- `pipeliner-executor` → absorción parcial en `pipeliner-runtime`
- `pipeliner-steps-*` → consolidar en `pipeliner-steps` workspace
- `pipeliner-worker` → absorbed into runtime
- `pipeliner-infrastructure` → diferir a futuro

#### ELIMINAR/MARCAR COMO DEPRECATED:
- Algunos steps crates redundantes
- Funcionalidad de agent/docker/k8s en MVP

---

## PARTE 4: ROADMAP DE IMPLEMENTACIÓN

### MVP-0: Vertical Slice Mínimo ⭐ (PRIORIDAD MÁXIMA)

**Objetivo**: Demo ejecutable `pipeliner run pipeline.rs`

**Entregables**:
```
- CLI con comando `run` y `check`
- pipeline! macro con sintaxis nueva
- PipelineSpec mínimo serializable
- --describe protocol (stdout JSON)
- Ejecución local de sh
- Eventos básicos en consola
- report.json mínimo
```

**Ejemplo soportado**:
```rust
pipeline! {
    stages {
        stage "Build" {
            steps {
                sh "cargo build";
                echo "done";
            }
        }
    }
}
```

**Estructura de crates MVP-0**:
```
pipeliner-core/src/
  ├── lib.rs
  ├── spec/
  │   ├── mod.rs
  │   ├── pipeline_spec.rs
  │   ├── stage_spec.rs
  │   ├── step_spec.rs
  │   └── post_spec.rs
  └── prelude.rs

pipeliner-macros/src/
  ├── lib.rs
  └── pipeline_macro.rs  (REESCRIBIR)

pipeliner-protocol/src/
  ├── lib.rs
  ├── describe.rs
  └── schemas.rs

pipeliner-script/src/
  ├── lib.rs
  ├── compiler.rs
  ├── cache.rs
  └── runner.rs

pipeliner-runtime/src/
  ├── lib.rs
  ├── local_executor.rs
  ├── events.rs
  └── report.rs

pipeliner-cli/src/
  ├── lib.rs
  └── main.rs
```

**Tests MVP-0**:
- Unit tests para pipeline macro parsing
- Snapshot tests para PipelineSpec JSON
- Integration tests con tempdir
- Exit codes verification

---

### MVP-1: DSL CI Básico

**Objetivo**: Pipeline CI funcional completo

**Agregar**:
```
- env (pipeline + stage level)
- with_env (step scoped)
- dir (step scoped)
- post (always/success/failure)
- options: timeout, retry
- interpolación $VAR/${VAR}
- sh raw
- sh extendido (label, capture_stdout, etc.)
- let_output
- check semántico
- events.jsonl
- report.json completo
```

**Ejemplo soportado**:
```rust
pipeline! {
    options {
        timeout minutes(60);
        retry 1;
    }

    env {
        RUST_BACKTRACE = "1";
        CARGO_TERM_COLOR = "always";
    }

    stages {
        stage "Build" {
            steps {
                dir "app" {
                    sh "cargo build";
                }
            }
        }

        stage "Test" {
            options {
                timeout minutes(10);
                retry 2;
            }

            steps {
                let_output TEST_RESULT = sh {
                    script "cargo test --format json";
                    capture_stdout true;
                };

                junit "target/test-results/**/*.xml";
            }
        }
    }

    post {
        always {
            echo "Pipeline finished";
        }
        failure {
            echo "Pipeline failed";
        }
    }
}
```

---

### MVP-2: Paralelismo y Control

**Objetivo**: Parallel stages funcional

**Agregar**:
```
- parallel { stage ... stage ... }
- parallel anidado (recursivo)
- max_stage_depth configurable (default: 4)
- fail_fast configurable por stage
- --parallelism CLI flag
- cancelación graceful + Ctrl+C
- dry-run
- graph --format mermaid/dot
```

**Modelo**:
```rust
enum StageExecution {
    Steps { steps: Vec<StepSpec> },
    Parallel { stages: Vec<StageSpec> },  // recursivo
}
```

**CLI**:
```bash
pipeliner run pipeline.rs --parallelism 4
pipeliner dry-run pipeline.rs
pipeliner graph pipeline.rs --format mermaid
```

---

### MVP-3: CI Real

**Objetivo**: Credenciales y artefactos

**Agregar**:
```
- CredentialProvider trait
- Local credential store (cifrado Argon2id + XChaCha20-Poly1305)
- Credentials providers: local, env, file
- with_credentials block
- secret masking (exact, URL-encoded, Base64)
- archive step
- junit step
- gc --keep
- Config global/local
- pipeliner_dir configurable
```

**CLI credentials**:
```bash
pipeliner credentials init
pipeliner credentials set npm-token
pipeliner credentials list
```

---

### MVP-4: Extensibilidad

**Objetivo**: Plugins como funciones Rust

**Agregar**:
```
- IntoSteps trait
- Generadores de steps: StepSpec, Vec<StepSpec>, Result<StepSpec,E>, Result<Vec<StepSpec>,E>
- Dependencias externas en bloque cargo embebido
- Contrato de pureza/determinismo para generadores
```

**Ejemplo plugin**:
```rust
//! ```cargo
//! [dependencies]
//! pipeliner_slack = "0.1"
//! ```

use pipeliner::prelude::*;
use pipeliner_slack::slack_message;

pipeline! {
    stages {
        stage "Notify" {
            steps {
                slack_message("#ci", "Build finished");
            }
        }
    }
}
```

---

## PARTE 5: MIGRACIÓN DE CRATES ACTUALES

### 5.1 Consolidação de Steps

**Estado Actual** (many small crates):
```
pipeliner-steps-core/
pipeliner-steps-git/
pipeliner-steps-http/
pipeliner-steps-tooling/
pipeliner-steps-maven/
pipeliner-steps-gradle/
pipeliner-steps-container/
pipeliner-steps-helm/
pipeliner-steps-scanner/
pipeliner-steps-policy/
pipeliner-steps-artifact/
pipeliner-steps-notify/
```

**Estado Objetivo**:
```
pipeliner-steps/           # Workspace
├── steps-core/            # sh, echo, dir, with_env, retry, timeout
├── steps-credential/       # with_credentials
├── steps-archive/         # archive
├── steps-report/          # junit
├── steps-notify/          # Slack, etc.
└── steps-plugin/          # Trait IntoSteps, plugin system
```

Los crates individuales (`steps-git`, `steps-http`, etc.) se mantienen como plugins externos o se mueven a repos separados.

### 5.2 Consolidação de Executor/Worker

El `pipeliner-executor` actual y `pipeliner-worker` se consolidan en `pipeliner-runtime`:

```rust
// pipeliner-runtime/src/lib.rs
pub trait Executor {
    fn run_step(&self, step: StepSpec, ctx: ExecutionContext) -> StepResult;
}

pub struct LocalExecutor { ... }

impl Executor for LocalExecutor {
    fn run_step(&self, step: StepSpec, ctx: ExecutionContext) -> StepResult {
        // ejecución local de sh, echo, dir, etc.
    }
}
```

Futuro (NO en MVP):
```rust
pub struct DockerExecutor { ... }
pub struct KubernetesExecutor { ... }
pub struct SshExecutor { ... }
```

---

## PARTE 6: DIRECTORIO DE TRABAJO

### 6.1 `.pipeliner/` Structure

```
workspace/.pipeliner/
├── runs/
│   └── <run_id>/
│       ├── events.jsonl
│       ├── report.json
│       ├── logs/
│       └── artifacts/
├── tmp/
└── config.toml        # opcional, sobreescribe global

~/.config/pipeliner/config.toml  # global
```

### 6.2 Cache de Compilación

```
~/.cache/pipeliner/scripts/<cache_key>/
├── Cargo.toml
├── Cargo.lock
└── src/main.rs
```

Cache key = hash de pipeline.rs + versión del CLI

---

## PARTE 7: SCHEMAS VERSIONADOS

```
Schema: pipeliner.pipeline.v1
Schema: pipeliner.run_report.v1
Schema: pipeliner.run_event.v1
```

Validación: CLI rechaza schema_version desconocido.

Compatibilidad: CLI y crate pipeliner version deben coincidir exactamente.

---

## PARTE 8: TESTS

### Suite Completa (desde MVP-0)

```
1. Parser/proc macro unit tests
   - bloques válidos
   - bloques fuera de orden
   - stage sin steps/parallel
   - steps desconocidos

2. trybuild
   - errores de compilación esperados
   - spans de macro
   - mensajes de ayuda

3. insta snapshots
   - pipeline DSL -> PipelineSpec JSON
   - graph Mermaid/DOT
   - report.json
   - events.jsonl normalizado

4. integration tests CLI
   - tempdir
   - run/check/dry-run/graph
   - cache hit/miss
   - exit codes

5. runtime tests
   - executor fake
   - shell command fake
   - timeout/retry/cancelación
   - parallelism determinista

6. security/logging tests
   - masking secreto exacto
   - URL encoded
   - base64
   - no secretos en report/events
```

### Dev Dependencies

```toml
[dev-dependencies]
trybuild = "1"
insta = "1"
assert_cmd = "2"
assert_fs = "1"
predicates = "3"
```

---

## PARTE 9: ORDEN DE IMPLEMENTACIÓN RECOMENDADO

### Sprint 1-2: MVP-0 Core
1. Reescribir `pipeliner-core` con PipelineSpec nuevo
2. Crear `pipeliner-protocol` con --describe
3. Reescribir `pipeliner-macros` con sintaxis nueva
4. Implementar cache de compilación en `pipeliner-script`
5. CLI básico con `run` y `check`
6. Runtime local minimal (solo sh y echo)
7. Tests del vertical slice

### Sprint 3-4: MVP-1 DSL
1. Implementar env, with_env, dir
2. Implementar post
3. Implementar options (timeout, retry)
4. Implementar interpolación
5. sh raw y sh extendido
6. let_output
7. events.jsonl y report.json completo
8. Tests completos

### Sprint 5-6: MVP-2 Parallel
1. Implementar parallel
2. Implementar fail_fast
3. Implementar cancelación graceful
4. dry-run
5. graph
6. Tests de concurrencia

### Sprint 7-8: MVP-3 Credentials
1. CredentialProvider trait
2. Local credential store
3. with_credentials
4. Secret masking
5. archive, junit
6. gc

### Sprint 9-10: MVP-4 Plugins
1. IntoSteps trait
2. Generadores con Result
3. Documentación de plugins
4. Ejemplo de plugin externo

---

## PARTE 10: Métricas de Éxito

### MVP-0 Checklist
- [ ] `pipeliner run examples/minimal.rs` ejecuta sin errores
- [ ] `pipeliner check examples/minimal.rs` valida correctamente
- [ ] PipelineSpec JSON se genera por --describe
- [ ] Exit codes correctos (0 success, 1 failure, 3 compile error)
- [ ] Tests pasan (>90% coverage en core)

### MVP-1 Checklist
- [ ] Todos los steps built-in funcionan (sh, echo, dir, with_env, retry, timeout)
- [ ] env propagation correcto
- [ ] post conditions ejecutan correctamente
- [ ] Interpolación $VAR funciona
- [ ] let_output captura stdout

### MVP-2 Checklist
- [ ] parallel ejecuta stages concurrentemente
- [ ] fail_fast cancela branches correctamente
- [ ] Ctrl+C funciona graceful
- [ ] dry-run muestra plan sin ejecutar
- [ ] graph genera Mermaid válido

### MVP-3 Checklist
- [ ] Credentials se almacenan cifrados
- [ ] with_credentials injecta secrets correctamente
- [ ] Secrets no aparecen en logs
- [ ] archive guarda artefactos
- [ ] junit parsea reports

### MVP-4 Checklist
- [ ] Plugin crate externo compila y funciona
- [ ] IntoSteps trait funciona con Result
- [ ] Documentación de plugin authoring

---

## ANEXO: Comandos CLI Objetivo

```bash
# Ejecución
pipeliner run pipeline.rs
pipeliner run pipeline.rs --workspace ./work
pipeliner run pipeline.rs --parallelism 4

# Validación
pipeliner check pipeline.rs

# Planificación
pipeliner dry-run pipeline.rs
pipeliner graph pipeline.rs --format mermaid
pipeliner graph pipeline.rs --format dot

# Gestión
pipeliner gc --keep 20
pipeliner cache clean
pipeliner cache list

# Credentials
pipeliner credentials init
pipeliner credentials set <name>
pipeliner credentials list
pipeliner credentials delete <name>

# Help
pipeliner --help
pipeliner run --help
```

---

## Exit Codes

```
0   success
1   pipeline failure
2   CLI usage/config error
3   compile error
4   validation/compatibility error
5   infrastructure/runtime error
130 aborted/cancelled (Ctrl+C)
```

---

*Documento generado a partir del análisis de `specs/deep-research.md`*