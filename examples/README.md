# Pipeliner — Ejemplos CI/CD

Colección de pipelines reales de compilación + test + verificación.

## Pipeline CI/CD

```yaml
name: Build + Test
stages:
  - build
  - test
  - verify

build:
  steps:
    - cargo build --release

test:
  steps:
    - cargo test --all

verify:
  steps:
    - stat target/release/pipeliner
    - ls -lh target/release/pipeliner
    - echo "Build: SUCCESS"
```

## Ejemplos

| Pipeline | Descripción |
|-----------|-------------|
| `examples/pipelines/` | Pipelines completos |
| `examples/` | Scripts auxiliares |
