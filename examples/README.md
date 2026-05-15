# Pipeliner — Ejemplos de Pipelines en Rust DSL

> ⚠️ **IMPORTANTE**: Pipeliner usa **Rust DSL** para definir pipelines con macros estilo Jenkinsfile. Los archivos `.json` son solo para serialización. NO uses YAML para definir pipelines.

## Sintaxis DSL Estilo Jenkinsfile

```rust
use pipeliner_macros::pipeline;

let my_pipeline = pipeline! {
    name = "Mi Pipeline CI"
    
    stages {
        stage!("Build") {
            steps {
                sh!("cargo build --release")
                echo!("Build complete!")
            }
        }
        
        stage!("Test") {
            steps {
                sh!("cargo test --all")
            }
        }
    }
};
```

## Ejemplos Disponibles

### 1. CI Build Pipeline
**Archivo**: `pipelines/ci_build.rs`

Pipeline CI típico que compila, testea y verifica el binary.

```bash
cargo run --example ci_build
```

### 2. PetClinic CI Pipeline
**Archivo**: `pipelines/petclinic_ci.rs`

Clona el repositorio Spring PetClinic, lo compila y ejecuta tests.

```bash
cargo run --example petclinic_ci
```

## Macros Disponibles

| Macro | Descripción | Ejemplo |
|-------|-------------|---------|
| `pipeline! { ... }` | Pipeline completo | `pipeline! { name = "X" stages { ... } }` |
| `stage!("name") { ... }` | Stage con steps | `stage!("Build") { steps { ... } }` |
| `steps { ... }` | Bloque de pasos | Dentro de stage |
| `sh!("cmd")` | Comando shell | `sh!("cargo build")` |
| `echo!("msg")` | Mensaje | `echo!("Done!")` |

## Estructura Completa de Pipeline

```rust
pipeline! {
    name = "Nombre del Pipeline"
    
    stages {
        stage!("Stage 1") {
            steps {
                sh!("comando 1")
                sh!("comando 2")
                echo!("Mensaje")
            }
        }
        
        stage!("Stage 2") {
            steps {
                sh!("otro comando")
            }
        }
    }
}
```

## Notas

- Los pipelines se definen como código Rust compilado
- Los macros internos (`sh!`, `echo!`) se expanden dentro del contexto de `pipeline!`
- El pipeline puede serializarse a JSON para persistencia
