# Estrategia TDD para Rustline

Este documento define la estrategia de Test-Driven Development (TDD) específica para el proyecto Rustline, asegurando calidad, mantenibilidad y excelencia operativa.

## Filosofía TDD

El proyecto sigue rigurosamente el ciclo **Red → Green → Refactor** para cada funcionalidad implementada:

1. **Red**: Escribir un test que falle antes de implementar la funcionalidad
2. **Green**: Escribir el código mínimo para que el test pase
3. **Refactor**: Mejorar el código manteniendo los tests verdes

> "If you don't know what the test should be, you don't know what the code should do."

---

## Tipos de Tests

### 1. Unit Tests

**Propósito**: Verificar la corrección de unidades individuales de código (funciones, structs, métodos).

**Ubicación**: Junto al código que prueban (`src/module.rs` → tests en `#[cfg(test)] mod tests`)

**Características**:
- Rápidos de ejecutar (< 1s)
- Sin dependencias externas (filesystem, network)
- Aislados: no dependen del estado de otros tests
- Determinísticos: misma salida para misma entrada

**Ejemplo**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_creation() {
        let agent = AgentType::Label("linux".to_string());
        assert!(matches!(agent, AgentType::Label(_)));
    }

    #[test]
    fn test_stage_name_validation() {
        assert!(Stage::new("", steps!()).is_err());
        assert!(Stage::new("valid", steps!()).is_ok());
    }
}
```

**Criterios de Cobertura**:
- 100% de funciones públicas tienen tests
- 100% de branches condicionales cubiertos
- Todos los edge cases probados (empty, null, boundary values)

### 2. Integration Tests

**Propósito**: Verificar la integración entre múltiples módulos y componentes del sistema.

**Ubicación**: Directorio `tests/` en la raíz del crate

**Características**:
- Pruebas de comportamiento de end-to-end
- Pueden usar dependencias externas (filesystem, process)
- Simulan escenarios reales de uso
- Más lentos que unit tests pero rápidos (< 10s)

**Ejemplo**:
```rust
// tests/pipeline_execution.rs
use rustline::prelude::*;

#[test]
fn test_complete_pipeline_execution() {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Build", steps!(sh!("echo 'building'"))),
            stage!("Test", steps!(sh!("echo 'testing'")))
        ),
        post!(
            always(sh!("echo 'cleanup'"))
        )
    );

    let executor = LocalExecutor::new();
    let result = executor.execute(&pipeline);

    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), PipelineResult::Success));
}

#[test]
fn test_pipeline_failure_propagates() {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Fail", steps!(sh!("exit 1")))
        )
    );

    let executor = LocalExecutor::new();
    let result = executor.execute(&pipeline);

    assert!(result.is_err());
}
```

**Criterios**:
- Todas las épicas tienen tests de integración
- Tests de escenarios reales de usuarios
- Tests de error handling y recovery

### 3. Property-Based Tests

**Propósito**: Verificar propiedades invariantes usando generadores de inputs aleatorios.

**Framework**: `proptest` crate

**Características**:
- Encuentran edge cases que tests manuales miss
- Generan miles de casos de test automáticamente
- Encuentran contradicciones en invariantes

**Ejemplo**:
```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_pipeline_execution_is_deterministic(
            stages_count in 0..10usize
        ) {
            let stages: Vec<_> = (0..stages_count)
                .map(|i| stage!(format!("Stage{}", i), steps!(sh!("echo test"))))
                .collect();

            let pipeline1 = pipeline!(
                agent_any(),
                stages!(stages),
                post!(always(sh!("echo done")))
            );

            let pipeline2 = pipeline1.clone();

            let executor = LocalExecutor::new();
            let result1 = executor.execute(&pipeline1).unwrap();
            let result2 = executor.execute(&pipeline2).unwrap();

            assert_eq!(result1, result2);
        }

        #[test]
        fn test_timeout_never_exceeds_limit(
            duration_secs in 1..100u64
        ) {
            let step = sh!("sleep 100"); // 100s sleep
            let timeout = timeout!(duration_secs, step);

            let start = Instant::now();
            let _ = timeout.execute(&context);
            let elapsed = start.elapsed();

            assert!(elapsed <= Duration::from_secs(duration_secs + 1));
        }
    }
}
```

**Criterios**:
- Macros del DSL tienen property tests
- Invariantes críticos (determinismo, límites) verificadas
- Encuentra bugs antes de producción

### 4. Benchmark Tests

**Propósito**: Medir y asegurar el rendimiento del código.

**Framework**: `criterion` crate

**Características**:
- Medición precisa de tiempo de ejecución
- Comparación entre versiones
- Detección de regresiones de performance

**Ejemplo**:
```rust
// benches/pipeline_parsing.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustline::prelude::*;

