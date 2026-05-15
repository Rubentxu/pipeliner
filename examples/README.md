# Pipeliner — Ejemplos

Pipeliner puede usarse de dos formas:

## 1. Como Librería (Rust crate)

Importa las structs y ejecuta pipelines programáticamente:

```bash
# Ejecutar examples
cargo run --example ci_build
cargo run --example petclinic
```

### Example: CI Build

```bash
cargo run --example ci_build
```

Output:
```
=== CI Build Pipeline ===

Pipeline: CI Build Pipeline
Stages: 4
  1. Prerequisites (3 steps)
  2. Build (2 steps)
  3. Test (2 steps)
  4. Verify (2 steps)

--- Running Pipeline ---
--- Results ---
Success: true
✅ Pipeline completed successfully!
```

### Example: PetClinic CI

```bash
cargo run --example petclinic
```

Clona Spring PetClinic, lo compila con Maven y ejecuta tests.

## 2. Como CLI

```bash
# Ejecutar pipeline desde JSON
pipeliner run --file pipeline.json

# Validar pipeline
pipeliner validate --file pipeline.json

# Dry-run
pipeliner run --file pipeline.json --dry-run
```

## Examples Disponibles

| Example | Descripción |
|---------|-------------|
| `ci_build.rs` | Pipeline CI básico: build + test + verify |
| `petclinic.rs` | Clone + build + test de repositorio Java |

## Uso como Librería

```rust
use pipeliner_core::{Pipeline, PipelineRunner, Stage, Step};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = Pipeline::new()
        .with_name("Mi Pipeline")
        .with_stage(
            Stage::new("Build")
                .with_step(Step::shell("cargo build"))
                .with_step(Step::echo("Done!"))
        );

    let mut runner = PipelineRunner::new();
    let results = runner.run_async(&pipeline).await?;
    
    if results.success {
        println!("Pipeline succeeded!");
    }
    
    Ok(())
}
```
