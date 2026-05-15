# Pipeliner DSL Context

Pipeline DSL declarativo estilo Jenkinsfile para Rust.

## Language

**Pipeline**:
Una definición declarativa de flujo de trabajo CI/CD.
_Avoid_: workflow, job

**Stage**:
Una unidad atómica de ejecución dentro de un pipeline.
_Avoid_: step group, phase

**Parallel**:
Un nivel de estructura que agrupa stages para ejecución concurrente.
_Uso_: `parallel { stage!("A") stage!("B") }`

**Step**:
Una acción individual ejecutable (shell, echo, checkout, etc).
_Avoid_: task, action, command

**StepType**:
Variante enum que define el tipo de step (Shell, Echo, Agent, etc).
_Avoid_: step kind, step variant

**DSL Declarativo**:
Sintaxis que define qué hacer, no cómo hacerlo paso a paso.
_Uso_: `pipeline! { stages { parallel { stage! { steps { sh!() } } } }`

**Plugin**:
Extensión que provee nuevos StepTypes via `StepFactory`.
_Avoid_: extension, module

**Script**:
Archivo `.rs` ejecutable con dependencias declaradas en manifest comments.
_Uso_: `//! [dependencies]` en comentarios del archivo

## Structure Levels

1. **pipeline!** (nivel root)
2. **stages { }** (contenedor de stages)
3. **parallel { }** (agrupación concurrente dentro de stages)
4. **stage!** (unidad ejecutable)
5. **steps { }** (bloque requerido - contiene sh!, script!, when!, etc)

## Syntax Requirements

### La sintaxis `steps { }` es NECESARIA porque dentro de un stage puede haber:
- `sh!`, `bat!` - comandos shell
- `script!` - scripts inline
- `when!` - condiciones
- `timeout!` - timeouts
- `retry!` - reintentos
- `withEnv!` - variables de entorno
- `withCredentials!` - credenciales

### Ejemplo correcto:
```rust
stage!("Build") {
    steps {
        sh!("cargo build")
        script! { /* rust code */ }
        when! { is!(production) }
    }
}
```

## StepFactory Pattern (Plugin System)

```rust
pub trait StepFactory: Send + Sync {
    fn name(&self) -> &str;
    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError>;
}

// Registro en runtime
let mut registry = StepRegistry::new();
registry.register(Arc::new(MyPluginFactory::new()));
```

## Script Dependencies (Plugin as Script)

Scripts declaran dependencias en manifest comments:
```rust
//! [dependencies]
//! serde = "1.0"
//! reqwest = "0.11"
//! tokio = { version = "1.0", features = ["full"] }

fn main() {
    // script code
}
```

## Steps Disponibles

### Built-in (core)
| Step | Descripción | Jenkins equivalent |
|------|-------------|-------------------|
| `sh!` | Comando shell | `sh` |
| `echo!` | Imprimir mensaje | `echo` |
| `bat!` | Batch Windows | `bat` |
| `checkout!` | Checkout SCM | `checkout` |
| `dir!` | Cambiar directorio | `dir` |
| `stash!` | Guardar archivos | `stash` |
| `unstash!` | Restaurar archivos | `unstash` |
| `input!` | Pausa para input | `input` |
| `timeout!` | Timeout | `timeout` |
| `retry!` | Reintentar | `retry` |
| `script!` | Script inline | `script` |
| `archive!` | Archivar artifacts | `archive` |
| `withCredentials!` | Credenciales | `withCredentials` |

### Control Flow
| Step | Descripción |
|------|-------------|
| `when!` | Condicional |
| `errorHandler!` | Try-catch |
| `is!` | Check environment |

### Structure
| Step | Descripción |
|------|-------------|
| `parallel!` | Nivel de estructura (NO step) |

### Agentic
| Step | Descripción |
|------|-------------|
| `agent!` | LLM-powered step |

## Relationships

- Un **Pipeline** contiene **stages**
- Un **Stage** puede contener **parallel** o **steps**
- **Parallel** agrupa **stages** para ejecución concurrente
- Un **Stage** contiene **steps** (bloque requerido)
- Un **Step** tiene un **StepType**
- Un **StepType** puede ser built-in o de un **Plugin** (via StepFactory)

## Example dialogue

> **Dev:** "¿Cómo hago que dos stages corran en paralelo?"
> **Domain expert:** "Usa `parallel!` como nivel de estructura."
> **Dev:** "¿Y si quiero ejecutar scripts con dependencias externas?"
> **Domain expert:** "Usa un archivo `.rs` con `//! [dependencies]` y ejecútalo con `script!`."

## Flagged ambiguities

- "step" vs "task" — resolved: usar "step" (consistente con Jenkins)
- "plugin" vs "extension" — resolved: usar "plugin" (consistente con Jenkins)
- "parallel" como step vs estructura — resolved: parallel es NIVEL DE ESTRUCTURA, no step