fn benchmark_pipeline_macro(c: &mut Criterion) {
    c.bench_function("pipeline_macro_simple", |b| {
        b.iter(|| {
            let pipeline = pipeline!(
                agent_any(),
                stages!(
                    stage!("Build", steps!(sh!("cargo build"))),
                    stage!("Test", steps!(sh!("cargo test")))
                )
            );
            black_box(pipeline)
        })
    });
}

fn benchmark_pipeline_execution(c: &mut Criterion) {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Build", steps!(sh!("echo 'building'")))
        )
    );

    c.bench_function("pipeline_execution", |b| {
        let executor = LocalExecutor::new();
        b.iter(|| {
            black_box(executor.execute(&pipeline))
        })
    });
}

criterion_group!(benches, benchmark_pipeline_macro, benchmark_pipeline_execution);
criterion_main!(benches);
```

**Criterios**:
- Macros tienen benchmarks (latencia < 10ms)
- Executor tiene benchmarks (overhead < 100ms)
- No regresiones de performance entre versiones

### 5. Snapshot Tests

**Propósito**: Verificar que el output de una función no cambia inesperadamente.

**Framework**: `insta` crate

**Características**:
- Ideal para testing de code generation (YAML, JSON)
- Detección de cambios en output
- Facilita review de cambios

**Ejemplo**:
```rust
#[test]
fn test_github_actions_workflow_generation() {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Build", steps!(sh!("cargo build")))
        )
    );

    let backend = GitHubActionsBackend::new();
    let workflow = backend.translate(&pipeline).unwrap();

    insta::assert_yaml_snapshot!(workflow);
}
```

**Criterios**:
- Generadores de YAML/JSON tienen snapshots
- Cambios en snapshots requieren review explícito
- Snapshots versionados en Git

---

## Workflow de Desarrollo TDD

### Paso 1: Escribir el Test (Red)

**Antes de escribir cualquier código funcional:**

```rust
// 1. Crear test que falle
#[test]
fn test_agent_docker_config_validation() {
    let config = DockerConfig {
        image: "".to_string(), // Invalid: empty image
        ..Default::default()
    };

    assert!(config.validate().is_err());
}

// 2. Ejecutar test → debe fallar (compilation error o panic)
// $ cargo test test_agent_docker_config_validation
//
// error[E0599]: no method named `validate` found for struct `DockerConfig`
```

### Paso 2: Implementar Mínimo Código (Green)

**Escribir solo el código necesario para que el test pase:**

```rust
impl DockerConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.image.is_empty() {
            return Err(ConfigError::InvalidImage("image cannot be empty".to_string()));
        }
        Ok(())
    }
}
```

**Ejecutar test → debe pasar:**
```bash
$ cargo test test_agent_docker_config_validation

test test_agent_docker_config_validation ... ok
```

### Paso 3: Refactor

**Mejorar el código manteniendo tests verdes:**

```rust
// Mejor: trait de validación reutilizable
pub trait Validate {
    type Error;
    fn validate(&self) -> Result<(), Self::Error>;
}

impl Validate for DockerConfig {
    type Error = ConfigError;

    fn validate(&self) -> Result<(), Self::Error> {
        ensure!(
            !self.image.is_empty(),
            ConfigError::InvalidImage("image cannot be empty".to_string())
        );
        Ok(())
    }
}
```

**Ejecutar tests completos → todos deben pasar:**
```bash
$ cargo test

test result: ok. 42 passed; 0 failed; 0 ignored
```

---

## Jerarquía de Tests (Testing Pyramid)

```
            /\
           /  \
          / E2E \      ← Integration Tests (10%)
         /--------\
        /  Snapshots \
       /--------------\
      /   Benchmarks    \
     /--------------------\
    /   Property Tests     \
   /------------------------\
  /    Unit Tests (70%)       \
 /----------------------------\
/  Doc Tests (20%)              \
--------------------------------
```

**Proporciones**:
- **Unit Tests**: 70% - Mayoría, rápidos, específicos
- **Doc Tests**: 20% - Ejemplos en documentación
- **Integration Tests**: 10% - End-to-end
- **Property/Benchmark/Snapshot**: < 5% - Complementarios

---

## Convenciones de Nomenclatura de Tests

### Formato de nombres

`test_<funcionalidad>_<escenario>_<resultado_esperado>`

**Ejemplos**:
```rust
// ✅ Bien
fn test_pipeline_execution_with_single_stage_succeeds() { }
fn test_retry_with_zero_count_fails_validation() { }
fn test_timeout_when_command_exceeds_limit_cancels() { }

