# Integración Extendida de Rustline con rust-script

## Resumen Ejecutivo

Este documento propone una integración más profunda entre Rustline y rust-script, transformando Rustline en una **extensión natural** de rust-script en lugar de simplemente ser una biblioteca que se ejecuta a través de él.

## Estado Actual

### Lo que funciona actualmente

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! rustline = "0.1"
//!

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        stages!(stage!("Build", steps!(sh!("cargo build"))))
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;
    Ok(())
}
```

```bash
rust-script mi-pipeline.rs
```

### Limitaciones del modelo actual

1. **Sin comandos dedicados**: No hay forma de hacer `rust-script pipeline run ...`
2. **Sin templates**: El usuario debe escribir el boilerplate cada vez
3. **Sin detección automática**: rust-script no sabe que es un pipeline
4. **Sin comandos de gestión**: No hay forma de listar, validar, o generar pipelines
5. **Sin integración con el ecosistema**: No aprovecha características de rust-script como `--package`, `--wrapper`, etc.

---

## Propuesta de Integración Extendida

### 1. Shebang Especializado

```rust
#!/usr/bin/env rustline-run
// rustline: name = "mi-pipeline"
// rustline: description = "Pipeline de producción"
// rustline: version = "1.0.0"

pipeline!(
    agent_any(),
    stages!(stage!("Build", steps!(sh!("cargo build"))))
)
```

### 2. Comandos Dedicados de rust-script

```bash
# Ejecutar pipeline directamente
rust-script run pipeline.rs

# Generar proyecto Cargo desde pipeline
rust-script --package pipeline.rs

# Validar pipeline sin ejecutar
rust-script validate pipeline.rs

# Dry-run del pipeline
rust-script dry-run pipeline.rs

# Generar documentación del pipeline
rust-script doc pipeline.rs
```

### 3. Template System

```bash
# Iniciar nuevo pipeline desde template
rust-script init --template=rust my-pipeline.rs
rust-script init --template=docker my-pipeline.rs
rust-script init --template=full-stack my-pipeline.rs
rust-script init --template=library my-pipeline.rs

# Listar templates disponibles
rust-script init --list-templates
```

Templates disponibles:

| Template | Descripción | Comandos |
|----------|-------------|----------|
| `basic` | Pipeline mínimo | build, test |
| `rust` | Proyecto Rust estándar | fetch, build, test, clippy, fmt |
| `docker` | Build y push Docker | build, docker build, docker push |
| `library` | Biblioteca Rust | build, test, doc, publish |
| `full-stack` | Aplicación completa | build, test, integration, deploy |
| `microservice` | Microservicio | build, test, containerize, deploy |
| `workspace` | Cargo workspace | build all, test all |

### 4. Metadatos Embebidos

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! rustline = "0.1"
//!
//! [package]
//! name = "mi-pipeline"
//! version = "1.0.0"
//! description = "Pipeline de CI/CD para producción"
//! authors = ["Team DevOps <devops@empresa.com>"]
//!
//! [pipeline]
//! defaultExecutor = "local"
//! timeout = 3600
//! retryOnFailure = true

pipeline!(
    agent_any(),
    stages!(...)
)
```

### 5. Variables de Entorno Especiales

```bash
# Directorio de trabajo del pipeline
export RUSTLINE_WORKSPACE="/path/to/project"

# Archivo de configuración del pipeline
export RUSTLINE_CONFIG="$HOME/.rustline/config.yaml"

# Modo de ejecución (development, staging, production)
export RUSTLINE_MODE="production"

# Nivel de caché
export RUSTLINE_CACHE="full"  # full, dependencies, none
```

### 6. Extensión de Comandos rust-script

#### Subcomando `rust-script pipeline`

