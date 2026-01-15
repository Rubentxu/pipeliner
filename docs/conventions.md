# Convenciones del Proyecto Rustline

Este documento define las convenciones y estándares que deben seguirse en el desarrollo del proyecto Rustline.

## Índice

- [Git y Commits](#git-y-commits)
- [Naming Conventions](#naming-conventions)
- [Code Style](#code-style)
- [Testing](#testing)
- [Documentation](#documentation)
- [Error Handling](#error-handling)
- [File Organization](#file-organization)
- [Versioning](#versioning)

---

## Git y Commits

### Branch Naming

**Formato**: `<type>/<short-description>`

**Tipos**:
- `feature/US-ID`: Nueva funcionalidad (ej: `feature/US-1.1`)
- `bugfix/description`: Corrección de bug (ej: `bugfix/memory-leak`)
- `refactor/description`: Refactorización (ej: `refactor/executor-trait`)
- `test/description`: Agregar/modificar tests (ej: `test/parallel-execution`)
- `docs/description`: Documentación (ej: `docs/api-reference`)
- `perf/description`: Mejora de performance (ej: `perf/cache-optimization`)
- `chore/description`: Mantenimiento (ej: `chore/update-dependencies`)
- `release/vX.Y.Z`: Preparación de release (ej: `release/v0.1.0`)

### Commit Messages

**Formato**: `type(scope): description`

**Tipos**:
- `feat`: Nueva funcionalidad
- `fix`: Bug fix
- `refactor`: Refactorización sin cambio funcional
- `test`: Agregar o modificar tests
- `docs`: Documentación
- `perf`: Mejora de performance
- `chore`: Mantenimiento
- `style`: Formato de código (sin lógica)

**Ejemplos**:
```bash
feat(pipeline): add support for matrix directive
fix(executor): handle timeout correctly on SIGKILL
refactor(steps): extract common execution logic
test(pipeline): add integration test for parallel stages
docs(api): document PipelineExecutor trait
perf(cache): implement LRU cache for expressions
chore(deps): upgrade tokio to v1.20
style(fmt): run cargo fmt
```

**Reglas**:
- Usar imperativo presente ("add" no "added" o "adds")
- No finalizar con punto
- Línea de asunto ≤ 72 caracteres
- Body envuelto en 72 caracteres
- Referenciar issues con `#ID`

**Plantilla**:
```
type(scope): subject

<body>

<footer>
```

**Ejemplo completo**:
```
feat(pipeline): add support for when conditions

Add the `when!` macro that allows stages to execute conditionally
based on Git branch, environment variables, or custom expressions.

Closes #42
```

### Pull Requests

**Título**: Seguir formato de commit message

**Descripción**:
- Cambios incluidos
- Motivación del cambio
- Testing realizado
- Screenshots si aplica (UI)

**Checklist**:
- [ ] Tests agregados/actualizados
- [ ] `cargo test` pasa
- [ ] `cargo clippy -- -D warnings` sin warnings
- [ ] `cargo fmt --check` sin cambios
- [ ] `cargo doc --no-deps` sin warnings
- [ ] Documentación actualizada
- [ ] Changelog actualizado

**Labels**:
- `epic: X`: Épica relacionada
- `sprint: X`: Sprint actual
- `type: feat/fix/refactor/etc`: Tipo de PR
- `priority: high/medium/low`: Prioridad
- `size: XS/S/M/L/XL`: Tamaño estimado

**Review**:
- Mínimo 1 approval de maintainer
- CI debe estar verde
- Comments addressed o justificados

---

## Naming Conventions

### Rust Naming

**Tipos (Structs, Enums, Traits)**:
```rust
pub struct Pipeline { }
pub enum AgentType { }
pub trait PipelineExecutor { }
```
- PascalCase
- Descriptivo y completo
- Evitar abreviaciones (a menos que sean muy comunes)

**Funciones y Métodos**:
```rust
pub fn execute_pipeline() { }
pub fn build() -> Self { }
pub fn validate(&self) -> Result<(), Error> { }
```
- snake_case
- Verbos imperativos para funciones públicas
- Getters: omitir `get_` prefijo a menos que sea necesario
- Setters: usar `set_` prefijo

**Variables**:
```rust
let pipeline_name = "build";
let max_retries = 3;
let is_valid = true;
```
- snake_case
- Descriptivo pero conciso
- Booleanos: usar `is_`, `has_`, `can_` prefijos

**Constantes**:
```rust
pub const MAX_PIPELINE_NAME_LENGTH: usize = 100;
pub const DEFAULT_TIMEOUT_SECS: u64 = 600;
```
- SCREAMING_SNAKE_CASE
- Solo para true constants

**Lifetime Parameters**:
```rust
pub struct Stage<'a> {
    pub name: Cow<'a, str>,
}

pub fn parse_with_lifetime<'a>(input: &'a str) -> Result<...>
```
- Nombres cortos: `'a`, `'b`, `'c`
- O descriptivos: `'input`, `'env`

### Macros

**Macro names**:
```rust
pipeline!()    // snake_case con ! sufijo
stage!()       // snake_case con ! sufijo
```
- snake_case con `!` sufijo
- Seguir convención del DSL original (Jenkins usa snake_case)

### Modules

```rust
mod pipeline { }
mod executor { }
mod infrastructure { }
```
- snake_case
- Un módulo por archivo (principalmente)
- `mod.rs` para directorios

### Newtype Pattern

```rust
pub struct StageName(String);
pub struct ShellCommand(String);
pub struct EnvironmentKey(String);
```
- PascalCase (es un tipo)
- Descriptivo del contenido

---

## Code Style

### General

- **Indentation**: 4 espacios (no tabs)
- **Line length**: Máximo 100 caracteres (idealmente < 80)
- **Blank lines**: Una línea entre funciones y módulos principales
- **Trailing whitespace**: Prohibido

### Imports

**Orden**:
1. `std` imports
2. `extern crate` imports
3. Tercer-party crate imports
4. `crate` imports
5. `super` y `self` imports

**Grupos separados por línea en blanco**:

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::pipeline::Pipeline;
use crate::executor::PipelineExecutor;

use super::context::PipelineContext;
```

**Reglas**:
- Ordenar alfabéticamente dentro de grupos
- Preferir `{use}` para múltiples items del mismo módulo
- Evitar `use *` excepto en prelude

### Structs

```rust
/// Documentation comment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pipeline {
    /// Public field documentation
    pub name: String,

    /// Private field documentation
    stages: Vec<Stage>,
}

impl Pipeline {
    /// Constructor documentation
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stages: Vec::new(),
        }
    }

    /// Public method documentation
    pub fn add_stage(&mut self, stage: Stage) {
        self.stages.push(stage);
    }

    /// Private method
    fn validate(&self) -> Result<(), Error> {
        // ...
    }
}
```

**Reglas**:
- Documentar structs públicos con `///`
- Derivar traits comunes (`Debug`, `Clone`, `PartialEq`)
- Campos públicos primero, privados después
- Constructor `new()` público
- Métodos públicos primero, privados después

### Enums

```rust
/// Agent type enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentType {
    /// Execute on any available agent
    Any,

    /// Execute on agent with specific label
    Label(String),

    /// Execute in Docker container
    Docker(DockerConfig),

    /// Execute in Kubernetes pod
    Kubernetes(KubernetesConfig),
}
```

**Reglas**:
- Documentar variantes
- Variants en PascalCase
- Simple variants primero, con datos después

### Functions

```rust
/// Executes a shell command and captures output.
///
/// # Arguments
///
/// * `command` - The shell command to execute
///
/// # Returns
///
/// * `Ok(String)` - Command stdout
/// * `Err(CommandError)` - If command fails
///
/// # Examples
///
/// ```
/// let output = execute_shell("echo test")?;
/// assert_eq!(output.trim(), "test");
/// ```
pub fn execute_shell(command: &str) -> Result<String, CommandError> {
    // Implementation
}
```

**Reglas**:
- Documentar con `///`
- Incluir secciones: descripción, arguments, returns, errors, examples, panics
- Type hints en parámetros: `impl Into<String>`, `&str`, etc.
- Return types explícitos

### Closures

```rust
// Preferible para closure simple
let names = stages.iter().map(|s| &s.name).collect::<Vec<_>>();

// Con bloque para lógica compleja
let valid = stages.iter().all(|stage| {
    !stage.name.is_empty() && !stage.steps.is_empty()
});

// Con move cuando es necesario
let name = pipeline_name.clone();
let result = thread::spawn(move || {
    execute_pipeline(name)
}).join();
```

**Reglas**:
- Preferir `.map()`, `.filter()`, etc. sobre for loops
- Usar `{}` cuando closure es multiline
- Usar `move` explícitamente cuando captura owned data

### Pattern Matching

```rust
// Exhaustive matching
match agent_type {
    AgentType::Any => println!("Any agent"),
    AgentType::Label(label) => println!("Label: {}", label),
    AgentType::Docker(config) => println!("Docker: {}", config.image),
    AgentType::Kubernetes(config) => println!("K8s: {}", config.namespace),
}

// Matching con guards
match result {
    StageResult::Success if !warnings.is_empty() => {
        println!("Success with warnings");
    }
    StageResult::Success => {
        println!("Clean success");
    }
    _ => {}
}

// If let para casos parciales
if let AgentType::Docker(config) = &agent {
    println!("Using Docker image: {}", config.image);
}
```

**Reglas**:
- Usar `match` para branching múltiple
- Usar `if let`/`while let` para casos parciales
- Manejar explícitamente todos los casos o usar `_`
- Usar guards cuando sea necesario

### Error Handling

```rust
// Use ? para propagación
pub fn execute(&self) -> Result<PipelineResult, PipelineError> {
    let context = self.create_context()?;
    let result = self.execute_stages(&context)?;
    Ok(result)
}

// Use map_err para transformación de errores
pub fn parse(input: &str) -> Result<Pipeline, ParseError> {
    serde_yaml::from_str(input)
        .map_err(|e| ParseError::InvalidYaml(e.to_string()))
}

// Use context para agregar contexto
pub fn execute_stage(&self, stage: &Stage) -> Result<StageResult, StageError> {
    self.execute_steps(&stage.steps)
        .context("Failed to execute steps")?
        .map(|_| StageResult::Success)
}
```

**Reglas**:
- Use `thiserror` para errores personalizados
- Use `anyhow::Result` para application code
- Use `?` para propagación
- Agregar contexto con `.context()` cuando sea útil
- Evitar `unwrap()` y `expect()` en código de producción

---

## Testing

### Test Naming

**Formato**: `test_<unit>_<scenario>_<expected_result>`

```rust
#[test]
fn test_pipeline_execution_with_single_stage_succeeds() { }

#[test]
fn test_retry_with_zero_count_fails_validation() { }

#[test]
fn test_timeout_when_command_exceeds_limit_cancels() { }
```

**Reglas**:
- Descriptivo y completo
- Usa `succeeds`, `fails`, `returns`, etc. para resultado esperado
- Happy paths primero, luego error cases

### Test Structure

```rust
#[test]
fn test_pipeline_execution_with_multiple_stages() {
    // Arrange
    let pipeline = create_test_pipeline_with_3_stages();
    let executor = LocalExecutor::new();

    // Act
    let result = executor.execute(&pipeline);

    // Assert
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), PipelineResult::Success));
}
```

**Reglas**:
- Arrange-Act-Assert (AAA) pattern
- Comentarios marcando cada sección (opcional pero recomendado)
- Asertivos específicos (no solo `assert!(result.is_ok())`)
- Mensajes en aserciones cuando ayuda al debugging

### Test Organization

```
src/
├── lib.rs
├── pipeline/
│   ├── mod.rs
│   ├── types.rs
│   │   └── #[cfg(test)] mod tests { }  // Unit tests junto al código
│   └── stages.rs
│       └── #[cfg(test)] mod tests { }
└── executor/
    └── local.rs
        └── #[cfg(test)] mod tests { }

tests/
├── integration/                              // Integration tests
│   ├── mod.rs
│   └── pipeline_execution.rs
└── baseline/                                 // Critical baseline tests
    └── basic_pipeline.rs

benches/
└── pipeline_execution.rs                    // Benchmarks

examples/
├── basic_pipeline.rs
└── advanced_pipeline.rs
```

---

## Documentation

### Doc Comments

```rust
/// Executes a shell command and captures its output.
///
/// This function spawns a shell process, runs the specified command,
/// and returns the standard output if the command succeeds.
///
/// # Arguments
///
/// * `command` - The shell command to execute
///
/// # Returns
///
/// * `Ok(String)` - The command's stdout
/// * `Err(CommandError)` - If the command fails
///
/// # Errors
///
/// This function will return an error if:
/// - The shell cannot be spawned
/// - The command returns a non-zero exit code
///
/// # Examples
///
/// ```
/// use rustline::executor::execute_shell;
///
/// let output = execute_shell("echo 'Hello, World!')?;
/// assert_eq!(output.trim(), "Hello, World!");
/// # Ok::<(), rustline::executor::CommandError>(())
/// ```
///
/// # Panics
///
/// This function does not panic under normal circumstances.
pub fn execute_shell(command: &str) -> Result<String, CommandError> {
    // Implementation
}
```

**Secciones**:
- Breve descripción (primera línea)
- Detalles adicionales (párrafos)
- `# Arguments`
- `# Returns`
- `# Errors` (si aplica)
- `# Examples` (con doc tests)
- `# Panics` (si aplica)
- `# Safety` (para unsafe code)

### Module Documentation

```rust
//! Pipeline domain types and logic.
//!
//! This module contains the core domain types for defining
//! CI/CD pipelines, including `Pipeline`, `Stage`, and `Step`.
//!
//! # Examples
//!
//! ```rust
//! use rustline::pipeline::{Pipeline, Stage, Step};
//!
//! let pipeline = Pipeline::builder()
//!     .agent(AgentType::Any)
//!     .stage(Stage::new("Build", vec![Step::Shell("cargo build".to_string())]))
//!     .build()?;
//! # Ok::<(), rustline::pipeline::BuildError>(())
//! ```
```

**Reglas**:
- Primera línea: resumen del módulo
- `//!` para módulos
- Documentación en `lib.rs`, `mod.rs`

### Inline Comments

```rust
// Calculate the hash of the Cargo.lock file
let lock_hash = calculate_hash(cargo_lock_path)?;

// Cache key format: "cargo-{sha256_hash}"
let cache_key = format!("cargo-{}", lock_hash);
```

**Reglas**:
- `//` para comentarios inline
- Explicar *por qué*, no *qué* (el código explica qué)
- Mantener actualizados con el código

---

## Error Handling

### Error Type Definition

```rust
use thiserror::Error;

/// Errors that can occur during pipeline execution.
#[derive(Error, Debug)]
pub enum PipelineError {
    /// Validation failed with the specified reason.
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationError),

    /// Stage execution failed.
    #[error("Stage '{stage}' failed: {error}")]
    StageFailed {
        stage: String,
        error: Box<StageError>,
    },

    /// Command execution failed.
    #[error("Command failed with exit code {code}: {stderr}")]
    CommandFailed {
        code: i32,
        stderr: String,
    },

    /// IO error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Timeout exceeded.
    #[error("Timeout after {duration:?}")]
    Timeout {
        duration: std::time::Duration,
    },
}
```

**Reglas**:
- Usar `#[error(...)]` para mensajes
- Usar `#[from]` para conversiones automáticas
- Incluir contexto en variantes (stage name, command, etc.)

### Error Propagation

```rust
// Use ? para propagación simple
pub fn execute(&self) -> Result<PipelineResult, PipelineError> {
    self.prepare_environment()?;
    let result = self.execute_pipeline()?;
    self.cleanup()?;
    Ok(result)
}

// Use map_err para transformación
pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let content = fs::read_to_string(path)
        .map_err(|e| ConfigError::ReadError {
            path: path.to_path_buf(),
            source: e,
        })?;
    // ...
}
```

---

## File Organization

### Directory Structure

```
rustline/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── CHANGELOG.md
├── LICENSE-APACHE
├── LICENSE-MIT
│
├── docs/                           # Documentation
│   ├── README.md
│   ├── rust-jenkins-dsl-study.md
│   ├── epics.md
│   ├── tdd-strategy.md
│   └── architecture.md
│
├── src/
│   ├── lib.rs                      # Crate root, re-exports
│   ├── main.rs                     # Binary entry point (CLI)
│   ├── prelude.rs                  # Common imports
│   │
│   ├── macros.rs                   # Declarative macros
│   │
│   ├── pipeline/                   # Domain layer
│   │   ├── mod.rs
│   │   ├── types.rs
│   │   ├── agent.rs
│   │   ├── stages.rs
│   │   ├── steps.rs
│   │   ├── environment.rs
│   │   ├── parameters.rs
│   │   ├── triggers.rs
│   │   ├── options.rs
│   │   └── validation.rs
│   │
│   ├── executor/                   # Application layer
│   │   ├── mod.rs
│   │   ├── trait.rs
│   │   ├── context.rs
│   │   ├── local.rs
│   │   ├── docker.rs
│   │   └── kubernetes.rs
│   │
│   └── infrastructure/              # Infrastructure layer
│       ├── mod.rs
│       ├── github_actions.rs
│       ├── gitlab_ci.rs
│       ├── logging.rs
│       ├── metrics.rs
│       └── config.rs
│
├── tests/                          # Integration tests
│   ├── integration/
│   │   ├── mod.rs
│   │   └── pipeline_execution.rs
│   └── baseline/
│       └── basic_pipeline.rs
│
├── benches/                        # Benchmarks
│   └── pipeline_execution.rs
│
└── examples/                       # Examples
    ├── basic_pipeline.rs
    ├── parallel_pipeline.rs
    └── docker_pipeline.rs
```

### File Naming

- `snake_case.rs` para archivos Rust
- `snake_case.md` para documentos
- `UPPERCASE` para nombres de constantes (en archivos config)
- `snake_case` para directorios

---

## Versioning

### Semantic Versioning (SemVer)

**Formato**: `MAJOR.MINOR.PATCH`

- **MAJOR**: Cambios incompatibles en API
- **MINOR**: Nuevas funcionalidades backwards-compatible
- **PATCH**: Bug fixes backwards-compatible

**Ejemplos**:
- `0.1.0` → `0.1.1`: Bug fix
- `0.1.1` → `0.2.0`: Nueva feature backwards-compatible
- `0.2.0` → `1.0.0`: API estable
- `1.0.0` → `2.0.0`: Breaking change

### Pre-releases

**Formato**: `MAJOR.MINOR.PATCH-PRERELEASE`

**Ejemplos**:
- `0.1.0-alpha.1`
- `0.1.0-beta.1`
- `0.1.0-rc.1`

### Changelog

**Formato**: [Keep a Changelog](https://keepachangelog.com/)

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Support for `when!` directive with branch conditions
- `timeout!` macro for command timeout
- Caching of Cargo dependencies

### Changed
- Improved error messages for validation failures
- Refactored `PipelineExecutor` trait to use generics

### Fixed
- Fixed memory leak in parallel stage execution
- Corrected handling of empty stage names

### Removed
- Deprecated `Pipeline::execute()` method (use `PipelineExecutor`)

## [0.1.0] - 2025-01-15

### Added
- Initial release with basic pipeline DSL
- `pipeline!`, `stage!`, `steps!`, `sh!` macros
- `LocalExecutor` for executing commands
- Basic validation
```

**Reglas**:
- Categorías: Added, Changed, Deprecated, Removed, Fixed, Security
- Version tags en Git
- Links a issues/PRs con `(#123)`

---

## Referencias

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)
- [The Rust Style Guide](https://github.com/rust-lang/rust/blob/master/src/doc/style-guide/src/chapter_1.md)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Keep a Changelog](https://keepachangelog.com/)
- [Semantic Versioning](https://semver.org/)