// ❌ Mal
fn test_pipeline() { } // No específico
fn check_retry() { } // No indica resultado esperado
fn it_works() { } // Sin contexto
```

### Tests de happy path

```rust
#[test]
fn test_basic_pipeline_execution_succeeds() {
    let pipeline = create_basic_pipeline();
    let executor = LocalExecutor::new();
    assert!(executor.execute(&pipeline).is_ok());
}
```

### Tests de error cases

```rust
#[test]
fn test_pipeline_with_empty_stages_fails_validation() {
    let result = Pipeline::new().with_stages(vec![]);
    assert!(result.is_err());
}

#[test]
fn test_shell_command_with_non_zero_exit_propagates_error() {
    let executor = LocalExecutor::new();
    let step = sh!("exit 1");
    let result = executor.execute_step(&step);
    assert!(matches!(result, Err(PipelineError::CommandFailed(_))));
}
```

### Tests de edge cases

```rust
#[test]
fn test_stage_with_extremely_long_name_is_accepted() {
    let long_name = "a".repeat(1000);
    let stage = stage!(long_name, steps!(sh!("echo test")));
    assert_eq!(stage.name, long_name);
}

#[test]
fn test_timeout_with_zero_duration_is_rejected() {
    let step = sh!("echo test");
    let result = timeout!(0, step);
    assert!(result.is_err());
}
```

---

## Mocking y Test Doubles

### Estrategia de Inyección de Dependencias

**Usar traits para permitir mocking:**

```rust
// Trait que permite implementaciones mock
pub trait Executor {
    fn execute(&self, command: &str) -> Result<String, ExecutorError>;
}

// Implementación real
pub struct ShellExecutor;

impl Executor for ShellExecutor {
    fn execute(&self, command: &str) -> Result<String, ExecutorError> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

// Mock para tests
#[cfg(test)]
struct MockExecutor {
    responses: Vec<Result<String, ExecutorError>>,
}

#[cfg(test)]
impl Executor for MockExecutor {
    fn execute(&self, _command: &str) -> Result<String, ExecutorError> {
        self.responses.first().unwrap().clone()
    }
}

#[cfg(test)]
#[test]
fn test_pipeline_with_mock_executor() {
    let mut mock = MockExecutor {
        responses: vec![
            Ok("build output".to_string()),
            Ok("test output".to_string()),
        ],
    };

    let pipeline = create_test_pipeline();
    let result = pipeline.execute(&mut mock);

    assert!(result.is_ok());
}
```

### Testing de errores

```rust
#[test]
fn test_pipeline_handles_executor_failure_gracefully() {
    let mut mock = MockExecutor {
        responses: vec![
            Ok("build ok".to_string()),
            Err(ExecutorError::CommandFailed("test failed".to_string())),
        ],
    };

    let pipeline = create_test_pipeline();
    let result = pipeline.execute(&mut mock);

    assert!(matches!(result, Err(PipelineError::StageFailed(_))));
}
```

---

## Tests de Documentación (Doc Tests)

**Incluir ejemplos ejecutables en la documentación:**

```rust
/// Executes a shell command and captures its output.
///
/// # Examples
///
/// ```
/// use rustline::executor::ShellExecutor;
///
/// let executor = ShellExecutor::new();
/// let output = executor.execute("echo 'hello'").unwrap();
///
/// assert_eq!(output.trim(), "hello");
/// ```
pub fn execute(&self, command: &str) -> Result<String, ExecutorError> {
    // Implementation
}
```

**Ejecutar doc tests:**
```bash
$ cargo test --doc

test src/executor.rs - executor::ShellExecutor::execute (line 42) ... ok
```

---

## Tests en Macros

**Probar expansión y comportamiento de macros:**

```rust
#[cfg(test)]
mod macro_tests {
    use super::*;

    #[test]
    fn test_pipeline_macro_generates_valid_struct() {
        let pipeline = pipeline!(
            agent_any(),
            stages!(
                stage!("Test", steps!(sh!("echo test")))
            )
        );

        assert_eq!(pipeline.stages.len(), 1);
        assert_eq!(pipeline.stages[0].name, "Test");
    }