```bash
# Ejecutar pipeline
rust-script pipeline run pipeline.rs
rust-script pipeline run pipeline.rs --executor=local
rust-script pipeline run pipeline.rs --executor=docker
rust-script pipeline run pipeline.rs --executor=kubernetes

# Validar pipeline
rust-script pipeline validate pipeline.rs
rust-script pipeline validate pipeline.rs --strict

# Generar desde template
rust-script pipeline new my-pipeline.rs
rust-script pipeline new --template=rust my-pipeline.rs

# Listar pipelines en directorio
rust-script pipeline list

# Mostrar información del pipeline
rust-script pipeline info pipeline.rs

# Generar documentación
rust-script pipeline doc pipeline.rs

# Exportar a otros formatos
rust-script pipeline export pipeline.rs --format=github-actions
rust-script pipeline export pipeline.rs --format=gitlab-ci
rust-script pipeline export pipeline.rs --format=jenkins
```

#### Flags Globales

```bash
# Formato de salida
rust-script pipeline run pipeline.rs --output=human  # default
rust-script pipeline run pipeline.rs --output=json
rust-script pipeline run pipeline.rs --output=yaml
rust-script pipeline run pipeline.rs --output=quiet

# Nivel de logging
rust-script pipeline run pipeline.rs --log=error
rust-script pipeline run pipeline.rs --log=warn
rust-script pipeline run pipeline.rs --log=info
rust-script pipeline run pipeline.rs --log=debug
rust-script pipeline run pipeline.rs --log=trace

# Opciones de ejecución
rust-script pipeline run pipeline.rs --dry-run
rust-script pipeline run pipeline.rs --steps=build,test  # ejecutar solo stages específicos
rust-script pipeline run pipeline.rs --parallel  # forzar ejecución paralela
rust-script pipeline run pipeline.rs --timeout=300
rust-script pipeline run pipeline.rs --retry=2
```

---

## User Stories Nuevas

### US-X.1: Pipeline Runner como Extensión de rust-script

**Descripción**: Integrar Rustline como un conjunto de subcomandos de rust-script.

**Criterios de Aceptación**:
- [ ] `rust-script pipeline run` ejecuta pipelines
- [ ] `rust-script pipeline validate` valida sintaxis
- [ ] `rust-script pipeline new` genera pipelines desde templates
- [ ] `rust-script pipeline list` lista pipelines en directorio
- [ ] `rust-script pipeline export` exporta a otros formatos
- [ ] Compatibilidad total con flags de rust-script existentes

### US-X.2: Sistema de Templates

**Descripción**: Implementar sistema de templates para crear pipelines rápidamente.

**Criterios de Aceptación**:
- [ ] Templates para casos de uso comunes (rust, docker, library, etc.)
- [ ] Personalización de templates via flags
- [ ] Usuario puede crear templates personalizados
- [ ] Templates incluyen documentación embebida
- [ ] Templates validan configuración requerida

### US-X.3: Metadatos de Pipeline

**Descripción**: Agregar metadatos embebidos en scripts de pipeline.

**Criterios de Aceptación**:
- [ ] Sección `[pipeline]` en comentarios de manifiestos
- [ ] Configuración de executor por defecto
- [ ] Configuración de timeout y retry global
- [ ] Descripción y documentación embebida
- [ ] Validación de metadatos al cargar pipeline

### US-X.4: Modo Interactivo

**Descripción**: Modo interactivo para crear y ejecutar pipelines.

**Criterios de Aceptación**:
- [ ] `rust-script pipeline interactive` inicia modo interactivo
- [ ] Wizard para crear pipeline paso a paso
- [ ] Preview del pipeline antes de ejecutar
- [ ] Historial de pipelines ejecutados
- [ ] Sugerencias basadas en el contexto del proyecto

### US-X.5: Integración con rust-script cache

**Descripción**: Aprovechar el sistema de caché de rust-script para pipelines.

**Criterios de Aceptación**:
- [ ] Pipelines no modificados se ejecutan inmediatamente (cached)
- [ ] Detección automática de cambios en el pipeline
- [ ] Cache compartido entre ejecuciones
- [ ] Invalidación de cache configurable
- [ ] Métricas de cache hit/miss

---

## Implementación Técnica

### Arquitectura Propuesta

