# Pipelines CI/CD

Ejemplos de pipelines típicos de CI/CD.

## PetClinic Pipeline

```yaml
name: Build + Test + Verify
stages:
  - build
  - test
  - verify
  - report

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

## Más ejemplos

| Ejemplo | Descripción |
|----------|-------------|
| `01-build.yml` | Build básico |
| `01-ci.yml` | CI completo |
| `petclinic.yml` | Pipeline real |