    #[test]
    fn test_macro_compilation_fails_with_invalid_syntax() {
        // Este test verifica que la macro rechaza sintaxis inválida
        // Usando compiletest o checking compilation errors
    }
}
```

**Verificar expansión de macro:**
```bash
$ cargo expand --bin rustline > expanded.rs
```

---

## Pruebas de Integración Continua

### Pipeline de CI

**`.github/workflows/ci.yml`**:
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, nightly]

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy

      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests
        run: cargo test --all-features

      - name: Run clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: Check formatting
        run: cargo fmt --check

      - name: Run doc tests
        run: cargo test --doc

      - name: Generate coverage
        if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml

      - name: Upload coverage
        if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
        uses: codecov/codecov-action@v3

  benchmark:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Run benchmarks
        run: |
          cargo install cargo-criterion
          cargo bench
```

### Baseline Test Suite

**Suite crítica que debe mantener 100% de éxito:**

```rust
// tests/baseline.rs - La suite de pruebas más crítica
use rustline::prelude::*;

#[test]
fn baseline_basic_pipeline() {
    // Este test nunca debe fallar
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Build", steps!(sh!("echo build"))),
            stage!("Test", steps!(sh!("echo test")))
        )
    );

    let executor = LocalExecutor::new();
    assert!(executor.execute(&pipeline).is_ok());
}

#[test]
fn baseline_error_handling() {
    // Verifica que errores son manejados correctamente
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Fail", steps!(sh!("exit 1")))
        )
    );

    let executor = LocalExecutor::new();
    let result = executor.execute(&pipeline);
    assert!(result.is_err());
}
```

---

## Métricas de Calidad de Tests

### Coverage

```bash
# Generar reporte de coverage
$ cargo tarpaulin --out Html

# Resultado esperado:
# || Tested/Total Lines: 85.3% (1234/1446)
# || Tested/Total Functions: 92.1% (94/102)
# || Tested/Total Branches: 78.5% (201/256)
```

**Metas**:
- Line coverage: > 85%
- Function coverage: > 90%
- Branch coverage: > 80%

### Clippy

```bash
# Zero warnings policy
$ cargo clippy --all-targets -- -D warnings

# No output = success
```

### Formato

```bash
$ cargo fmt --check

# No output = success
```

### Doc Tests

```bash
$ cargo test --doc

# All doc tests should pass
```

---

## Recomendaciones Anti-Patterns

### ❌ Evitar

1. **Tests lentos en unit tests**
   ```rust
   // Mal: sleep en unit test
   #[test]
   fn test_slow() {
       thread::sleep(Duration::from_secs(5));
   }
   ```

2. **Tests dependientes del orden**
   ```rust
   // Mal: tests comparten estado
   static mut COUNTER: usize = 0;

   #[test]
   fn test_one() {
       unsafe { COUNTER = 1; }
   }

   #[test]
   fn test_two() {
       assert_eq!(unsafe { COUNTER }, 1); // Falla si test_one no corrió primero
   }
   ```

3. **Mocks complejos**
   ```rust
   // Mal: mock con lógica compleja
   struct ComplexMock {
       // 20 campos de configuración
   }
   ```

4. **Tests sin aserciones**
   ```rust
   // Mal: no verifica nada
   #[test]
   fn test_no_assertion() {
       let pipeline = create_pipeline();
       pipeline.execute();
   }
   ```

### ✅ Preferir

1. **Tests aislados y rápidos**
   ```rust
   // Bien: test determinístico y rápido
   #[test]
   fn test_parsing() {
       let result = parse("pipeline!(agent_any())");
       assert!(result.is_ok());
   }
   ```

2. **Tests independientes**
   ```rust
   // Bien: cada test crea su propio contexto
   #[test]
   fn test_one() {
       let context = Context::new();
       assert_eq!(context.get("key"), None);
   }

   #[test]
   fn test_two() {
       let context = Context::new();
       context.set("key", "value");
       assert_eq!(context.get("key"), Some("value"));
   }
   ```

3. **Mocks simples**
   ```rust
   // Bien: mock con comportamiento simple
   struct MockExecutor {
       should_fail: bool,
   }

   impl Executor for MockExecutor {
       fn execute(&self, _cmd: &str) -> Result<String, Error> {
           if self.should_fail {
               Err(Error::Mock)
           } else {
               Ok("success".to_string())
           }
       }
   }
   ```

---

## Referencias

- [The Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Rust by Example - Testing](https://doc.rust-lang.org/rust-by-example/testing.html)
- [TDD en Rust](https://blog.yoshuawuyts.com/tdd-in-rust/)
- [Proptest](https://proptest-rs.github.io/proptest/)
- [Criterion](https://bheisler.github.io/criterion.rs/book/)
- [Insta](https://insta.rs/)
- [Tarpaulin](https://github.com/xd009642/tarpaulin)