```
┌─────────────────────────────────────────────────────────────┐
│                    rust-script                              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                  rustline extension                      ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  ││
│  │  │  pipeline   │  │  template   │  │    metadata     │  ││
│  │  │   runner    │  │   system    │  │    parser       │  ││
│  │  └─────────────┘  └─────────────┘  └─────────────────┘  ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
          │                  │                    │
          ▼                  ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│                      Rustline Library                        │
│  ┌───────────┐  ┌───────────┐  ┌─────────────────────────┐ │
│  │  DSL      │  │Executor   │  │ Backends                │ │
│  │           │  │(Local/Dock│  │ (GitHub, GitLab, K8s)   │ │
│  └───────────┘  └───────────┘  └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Cambios en main.rs Actual

```rust
// src/main.rs actual (5 líneas)
fn main() {
    println!("Rustline - A Jenkins Pipeline DSL in Rust");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
}
```

```rust
// src/main.rs propuesto
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rustline")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a pipeline
    Run {
        file: String,
        #[arg(short, long)]
        executor: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long)]
        dry_run: bool,
    },
    /// Validate a pipeline
    Validate {
        file: String,
        #[arg(short, long)]
        strict: bool,
    },
    /// Create a new pipeline from template
    New {
        file: String,
        #[arg(short, long)]
        template: Option<String>,
    },
    /// Export pipeline to other formats
    Export {
        file: String,
        #[arg(short, long)]
        format: String,
    },
    /// List pipelines in directory
    List {
        dir: Option<String>,
    },
    /// Show pipeline information
    Info {
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { file, executor, output, dry_run }) => {
            // Run pipeline
        }
        Some(Commands::Validate { file, strict }) => {
            // Validate pipeline
        }
        Some(Commands::New { file, template }) => {
            // Create from template
        }
        Some(Commands::Export { file, format }) => {
            // Export to format
        }
        Some(Commands::List { dir }) => {
            // List pipelines
        }
        Some(Commands::Info { file }) => {
            // Show info
        }
        None => {
            // Interactive mode or help
        }
    }
}
```

---

## Beneficios de la Integración Extendida

### Para Usuarios Existentes de rust-script

1. **Curva de aprendizaje cero**: Si ya conocen rust-script, conocen Rustline
2. **Portabilidad**: Los pipelines siguen siendo scripts Rust autocontenidos
3. **Rendimiento**: Aprovechan el sistema de caché de rust-script
4. **Flexibilidad**: Pueden usar rust-script features (--wrapper, --package, etc.)

### Para Nuevos Usuarios

1. **Facilidad de inicio**: Templates generan código funcional inmediatamente
2. **CLI completa**: No necesitan escribir código para casos comunes
3. **Documentación integrada**: Los metadatos generan documentación automática
4. **Validación temprana**: Errores detectados antes de ejecutar

### Para Equipos de DevOps

1. **Consistencia**: Templates enforce best practices
2. **Estandarización**: Todos los pipelines siguen el mismo formato
3. **Integración**: Export a GitHub Actions, GitLab CI, etc.
4. **Mantenimiento**: Metadatos facilitan gestión de pipelines

---

## Plan de Implementación

### Fase 1: CLI Básica (US-4.4 existente)

- [ ] Subcomando `run`
- [ ] Subcomando `validate`
- [ ] Subcomando `dry-run`
- [ ] Flags de output y log level

### Fase 2: Templates (US-X.2)

- [ ] Template system básico
- [ ] Templates: basic, rust, docker, library
- [ ] Comando `new` y `--template` flag
- [ ] Validación de templates

### Fase 3: Metadatos (US-X.3)

- [ ] Parser de sección `[pipeline]`
- [ ] Configuración de executor por defecto
- [ ] Documentación embebida
- [ ] Validación de metadatos

### Fase 4: Integración con rust-script (US-X.1)

- [ ] Subcomando `list`
- [ ] Subcomando `info`
- [ ] Subcomando `export`
- [ ] Wrapper commands para rust-script

### Fase 5: Modo Interactivo (US-X.4)

- [ ] Wizard para creación
- [ ] Preview de pipelines
- [ ] Historial de ejecuciones
- [ ] Sugerencias contextuales

### Fase 6: Optimización (US-X.5)

- [ ] Cache de pipelines
- [ ] Detección de cambios
- [ ] Métricas de rendimiento
- [ ] Configuración de cache

---

## Ejemplos de Uso

### Pipeline Completo con Metadatos

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! rustline = "0.1"
//!
//! [package]
//! name = "production-pipeline"
//! version = "1.0.0"
//! description = "Pipeline de producción para el servicio de API"
//! authors = ["DevOps Team <devops@empresa.com>"]
//!
//! [pipeline]
//! defaultExecutor = "docker"
//! timeout = 3600
//! retryOnFailure = true
//! environment = "production"

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_docker!("rust:1.70"),
        environment!(
            "SERVICE_NAME" => "api-gateway",
            "DOCKER_REGISTRY" => "empresa.azurecr.io"
        ),
        stages!(
            stage!("Checkout", steps!(
                sh!("git fetch --all"),
                sh!("git checkout ${GIT_COMMIT}")
            )),
            stage!("Build", steps!(
                sh!("cargo build --release --locked")
            )),
            stage!("Test", steps!(
                sh!("cargo test --all-features"),
                sh!("cargo clippy -- -D warnings")
            )),
            stage!("Security Scan", steps!(
                sh!("cargo audit --deny warnings"),
                sh!("trivy image rust:1.70")
            )),
            stage!("Build Image", steps!(
                sh!("docker build -t ${DOCKER_REGISTRY}/${SERVICE_NAME}:${BUILD_NUMBER} ."),
                sh!("docker push ${DOCKER_REGISTRY}/${SERVICE_NAME}:${BUILD_NUMBER}")
            )),
            stage!("Deploy", steps!(
                when!(branch("main")),
                sh!("kubectl set image deployment/${SERVICE_NAME} ${SERVICE_NAME}=${DOCKER_REGISTRY}/${SERVICE_NAME}:${BUILD_NUMBER}")
            ))
        ),
        post!(
            success(sh!("echo 'Pipeline exitoso'")),
            failure(sh!("echo 'Pipeline falló'"))
        )
    );

    let executor = DockerExecutor::new();
    executor.execute(&pipeline)?;
    Ok(())
}
```

