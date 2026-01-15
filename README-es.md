# Pipeliner

<div align="center">

**Una biblioteca de orquestación de pipelines basada en Rust con DSL compatible con Jenkins**

[![Licencia: MIT OR Apache-2.0](https://img.shields.io/badge/Licencia-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Rubentxu/pipeliner/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-121%20pasando-green.svg)](#suite-de-tests)
[![Crates](https://img.shields.io/badge/crates-8-blue.svg)](#estructura-de-crates)

</div>

---

## Descripción General

Pipeliner es una **biblioteca de orquestación de pipelines type-safe** escrita en Rust que proporciona un DSL (Domain Specific Language) compatible con Jenkins para definir pipelines CI/CD. Combina la expresividad del DSL de Jenkins con las garantías de seguridad y rendimiento de Rust.

### Características Principales

- **Diseño DSL-First**: Define pipelines con intuitivas macros `pipeline!`, `stage!`, y `steps!` - sin necesidad de configurar executors
- **Ejecución Zero-Config**: Usa las macros `run!` o `run_sync!` para ejecutar pipelines inmediatamente
- **Type Safety**: Todas las definiciones de pipelines se validan en tiempo de compilación
- **Compatibilidad Jenkins**: Sintaxis familiar para usuarios de Jenkins, con las garantías de seguridad de Rust
- **Ejecución Multi-Backend**: Ejecuta localmente, en Docker, Kubernetes, o Podman sin problemas
- **Integración con Rust-Script**: Ejecuta pipelines directamente con `rust-script` para máxima portabilidad
- **Event Sourcing**: Almacén de eventos y bus de eventos integrado para observabilidad
- **Sistema de Plugins Extensible**: Añade steps personalizados, agentes y ejecutores

---

## Inicio Rápido

### Instalación

```bash
# Clonar el repositorio
git clone https://github.com/Rubentxu/pipeliner.git
cd pipeliner

# Ejecutar tests para verificar
cd crates && cargo test --workspace
```

### Tu Primer Pipeline

Crea un archivo llamado `mi_pipeline.rs`:

```rust
#!/usr/bin/env rust-script
//!
//! # Mi Primer Pipeline con Pipeliner
//!
//! Ejecutar con: rust-script mi_pipeline.rs
//!

use pipeliner_core::prelude::*;

fn main() {
    let pipeline = pipeline! {
        agent { any() }
        stages {
            stage!("Checkout", steps!(
                echo!("📦 Clonando repositorio..."),
                sh!("git clone https://github.com/miorg/miproyecto.git")
            ))
            stage!("Build", steps!(
                echo!("🔨 Compilando proyecto..."),
                sh!("cargo build --release")
            ))
            stage!("Test", steps!(
                echo!("🧪 Ejecutando tests..."),
                sh!("cargo test")
            ))
            stage!("Deploy", steps!(
                echo!("🚀 Desplegando a producción..."),
                sh!("kubectl apply -f k8s/")
            ))
        }
        post {
            success(echo!("✅ Pipeline exitoso!")),
            failure(echo!("❌ Pipeline fallido!"))
        }
    };

    run!(pipeline);  // ¡No necesitas executor - la macro lo maneja todo!
}
```

Ejecútalo:

```bash
rust-script mi_pipeline.rs
```

> **Nota:** La macro `run!` crea automáticamente un `LocalExecutor`, ejecuta tu pipeline y maneja errores. Para contextos no-async, usa `run_sync!(pipeline)`.

---

## DSL de Pipeline

El Domain Specific Language (DSL) de Pipeliner te permite definir pipelines con intuitivas macros de Rust. El DSL es **recomendado** para la mayoría de casos de uso - es conciso, expresivo y no requiere configuración de executors.

### Macros Principales

| Macro | Descripción |
|-------|-------------|
| `pipeline!` | Define un pipeline completo con agentes, stages y post-actions |
| `stage!` | Define un stage con uno o más steps |
| `steps!` | Agrupa múltiples steps juntos |
| `sh!` | Ejecuta un comando shell |
| `echo!` | Imprime un mensaje |
| `retry!` | Reintenta un step N veces |
| `timeout!` | Ejecuta con timeout |
| `dir!` | Ejecuta steps en un directorio |
| `run!` | Ejecuta un pipeline (async) |
| `run_sync!` | Ejecuta un pipeline (bloqueante) |

### Ejemplo Completo de Pipeline

```rust
use pipeliner_core::prelude::*;

let pipeline = pipeline! {
    agent { docker("rust:1.92") }
    environment {
        ("RELEASE", "true"),
        ("LOG_LEVEL", "debug")
    }
    parameters {
        string("VERSION", "1.0.0"),
        boolean("DEPLOY_ENABLED", false)
    }
    stages {
        stage!("Build", steps!(
            echo!("📦 Compilando aplicación..."),
            sh!("cargo build --release"),
            echo!("✅ Compilación completa!")
        ))
        stage!("Test", steps!(
            echo!("🧪 Ejecutando tests..."),
            sh!("cargo test --lib"),
            sh!("cargo test --doc")
        ))
        stage!("Deploy", steps!(
            echo!("🚀 Desplegando a producción..."),
            sh!("./deploy.sh ${VERSION}"),
            echo!("✅ Despliegue completo!")
        ))
    }
    post {
        success(echo!("🎉 Pipeline exitoso!")),
        failure(echo!("❌ Pipeline fallido!")),
        always(echo!("📊 Ejecución finalizada"))
    }
};

run!(pipeline);  // Ejecuta con manejo automático de errores
```

### Tipos de Steps

```rust
use pipeliner_core::prelude::*;

let stage = stage!("Ejemplo de Stage", steps!(
    // Imprime un mensaje
    echo!("Este es un mensaje informativo"),

    // Ejecuta comando shell
    sh!("cargo build --release"),

    // Reintenta step fallido (3 intentos)
    retry!(3, sh!("comando-inestable")),

    // Timeout después de 5 minutos
    timeout!(300, sh!("tarea-larga")),

    // Ejecuta en directorio
    dir!("./scripts", steps!(
        sh!("./setup.sh"),
        sh!("./run.sh")
    ))
));
```

### Post-Condiciones

```rust
pipeline! {
    agent { any() }
    stages {
        stage!("Build", steps!(sh!("cargo build")))
    }
    post {
        always(echo!("Siempre ejecuta - limpieza, notificaciones, etc.")),
        success(echo!("Ejecuta cuando el pipeline es exitoso")),
        failure(echo!("Ejecuta cuando el pipeline falla")),
        unstable(echo!("Ejecuta cuando el pipeline es inestable"))
    }
}
```

### Parámetros y Entorno

```rust
use pipeliner_core::prelude::*;

let pipeline = pipeline! {
    agent { any() }
    environment {
        ("DATABASE_URL", "postgres://localhost:5432/db"),
        ("CACHE_TTL", "3600")
    }
    parameters {
        string("VERSION", "1.0.0"),
        boolean("SKIP_TESTS", false),
        choice("ENVIRONMENT", ["dev", "staging", "production"])
    }
    stages {
        stage!("Deploy", steps!(
            sh!("echo Desplegando ${VERSION} en ${ENVIRONMENT}"),
            sh!("./deploy.sh ${VERSION} ${ENVIRONMENT}")
        ))
    }
};

run_sync!(pipeline);  // Ejecución bloqueante para scripts
```

---

## Pipeliner vs Jenkins Pipeline DSL

Pipeliner proporciona una alternativa nativa en Rust a Jenkins Pipeline con ventajas significativas:

### Comparación de Sintaxis

| Característica | Jenkins Pipeline | Pipeliner |
|----------------|------------------|-----------|
| **Lenguaje** | DSL basado en Groovy | Rust nativo |
| **Type Safety** | Tipado dinámico | Verificación de tipos en tiempo de compilación |
| **Soporte IDE** | Limitado | Soporte completo Rust (IntelliJ, VSCode) |
| **Testing** | Scripted, limitado | TDD/BDD con testing nativo de Rust |
| **Ejecución** | Solo JVM | Cualquier runtime de Rust (local, Docker, K8s) |
| **Dependencias** | Jenkins + plugins | Sin dependencias externas |

### Definición de Pipeline

**Jenkins Pipeline (Groovy):**
```groovy
pipeline {
    agent any
    environment {
        VERSION = '1.0.0'
    }
    parameters {
        string(name: 'TARGET', defaultValue: 'production')
    }
    stages {
        stage('Build') {
            steps {
                sh 'cargo build --release'
            }
        }
        stage('Test') {
            steps {
                sh 'cargo test'
            }
            post {
                always {
                    archiveArtifacts artifacts: '**/target/**', allowEmptyArchive: true
                }
            }
        }
    }
}
```

**Pipeliner (Rust DSL):**
```rust
use pipeliner_core::prelude::*;

let pipeline = pipeline! {
    agent { any() }
    environment {
        ("VERSION", "1.0.0")
    }
    parameters {
        string("TARGET", "production")
    }
    stages {
        stage!("Build", steps!(
            sh!("cargo build --release")
        ))
        stage!("Test", steps!(
            sh!("cargo test")
        ))
    }
};
```

### Stages y Steps

**Jenkins:**
```groovy
stage('Deploy') {
    when {
        branch 'main'
    }
    steps {
        timeout(time: 5, unit: 'MINUTES') {
            retry(3) {
                sh './deploy.sh'
            }
        }
    }
    post {
        success { echo '¡Desplegado!' }
        failure { echo '¡Fallo!' }
    }
}
```

**Pipeliner:**
```rust
use pipeliner_core::prelude::*;

let deploy_stage = stage!("Deploy", steps!(
    timeout!(300, retry!(3, sh!("./deploy.sh")))
));

let pipeline = pipeline! {
    agent { docker("rust:latest") }
    stages {
        deploy_stage
    }
    post {
        success(echo!("¡Desplegado!")),
        failure(echo!("¡Fallo!"))
    }
};
```

### Ventajas Clave de Pipeliner

| Aspecto | Beneficio |
|---------|-----------|
| **Type Safety** | Errores detectados en compilación, no en ejecución |
| **Rendimiento** | Ejecución nativa Rust, sin overhead de JVM |
| **Testing** | Tests unitarios/integración con `cargo test` |
| **Portabilidad** | Ejecuta pipelines donde Rust se ejecute |
| **Tooling** | Usa el ecosistema Rust (cargo, clippy, rust-analyzer) |
| **Seguridad** | Garantías de seguridad de memoria, sin excepciones puntero nulo |
| **Concurrencia** | Concurrencia async/await sin miedos |
| **Versioning** | Versionado semántico de definiciones de pipeline |

### Migración desde Jenkins

Pipeliner está diseñado para ser familiar para usuarios de Jenkins mientras proporciona beneficios de Rust:

```rust
// Jenkins: agent any
AgentType::any()

// Jenkins: agent { docker 'rust:latest' }
AgentType::docker("rust:latest")

// Jenkins: sh 'comando'
Step::shell("comando")

// Jenkins: echo 'mensaje'
Step::echo("mensaje")

// Jenkins: timeout(time: 10, unit: 'MINUTES') { ... }
Step::timeout(std::time::Duration::from_secs(600), step_interno)

// Jenkins: retry(3) { ... }
Step::retry(3, step_interno)

// Jenkins: dir('ruta') { ... }
Step::dir(PathBuf::from("ruta"), step_interno)
```

---

## Referencia del DSL

### Definición de Pipeline

```rust
use rustline::prelude::*;

let pipeline = pipeline! {
    agent { any() },  // o docker("rust:latest"), kubernetes("default"), etc.
    environment {
        ("DEBUG", "1"),
        ("ENTORNO", "produccion")
    }
    parameters {
        string("VERSION", "1.0.0"),
        boolean("DEPLOY_HABILITADO", true)
    }
    stages {
        stage!("Build", steps!(
            sh!("cargo build --release"),
            sh!("cargo test --lib")
        ))
        stage!("Deploy", steps!(
            echo!("Desplegando versión ${VERSION}"),
            sh!("./deployar.sh ${VERSION}")
        ))
    }
};
```

### Stages y Steps

```rust
stage!("NombreStage", steps!(
    echo!("Un mensaje"),
    sh!("comando shell a ejecutar"),
    dir!("./ruta", steps!(
        sh!("comando en directorio")
    )),
    retry!(3, sh!("comando que puede fallar")),
    timeout!(30, sh!("comando largo"))
))
```

### Post-Condiciones

```rust
post {
    always(echo!("Siempre se ejecuta")),
    success(echo!("Se ejecuta en éxito")),
    failure(echo!("Se ejecuta en fallo")),
    unstable(echo!("Se ejecuta cuando es inestable"))
}
```

---

## Arquitectura

Pipeliner sigue **Arquitectura Hexagonal** (Puertos y Adaptadores) con clara separación de responsabilidades:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Capa de Aplicación                           │
│   PipelineExecutor │ PluginManager │ ExecutionStrategy              │
├─────────────────────────────────────────────────────────────────────┤
│                          Capa de Dominio                             │
│   Pipeline │ Stage │ Step │ Agent │ Parameters │ Environment        │
├─────────────────────────────────────────────────────────────────────┤
│                      Capa de Infraestructura                         │
│   DockerExecutor │ K8sExecutor │ PodmanExecutor │ CLI │ API REST    │
└─────────────────────────────────────────────────────────────────────┘
```

### Capa de Dominio

Entidades del núcleo de negocio:

- **Pipeline**: Estructura principal con stages, parámetros y entorno
- **Stage**: Stages individuales con ejecución condicional
- **Step**: Unidades ejecutables (shell, echo, retry, timeout, dir)
- **Agent**: Objetivos de ejecución (any, docker, kubernetes, podman)
- **Parameters**: Parámetros de entrada con validación de tipos

### Capa de Aplicación

Casos de uso y orquestación:

- **PipelineExecutor**: Ejecuta pipelines con manejo de errores adecuado
- **PluginRegistry**: Gestiona plugins y extensiones personalizadas
- **ExecutionStrategy**: Ejecución paralela, secuencial y matricial

### Capa de Infraestructura

Adaptadores externos:

- **DockerExecutor**: Ejecuta steps en contenedores Docker
- **K8sExecutor**: Ejecuta en pods de Kubernetes
- **PodmanExecutor**: Soporte nativo de Podman
- **API gRPC/REST**: Acceso programático
- **CLI**: Interfaz de línea de comandos

---

## Estructura de Crates

```
pipeliner/
├── crates/
│   ├── pipeliner-core/        # Tipos DSL de pipeline y validación
│   ├── pipeliner-executor/    # Motor de ejecución de pipelines
│   ├── pipeliner-infrastructure/ # Proveedores Docker, Podman, K8s
│   ├── pipeliner-worker/      # Programación de trabajos y pool workers
│   ├── pipeliner-events/      # Infraestructura de event sourcing
│   ├── pipeliner-api/         # Capa API gRPC y REST
│   ├── pipeliner-cli/         # Interfaz de línea de comandos
│   └── pipeliner-macros/      # Macros procedimentales para DSL
├── docs/                      # Documentación (Español e Inglés)
│   ├── USER_MANUAL.md
│   ├── architecture.md
│   ├── jenkins-sh-compatibility.md
│   ├── rust-script-integration.md
│   └── tdd-strategy.md
├── examples/                  # Ejemplos ejecutables
│   ├── mi_pipeline.rs         # Ejemplo en español con rust-script
│   ├── pipeline_example.rs    # Ejemplo de DSL en inglés
│   ├── docker_test.rs         # Integración Docker
│   └── podman_test.rs         # Integración Podman
└── tests/                     # Tests de integración
```

---

## Suite de Tests

Los 121 tests unitarios pasan en el workspace:

```bash
cd crates && cargo test --workspace
```

| Crate | Tests | Estado |
|-------|-------|--------|
| pipeliner-core | 43 | ✅ |
| pipeliner-executor | 22 | ✅ |
| pipeliner-infrastructure | 5 | ✅ |
| pipeliner-worker | 19 | ✅ |
| pipeliner-events | 15 | ✅ |
| pipeliner-api | 10 | ✅ |
| pipeliner-cli | 7 | ✅ |
| **Total** | **121** | **✅ Todos pasando** |

---

## Configuración

Crea un `pipeliner.yaml` para configuración avanzada:

```yaml
pipeline:
  name: mi-pipeline-ci
  agent:
    type: kubernetes
    image: rust:1.92

stages:
  - name: build
    steps:
      - name: compile
        type: shell
        command: cargo build --release
        retry: 3

execution:
  timeout: 3600
  parallel:
    stages:
      - build
      - test
```

---

## Anexo: API Programática

Aunque el **DSL es recomendado** para la mayoría de casos de uso, Pipeliner también proporciona una API programática para casos de uso avanzados que requieren control detallado.

### Usando LocalExecutor Directamente

Para escenarios que requieren manejo de ejecución personalizado:

```rust
use pipeliner_executor::LocalExecutor;
use pipeliner_core::{Pipeline, Stage, Step, AgentType};

#[tokio::main]
async fn main() {
    let pipeline = Pipeline::builder()
        .name("Mi Pipeline")
        .with_agent(AgentType::any())
        .with_stage(
            Stage::new("Build")
                .with_step(Step::echo("Iniciando build..."))
                .with_step(Step::shell("cargo build").with_retry(3))
        )
        .build();

    let executor = LocalExecutor::new();
    let results = executor.execute(&pipeline).await;

    for result in &results {
        println!("[{}] {} - {}", result.stage, result.success, result.output);
    }

    // Verificar si todos los steps fueron exitosos
    let todos_exitos = results.iter().all(|r| r.success);
    if todos_exitos {
        println!("✅ Pipeline completado exitosamente!");
    }
}
```

### API con Patrón Builder

Todos los tipos principales soportan métodos builder para construcción programática:

```rust
use pipeliner_core::{Pipeline, Stage, Step, AgentType};

let pipeline = Pipeline::builder()
    .name("Mi Pipeline")
    .description("Un pipeline de prueba")
    .with_agent(AgentType::docker("rust:1.92"))
    .with_stage(
        Stage::new("Build")
            .with_agent(AgentType::any()) // Sobrescribir agent del stage
            .with_step(
                Step::shell("cargo build --release")
                    .with_name("build-release")
                    .with_timeout(std::time::Duration::from_secs(300))
            )
    )
    .with_stage(
        Stage::new("Test")
            .with_step(Step::shell("cargo test").with_retry(2))
    )
    .build();
```

### Cuándo Usar API Programática

- Implementaciones de executor personalizadas
- Generación dinámica de pipelines basada en configuración
- Integración con frameworks async existentes
- Control detallado sobre resultados de ejecución

Para la mayoría de pipelines, el DSL con las macros `run!` o `run_sync!` es más simple y recomendado.

---

## Contribuir

¡Las contribuciones son bienvenidas! Por favor lee nuestras guías de contribución:

1. Haz fork del repositorio
2. Crea una rama de feature (`git checkout -b feature/caracteristica-increible`)
3. Commitea tus cambios siguiendo [Conventional Commits](https://www.conventionalcommits.org/)
4. Push a la rama (`git push origin feature/caracteristica-increible`)
5. Abre un Pull Request

### Configuración de Desarrollo

```bash
# Instalar dependencias
cd crates && cargo fetch

# Ejecutar todos los tests
cargo test --workspace

# Ejecutar lints
cargo clippy --workspace

# Construir documentación
cargo doc --no-deps
```

---

## Licencia

Licenciado bajo **MIT OR Apache-2.0**. Ver el archivo [LICENSE](LICENSE) para más detalles.

---

<div align="center">

**Construido con ❤️ usando Rust**

[Repositorio](https://github.com/Rubentxu/pipeliner) · [Issues](https://github.com/Rubentxu/pipeliner/issues)

</div>
