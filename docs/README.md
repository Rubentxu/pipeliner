# Rustline - DSL de Jenkins Pipeline en Rust

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Docs](https://img.shields.io/badge/docs-latest-blue.svg)](docs/)

Un DSL (Domain Specific Language) en Rust que replica la sintaxis y semántica del DSL de Jenkins Pipeline, ejecutable mediante rust-script para proporcionar capacidades de automatización de pipelines CI/CD directamente desde el ecosistema Rust.

## 📋 Índice

- [Visión](#visión)
- [Características](#características)
- [Documentación](#documentación)
- [Estado del Proyecto](#estado-del-proyecto)
- [Roadmap](#roadmap)
- [Principios](#principios)
- [Contributing](#contributing)

---

## 🎯 Visión

Rustline busca llenar el vacío en el ecosistema Rust de herramientas de CI/CD, proporcionando un DSL type-safe que combine:

- **Expresividad** de Jenkins Pipeline
- **Seguridad de tipos** de Rust
- **Portabilidad** de rust-script
- **Alto rendimiento** con zero-cost abstractions

### Problema que Resuelve

El ecosistema Rust actual sufre de:

1. **Fragmentación**: Múltiples herramientas (cargo-make, just, xtask) sin integración
2. **Verbosidad**: Configuraciones YAML extensas en GitHub Actions/GitLab CI
3. **Falta de portabilidad**: Pipelines no son portables entre plataformas CI/CD
4. **Limitaciones**: Gestión compleja de estado y condiciones

### Solución Propuesta

Un DSL en Rust que:

- Permite definir pipelines como código Rust portable y versionable
- Ejecuta directamente con rust-script sin configuración adicional
- Se integra nativamente con el ecosistema Rust
- Proporciona validación en tiempo de compilación
- Ofrece múltiples backends de ejecución (local, Docker, Kubernetes, GitHub Actions, GitLab CI)

---

## ✨ Características

### Sintaxis Familiar

```rust
use rustline::prelude::*;

let pipeline = pipeline!(
    agent_any(),
    stages!(
        stage!("Build", steps!(
            sh!("cargo build --release"),
            sh!("cargo clippy -- -D warnings")
        )),
        stage!("Test", steps!(
            sh!("cargo test --all-features"),
            timeout!(600, sh!("cargo test --release"))
        )),
        stage!("Deploy", steps!(
            when!(branch("main")),
            sh!("cargo publish"),
            custom_step!("slack_notify", channel: "#releases")
        ))
    ),
    post!(
        always(sh!("cargo clean")),
        success(custom_step!("slack_notify", message: "Pipeline succeeded!")),
        failure(custom_step!("slack_notify", message: "Pipeline failed!"))
    )
);

let executor = LocalExecutor::new();
executor.execute(&pipeline)?;
```

### Ejecución Paralela

```rust
parallel!(
    branch!("Linux", stage!("Linux", steps!(sh!("cargo test")))),
    branch!("macOS", stage!("macOS", steps!(sh!("cargo test")))),
    branch!("Windows", stage!("Windows", steps!(sh!("cargo test"))))
)
```

### Matrix Testing

```rust
matrix!(
    axes!(
        rust = ["stable", "beta", "nightly"],
        os = ["linux", "macos", "windows"]
    ),
    exclude!(os == "windows", rust == "nightly")
)
```

### Múltiples Backends

```rust
// Ejecución local
let executor = LocalExecutor::new();

// En Docker
let executor = DockerExecutor::new()
    .with_image("rust:latest")
    .with_workdir("/workspace");

// En Kubernetes
let executor = KubernetesExecutor::new()
    .with_namespace("ci")
    .with_pod_spec(pod_template);

// Generar GitHub Actions workflow
let backend = GitHubActionsBackend::new();
let workflow = backend.translate(&pipeline)?;
fs::write(".github/workflows/ci.yml", workflow)?;
```

---

## 📚 Documentación

### Documentación Principal

| Documento | Descripción |
|-----------|-------------|
| [Estudio Técnico](docs/rust-jenkins-dsl-study.md) | Análisis completo del DSL de Jenkins Pipeline, técnicas DSL en Rust, y propuesta de diseño (50+ páginas) |
| [Épicas por Sprint](docs/epics.md) | Plan detallado de 12 sprints con 4 épicas, user stories y criterios de aceptación |
| [Estrategia TDD](docs/tdd-strategy.md) | Guía completa de Test-Driven Development con tipos de tests, workflows y convenciones |
| [Arquitectura y Alto Rendimiento](docs/architecture.md) | Arquitectura hexagonal, patrones de diseño, optimizaciones de performance y excelencia operativa |

### Estructura de Documentación

```
docs/
├── README.md                        # Este archivo
├── rust-jenkins-dsl-study.md       # Estudio técnico completo
├── epics.md                         # Épicas organizadas por sprint
├── tdd-strategy.md                 # Estrategia de TDD
└── architecture.md                  # Arquitectura y performance
```

---

## 🚀 Estado del Proyecto

### Fase Actual: Planificación y Diseño ✅

**Completado**:
- [x] Estudio técnico del DSL de Jenkins Pipeline
- [x] Análisis de técnicas DSL en Rust
- [x] Diseño de arquitectura hexagonal
- [x] Planificación de 4 épicas por sprint
- [x] Estrategia TDD definida
- [x] Especificación de arquitectura y performance

**En Progreso**:
- [ ] Configuración inicial del proyecto (Cargo workspace, estructura de directorios)
- [ ] Setup de CI/CD (GitHub Actions con tests, clippy, fmt)

### Próximos Pasos

1. **Sprint 1** (Fase 1): Fundamentos del DSL
   - Estructuras de datos fundamentales
   - Macros declarativas básicas
   - Primer motor de ejecución

2. **Sprint 2-3**: Características avanzadas
   - Bloques post y when
   - Steps de control de flujo
   - Ejecución paralela

3. **Sprint 4-6**: Backends de ejecución
   - GitHub Actions backend
   - GitLab CI backend
   - Docker y Kubernetes executors

4. **Sprint 7-9**: Ecosistema
   - Sistema de plugins
   - Observabilidad y telemetría
   - Herramientas de desarrollo

5. **Sprint 10-12**: Lanzamiento
   - Documentación completa
   - Lanzamiento v0.1.0
   - Outreach a la comunidad

---

## 🗓️ Roadmap de Implementación

### Épica 1: Fundamentos del DSL (Sprints 1-3)

**Objetivo**: Establecer los cimientos del DSL con estructuras de datos y macros básicas.

**Entregables**:
- Structs y enums del dominio (Pipeline, Stage, Step, Agent)
- Macros declarativas (`pipeline!`, `stage!`, `steps!`, `sh!`, `echo!`)
- LocalExecutor básico con ejecución de comandos shell
- Tests de integración completos

**Métricas**:
- Coverage > 80%
- Zero clippy warnings
- Tiempo de parsing < 10ms

### Épica 2: Características Avanzadas (Sprints 4-6)

**Objetivo**: Implementar funcionalidades que diferencian el DSL de herramientas simples.

**Entregables**:
- Bloques post con condiciones (always, success, failure, unstable, changed)
- Directiva when con condiciones complejas
- Steps de control de flujo (retry, timeout, stash, unstash)
- Ejecución paralela con parallel! y matrix!

**Métricas**:
- Coverage > 85%
- Benchmarks para todas las features
- Property tests para invariantes

### Épica 3: Backends de Ejecución (Sprints 7-9)

**Objetivo**: Desarrollar backends alternativos para diferentes entornos.

**Entregables**:
- GitHub Actions backend (DSL → YAML workflow)
- GitLab CI backend (DSL → .gitlab-ci.yml)
- Docker Executor (ejecución en contenedores)
- Kubernetes Executor (ejecución en pods)
- Sistema de plugins para steps custom

**Métricas**:
- Todos los backends tienen tests de integración
- Validación de YAMLs generados
- Limpieza de recursos (containers, pods)

### Épica 4: Ecosistema y Documentación (Sprints 10-12)

**Objetivo**: Establecer el ecosistema alrededor del DSL.

**Entregables**:
- Observabilidad (tracing, Prometheus metrics)
- CLI con integración rust-script
- Extensiones de IDE (VS Code)
- Documentación completa (Quick Start, API Reference, Examples)
- Templates de pipelines comunes
- Lanzamiento v0.1.0 en crates.io

**Métricas**:
- 100% de API pública documentada
- Quick Start < 5 minutos
- Instalación exitosa < 15 minutos
- Lanzamiento sin breaking changes conocidos

---

## 🏗️ Principios

### Desarrollo

1. **TDD (Test-Driven Development)**
   - Red → Green → Refactor para cada funcionalidad
   - 100% de código cubierto por tests
   - Baseline Test Suite con 100% success rate

2. **Alto Rendimiento**
   - Zero-copy y ownership transfers
   - Lazy evaluation cuando es apropiado
   - Parallelism con Rayon
   - Caching inteligente
   - Benchmarks con Criterion

3. **Excelencia Operativa**
   - Observabilidad con tracing y Prometheus
   - Error handling estructurado con thiserror
   - Health checks y diagnóstico
   - Graceful shutdown
   - Configuration management

4. **Seguridad de Tipos**
   - Aprovechar el sistema de tipos de Rust
   - Newtype pattern para validación
   - State machines con tipos
   - Compile-time guarantees

### Arquitectura

1. **Hexagonal Architecture**
   - Domain layer sin dependencias externas
   - Ports (traits) y Adapters (implementaciones)
   - Testability sin mocks complejos

2. **Composability**
   - Pequeñas primitivas composables
   - Builder pattern para configuración compleja
   - Method chaining para interfaces fluidas

3. **SOLID Principles**
   - Single Responsibility: Cada module tiene una responsabilidad clara
   - Open/Closed: Abierto para extensión, cerrado para modificación
   - Liskov Substitution: Traits pueden substituirse libremente
   - Interface Segregation: Traits pequeños y específicos
   - Dependency Inversion: Depende de abstracciones, no implementaciones

### Calidad de Código

1. **Conventional Commits**
   - Formato: `type(scope): description`
   - Types: feat, fix, refactor, test, docs, perf, chore

2. **Code Review**
   - Mínimo 1 approval de maintainer
   - CI verde antes de merge
   - Todos los tests pasan
   - Sin clippy warnings
   - Formato correcto (cargo fmt)

3. **Continuous Integration**
   - Tests en múltiples plataformas (Linux, macOS, Windows)
   - Múltiples versiones de Rust (stable, nightly)
   - Coverage report con tarpaulin
   - Security audit con cargo-audit

---

## 🤝 Contributing

### Para Empezar

1. Lee la [Estrategia TDD](docs/tdd-strategy.md)
2. Revisa las [Épicas](docs/epics.md) para entender el roadmap
3. Estudia la [Arquitectura](docs/architecture.md)
4. Clona el repositorio y ejecuta los tests:
   ```bash
   git clone https://github.com/your-org/rustline.git
   cd rustline
   cargo test
   ```

### Flujo de Contribución

1. **Crear una issue** para la funcionalidad o bug fix
2. **Fork y branch**: `feature/US-ID` o `bugfix/description`
3. **Desarrollar con TDD**:
   - Escribir test que falle
   - Implementar código mínimo
   - Refactorizar manteniendo tests verdes
4. **Asegurar calidad**:
   - `cargo test` → All tests pass
   - `cargo clippy -- -D warnings` → Zero warnings
   - `cargo fmt --check` → Correct formatting
   - `cargo doc --no-deps` → No warnings
5. **Crear Pull Request** con descripción clara
6. **Address feedback** de reviewers
7. **Merge** cuando sea aprobado y CI verde

### Convenciones de Code

- **Tests**: `test_<funcionalidad>_<escenario>_<resultado>`
- **Comments**: Doc comments `///` para API pública, `//` para inline
- **Error handling**: Use `thiserror` para errores personalizados
- **Logging**: Use `tracing` para logging estructurado
- **Naming**: Snake case para todo excepto tipos (PascalCase)

---

## 📊 Progreso del Proyecto

### Métricas Actuales

| Métrica | Meta | Actual |
|---------|------|--------|
| Documentación Completa | 100% | ✅ 100% |
| Tests escritos | - | 🔄 Iniciando |
| Coverage | > 80% | - |
| Clippy warnings | 0 | ✅ 0 |
| Sprints completados | 12 | 0/12 |
| Épicas completadas | 4 | 0/4 |

### Dashboard de Sprints

```
Sprint 1  |████████████████████| 0%   (Fundamentos - Estructuras)
Sprint 2  |████████████████████| 0%   (Fundamentos - Macros)
Sprint 3  |████████████████████| 0%   (Fundamentos - Executor)
Sprint 4  |████████████████████| 0%   (Avanzadas - Post/When)
Sprint 5  |████████████████████| 0%   (Avanzadas - Control Flow)
Sprint 6  |████████████████████| 0%   (Avanzadas - Paralelismo)
Sprint 7  |████████████████████| 0%   (Backends - GitHub Actions)
Sprint 8  |████████████████████| 0%   (Backends - GitLab/Docker)
Sprint 9  |████████████████████| 0%   (Backends - K8s/Plugins)
Sprint 10 |████████████████████| 0%   (Ecosistema - Observabilidad)
Sprint 11 |████████████████████| 0%   (Ecosistema - Herramientas)
Sprint 12 |████████████████████| 0%   (Ecosistema - Lanzamiento)
```

---

## 🔗 Recursos

### Documentación Oficial

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)

### Herramientas del Ecosistema

- [Jenkins Pipeline Syntax](https://www.jenkins.io/doc/book/pipeline/syntax/)
- [rust-script](https://rust-script.org/)
- [rayon](https://docs.rs/rayon/) - Data parallelism
- [tokio](https://tokio.rs/) - Async runtime
- [tracing](https://docs.rs/tracing/) - Structured logging
- [thiserror](https://docs.rs/thiserror/) - Error handling
- [criterion](https://bheisler.github.io/criterion.rs/) - Benchmarking
- [proptest](https://proptest-rs.github.io/proptest/) - Property testing

### Comunidades

- [r/rust](https://reddit.com/r/rust)
- [Rust Discord](https://discord.gg/rust-lang)
- [Rust Users Forum](https://users.rust-lang.org/)

---

## 📄 Licencia

Este proyecto está licenciado bajo MIT License o Apache License 2.0, a tu elección.

---

## 📮 Contacto

- **Issues**: [GitHub Issues](https://github.com/your-org/rustline/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/rustline/discussions)
- **Email**: rustline@example.com

---

*Construido con ❤️ en Rust para el ecosistema Rust*