### Uso de la CLI

```bash
# Ejecutar pipeline
rustline run production-pipeline.rs

# Validar antes de ejecutar
rustline validate production-pipeline.rs --strict

# Generar nuevo pipeline desde template
rustline new api-service.rs --template=rust

# Listar pipelines en el directorio
rustline list

# Exportar a GitHub Actions
rustline export production-pipeline.rs --format=github-actions -o .github/workflows/ci.yml

# Dry-run para testing
rustline run production-pipeline.rs --dry-run

# Ejecutar con logging detallado
rustline run production-pipeline.rs --log=debug

# Ejecutar solo ciertos stages
rustline run production-pipeline.rs --steps=build,test
```

---

## Conclusiones

La integración extendida de Rustline con rust-script transforma la herramienta de:

**Estado actual**: Una biblioteca que se puede ejecutar via `rust-script file.rs`

**Estado propuesto**: Una extensión de rust-script que proporciona:
1. Comandos dedicados para pipelines
2. Sistema de templates para inicio rápido
3. Metadatos embebidos para documentación y configuración
4. CLI completa para validación, exportación y gestión
5. Modo interactivo para usuarios no técnicos

Esta integración aprovecha lo mejor de ambos mundos:
- **rust-script**: Caching, portabilidad, familiaridad para usuarios Rust
- **Rustline**: Type-safety, DSL expresivo, múltiples backends

El resultado es una herramienta que es simultáneamente:
- **Fácil de usar**: Templates y CLI simple
- **Potente**: DSL completo con control de flujo avanzado
- **Portable**: Funciona en cualquier lugar donde funcione rust-script
- **Profesional**: Métricas, logging, validación, exportación
