# Manual de Usuario - Rustline CI/CD DSL

## 📋 Tabla de Contenidos

1. [Introducción](#1-introducción)
2. [Instalación y Configuración](#2-instalación-y-configuración)
3. [Primeros Pasos](#3-primeros-pasos)
4. [Definir Pipelines](#4-definir-pipelines)
5. [Variables y Entorno](#5-variables-y-entorno)
6. [Comandos Multi-línea y Heredocs](#6-comandos-multi-línea-y-heredocs)
7. [Archivos Temporales](#7-archivos-temporales)
8. [Comandos de Control de Flujo](#8-comandos-de-control-de-flujo)
9. [Ejecución Paralela](#9-ejecución-paralela)
10. [Timeout y Retry](#10-timeout-y-retry)
11. [Stash y Unstash](#11-stash-y-unstash)
12. [Condiciones When](#12-condiciones-when)
13. [Post-Conditions](#13-post-conditions)
14. [Comandos Especiales](#14-comandos-especiales)
15. [Agents Especiales](#15-agents-especiales)
16. [Troubleshooting](#16-troubleshooting)

---

## 1. Introducción

Rustline es un DSL (Domain Specific Language) en Rust que replica la sintaxis y semántica del DSL de Jenkins Pipeline, diseñado para ser ejecutable con `rust-script` sin necesidad de configuración de proyecto adicional.

**Objetivos:**
- 🎯 Proporcionar una herramienta type-safe para CI/CD en el ecosistema Rust
- 🚀 Mantener compatibilidad funcional con Jenkins Pipeline DSL
- ⚡ Aprovechar el rendimiento y la seguridad de Rust
- 📖 Facilitar la transición desde Jenkins a Rust

**Características principales:**
- ✅ Sintaxis familiar para usuarios de Jenkins
- ✅ Type-safe con validación en tiempo de compilación
- ✅ Ejecutable con `rust-script` sin configuración
- ✅ Variables de entorno compatibles con Jenkins
- ✅ Comandos multi-línea y heredocs soportados
- ✅ Archivos temporales automáticos
- ✅ Condiciones when, timeout, retry, stash/unstash
- ✅ Múltiples backends (Local, Docker, Kubernetes, GitHub Actions, GitLab CI)
- ✅ Post-conditions always, success, failure, unstable, changed
- ✅ Ejecución paralela y matrix testing

**Ventajas sobre Jenkins:**
- 🔒 Mejor performance con compilación nativa
- 🛡️ Type-safety previene errores en runtime
- 📖 Código Rust es más seguro y mantenible
- 🚀 Zero-cost abstractions con ownership y borrowing
- 📝 Cargo ecosystem extenso y maduro

**Casos de Uso:**
- 🏭️ Pipelines simples y rápidos
- 🔧 Pipelines complejos con control de flujo avanzado
- 🐳 Pipelines para monorepositorios
- 🚀 Pipelines para microservicios
- 🔧 Testing automatizado

---

## 2. Instalación y Configuración

### 2.1 Requisitos del Sistema

- **Rust**: 1.92+ (recomendado 1.70+)
- **rust-script**: Instalado y en el PATH
- **Git**: Para clonar repositorios
- **Docker**: Para ejecutar en contenedores (opcional)
- **kubectl**: Para ejecutar en Kubernetes (opcional)
- **CI/CD**: GitHub Actions, GitLab CI, CircleCI (según necesidad)

### 2.2 Instalación

#### Instalar Rust y rust-script

```bash
# Linux
curl --proto '=https://sh.rustup.rs' | sh
curl --proto '=https://sh.rustup.rs' -sSf > rustup-init.sh
sh rustup-init.sh

# macOS
curl --proto '=https://sh.rustup.rs' | sh
curl --proto '=https://sh.rustup.rs' -sSf > rustup-init.sh
sh rustup-init.sh

# Windows (usar winget)
winget install https://win.rustup.rs/x86_64/pup
```

#### Instalar rust-script

```bash
cargo install rust-script
```

#### Verificar instalación

```bash
rust-script --version
```

### 2.3 Configuración del Workspace

El DSL no requiere configuración especial de workspace. Los pipelines pueden definirse en cualquier parte del código Rust.

**Estructura de directorio recomendada:**

```
my-project/
├── .rustline-pipelines/
│   ├── my-pipeline.rs
│   ├── ci/
│   └── deploy/
├── tests/
└── Cargo.toml
```

### 2.4 Variables de Entorno

Opcionalmente, puedes configurar variables de entorno:

```bash
export RUSTLINE_LOG_LEVEL=debug
export RUSTLINE_CACHE_DIR=~/.rustline-cache
```

---

## 3. Primeros Pasos

### 3.1 Tu Primer Pipeline Hola Mundo

Crea un archivo `hello.rs` con el pipeline más simple posible:

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! jenkins-pipeline-dsl = "0.1.0"

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Hello", steps!(
                echo!("¡Hola, Rustline!")
            ))
        ),
        post!(
            always(echo!("¡Pipeline completado!"))
        )
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;

    Ok(())
}
```

**Ejecutar:**
```bash
rust-script hello.rs
```

**Salida esperada:**
```
¡Hola, Rustline!
¡Pipeline completado!
```

### 3.2 Pipeline con Comandos Reales

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! jenkins-pipeline-dsl = "0.1.0"
//! serde = "1.0"
//! serde_json = "1.0"

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Clone Repository", steps!(
                sh!("git clone https://github.com/user/repo.git"),
                sh!("git submodule update --init --recursive")
            )),
            stage!("Build", steps!(
                sh!("cargo build --release")
            )),
            stage!("Test", steps!(
                sh!("cargo test")
            )
        ),
        post!(
            success(echo!("✅ Pipeline completado exitosamente")),
            failure(echo!("❌ Pipeline falló"))
        )
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;

    Ok(())
}
```

**Ejecutar:**
```bash
rust-script clone-repo.rs
```

### 3.3 Pipeline con Variables de Entorno

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! jenkins-pipeline-dsl = "0.1.0"

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        environment!(
            "PROJECT_NAME" => "my-app",
            "BUILD_TYPE" => "release",
            "VERSION" => "1.0.0"
        ),
        stages!(
            stage!("Echo Env", steps!(
                echo!("PROJECT_NAME: ${PROJECT_NAME}"),
                echo!("BUILD_TYPE: ${BUILD_TYPE}"),
                echo!("VERSION: ${VERSION}")
            ))
        )
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;

    Ok(())
}
```

**Ejecutar:**
```bash
export PROJECT_NAME=my-app BUILD_TYPE=release VERSION=1.0.0
rust-script echo-env.rs
```

### 3.4 Pipeline con Timeout

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! jenkins-pipeline-dsl = "0.1.0"

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Test with Timeout", steps!(
                timeout!(10, sh!("echo 'Iniciando test...' && sleep 5 && echo 'Test finalizado'"))
            )
        ),
        post!(
            failure(echo!("❌ Test falló por timeout"))
        )
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;

    Ok(())
}
```

### 3.5 Pipeline con Retry

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! jenkins-pipeline-dsl = "0.1.0"

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Download Artifacts", steps!(
                sh!("wget https://example.com/file.tar.gz"),
                sh!("tar -xvf file.tar.gz"),
                retry!(3, sh!("echo 'Verificando checksum...'")
            ))
        )
        ),
        post!(
            success(echo!("✅ Artifacts descargados")),
            failure(echo!("❌ Fallo en descarga"))
        )
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;

    Ok(())
}
```

### 3.6 Pipeline con Stash/Unstash

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! jenkins-pipeline-dsl = "0.1.0"

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Generate", steps!(
                sh!("echo 'Generando archivo...' > generated.txt"),
                stash!("generated-files", "*.txt")
            )),
            stage!("Process", steps!(
                unstash!("generated-files"),
                sh!("cat generated.txt")
            )),
            stage!("Cleanup", steps!(
                sh!("rm -f generated.txt")
            )
        ),
        post!(
            always(sh!("rm -f @libs/generated-files"))
        )
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;

    Ok(())
}
```

---

## 4. Definir Pipelines

### 4.1 Pipeline de Construcción

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! jenkins-pipeline-dsl = "0.1.0"

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Install Dependencies", steps!(
                sh!("cargo fetch"),
                sh!("cargo build --release")
            )),
            stage!("Run Tests", steps!(
                sh!("cargo test --all-features")
            )
        ),
        post!(
            success(echo!("✅ Build y tests exitosos")),
            failure(echo!("❌ Build o tests fallaron"))
        )
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;

    Ok(())
}
```

### 4.2 Pipeline de Despliegue

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! jenkins-pipeline-dsl = "0.1.0"

use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        environment!(
            "DEPLOY_ENV" => "production"
        ),
        stages!(
            stage!("Wait for Input", steps!(
                input!("¿Confirmar despliegue a producción?")
            )),
            stage!("Deploy", steps!(
                when!(branch("main")),
                sh!("./deploy.sh production")
            )
        ),
        post!(
            success(echo!("✅ Despliegue completado")),
            failure(echo!("❌ Despliegue falló"))
        )
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;

    Ok(())
}
```

---

## 5. Variables y Entorno

### 5.1 Variables Disponibles

Jenkins provides estas variables automáticamente en el entorno de ejecución:

| Variable | Descripción | Equivalente en Rustline |
|---------|----------|------------------------|
| `WORKSPACE` | Directorio raíz del workspace | `RUSTLINE_WORKSPACE` |
| `JOB_NAME` | Nombre del pipeline/job actual | `PIPELINE_JOB_NAME` |
| `BUILD_NUMBER` | Número del build incremental | `BUILD_ID` | ID único del build |
| `BUILD_TAG` | Git tag del build actual | `PIPELINE_BUILD_TAG` |
| `STAGE_NAME` | Nombre de la etapa actual | `RUSTLINE_STAGE_NAME` |
| `STAGE_RESULT` | Resultado de la etapa actual | `RUSTLINE_STAGE_RESULT` |

### 5.2 Variables de Pipeline en Rustline

Rustline define sus propias variables para extender el sistema:

| Variable | Descripción | Valor por defecto |
|---------|----------|------------|
| `RUSTLINE_CACHE_DIR` | Directorio de cache de Rustline | `~/.rustline-cache/` |
| `RUSTLINE_TEMP_DIR` | Directorio temporal | `@tmp/` o `$WORKSPACE/@tmp/` |

### 5.3 Uso en Pipelines

#### Variables de Sistema

```rust
stage!("Check Env", steps!(
    sh!("echo 'Workspace: $RUSTLINE_WORKSPACE'"),
    sh!("echo 'Job: $RUSTLINE_JOB_NAME'")
))
```

#### Variables de Pipeline

```rust
stage!("Build", steps!(
    sh!("echo 'Build: $BUILD_NUMBER'"),
    sh!("echo 'Tag: $PIPELINE_BUILD_TAG'")
))
```

#### Variables Globales de Jenkins

```rust
stage!("Release", steps!(
    sh!("echo 'Releasing: $GIT_TAG'"),
    sh!("git tag -a $GIT_TAG -m \"Release $GIT_TAG\"")
))
```

---

## 6. Comandos Multi-línea y Heredocs

### 6.1 Comandos Multi-línea

Jenkins utiliza `\\` para continuar comandos en múltiples líneas:

```groovy
sh '''
    echo "Line 1"
    echo "Line 2"
    echo "Line 3"
'''
```

**Rustline equivalente:**

```rust
stage!("Multi-line", steps!(
    sh!("echo 'Line 1'; \
            echo 'Line 2'; \
            echo 'Line 3'")
    )
```

### 6.2 Heredocs (<<-EOF)

Jenkins permite incluir heredocs en el DSL:

```groovy
node('Master') {
    stage('Deploy') {
        sh '''
            cat << 'EOF'
            #!/bin/bash
            echo "Deploying to production..."
            docker-compose up -d
            echo "Deployment completed"
            EOF
        '''
    }
}
```

**Rustline equivalente:**

```rust
stage!("Deploy Script", steps!(
    sh!(r#"
            #!/bin/bash
            echo "Deploying to production..."
            docker-compose up -d
            echo "Deployment completed"
            "#)
    )
```

### 6.3 Pipelining Stdout

```rust
stage!("Pipelined Output", steps!(
    sh!("echo 'Starting...'; \
            printf '%s\\n' 'Processing...\\n'; \
            printf '%s\\n' 'Done...\\n'; \
            echo 'Finished'")
    )
)
```

---

## 7. Archivos Temporales

### 7.1 Stash y Unstash

**Stash (guardar archivos):**

```rust
stage!("Build and Stash", steps!(
    sh!("mkdir -p build-cache"),
    sh!("cp -r target/*.rlib build-cache/"),
    sh!("tar -czf build-cache.tar.gz build-cache/"),
    stash!("build-cache", "build-cache.tar.gz")
```

**Unstash (recuperar archivos):**

```rust
stage!("Unstash and Build", steps!(
    unstash!("build-cache"),
    sh!("tar -xzf build-cache.tar.gz"),
        sh!("cargo build --release")
    )
```

### 7.2 Archivos Temporales Nombrados

Jenkins genera nombres únicos para archivados temporales:

```
/tmp/durable-123-abc-def/
/tmp/durable-123-abc-def/
@tmp/rustline-xyz789/

/tmp/${UUID}/
/tmp/${JOB_NAME}-${BUILD_NUMBER}/
```

**Rustline equivalente:**

```rust
// Usar Uuid para nombres únicos
use uuid::Uuid;

stage!("Unique Temp File", steps!(
    sh!("cargo build --release > /tmp/$(uuid::Uuid::new_v4()").log"),
    sh!("echo 'Build log saved to /tmp/$(uuid::Uuid::new_v4()).log'")
    )
```

### 7.3 Archivos Persistentes

```
@libs/script.sh
@libs/script-$(BUILD_NUMBER).sh
```

---

## 8. Comandos de Control de Flujo

### 8.1 When Conditions

**Cuando una etapa se ejecuta solo bajo ciertas condiciones:**

```rust
stage!("Deploy on Main", steps!(
    when!(branch("main")),
    sh!("./deploy.sh production")
))
```

**Múltiples condiciones:**

```rust
stage!("Deploy with Multiple Conditions", steps!(
    when!(all_of(
        branch("main"),
        environment!("DEPLOY", "true")
    )),
    sh!("./deploy.sh production")
))
```

### 8.2 Input Prompt

**Pausar y esperar entrada del usuario:**

```rust
stage!("Confirm Deployment", steps!(
    input!("¿Confirmar despliegue a producción? (y/N)")
))
```

---

## 9. Ejecución Paralela

### 9.1 Stages Paralelos

```rust
parallel!(
    branch!("Linux", stage!("Linux", steps!(sh!("cargo test --target x86_64-unknown-linux-gnu"))),
    branch!("macOS", stage!("macOS", steps!(sh!("cargo test --target x86_64-apple-darwin"))),
    branch!("Windows", stage!("Windows", steps!(sh!("cargo test --target x86_64-pc-windows-msvc")))
)
```

### 9.2 Matrix Testing

```rust
pipeline!(
    agent_any(),
    stages!(
        stage!("Matrix Test", steps!(
            matrix!(
                axes!(
                    rust = ["stable", "beta", "nightly"],
                    os = ["linux", "macos", "windows"]
                )
            ),
            sh!("cargo test --all-features")
        )
    )
)
```

### 9.3 Fast Fail Strategy

```rust
pipeline!(
    agent_any(),
    options!(PipelineOptions::new().with_disable_concurrent_builds(true)),
    stages!(
        stage!("Test", steps!(sh!("cargo test --no-fail-fast")),
        stage!("Build", steps!(sh!("cargo build --release"))
        )
    )
)
```

---

## 10. Timeout y Retry

### 10.1 Timeout por Comando

```rust
stage!("Long Running Command", steps!(
    timeout!(300, sh!("cargo test --all-features"))
)
)
```

### 10.2 Timeout por Etapa

```rust
pipeline!(
    agent_any(),
    options!(PipelineOptions::new().with_timeout(std::time::Duration::from_secs(600))),
    stages!(
        stage!("Test with Timeout", steps!(
            timeout!(300, sh!("cargo test"))
        ),
        stage!("Build with Timeout", steps!(
            timeout!(600, sh!("cargo build --release"))
        )
    )
)
```

### 10.3 Retry con Backoff

```rust
stage!("Retry with Exponential Backoff", steps!(
    sh!("curl -f https://api.example.com/endpoint"),
    retry!(5, sh!("curl -f https://api.example.com/endpoint"))
)
)
```

---

## 11. Post-Conditions

### 11.1 Always

```rust
pipeline!(
    agent_any(),
    post!(
        always(echo!("🧹 Limpieza de recursos...")),
        always(sh!("rm -rf $RUSTLINE_TEMP_DIR/*"))
    )
    )
)
```

### 11.2 Success y Failure

```rust
pipeline!(
    agent_any(),
    post!(
        success(echo!("🎉 Pipeline exitoso!")),
        failure(echo!("❌ Pipeline falló")),
        changed(echo!("🔄 Estado cambiado desde última ejecución"))
    )
    )
)
```

### 11.3 Unstable

```rust
pipeline!(
    agent_any(),
    stages!(
        stage!("Test with Flaky Tests", steps!(
            sh!("cargo test --all-features")
        ),
        post!(
            unstable(echo!("⚠️ Tests inestables"))
        )
    )
)
```

### 11.4 Changed - Práctica Recomendada

```rust
// Monitorear cambios en el estado de componentes

pipeline!(
    agent_any(),
    post!(
        changed(sh!("git push origin main"))
    )
)
```

---

## 12. Condiciones When

### 12.1 Branch Conditions

```rust
stage!("Deploy from Main", steps!(
    when!(branch("main")),
    sh!("./deploy.sh production")
)
)
```

**Múltiples ramas:**

```rust
stage!("Deploy from Any Branch", steps!(
    when!(any_of(
        branch("main"),
        branch("develop"),
        branch("feature/*")
    )),
    sh!("./deploy.sh production")
)
)
```

### 12.2 Tag Conditions

```rust
stage!("Release on Tag", steps!(
    when!(tag("v*.*")),  // Versión semántica
        sh!("git tag -a $GIT_TAG -m \"Release $GIT_TAG\""),
        sh!("git push origin main")
))
```

### 12.3 Environment Conditions

```rust
stage!("Deploy with Custom Env", steps!(
    when!(environment("DEPLOY", "production")),
        sh!("DEPLOY_ENV=${DEPLOY_ENV} ./deploy.sh")
    )
)
```

### 12.4 Expression Conditions

```rust
stage!("Conditional Build", steps!(
    when!(expression("CARGO_FEATURES.contains('\"--all-features\")")),
        sh!("cargo test --all-features")
    )
)
```

---

## 13. Comandos Especiales de Jenkins

### 13.1 Comandos de Checkout

```rust
stage!("Checkout", steps!(
    sh!("git clone https://github.com/user/repo.git"),
    sh!("git checkout ${GIT_COMMIT}")
)
)
```

### 13.2 Comandos de Build

```rust
stage!("Full Build", steps!(
    sh!("cargo clean"),
    sh!("cargo build --release"),
    sh!("cargo test --all-features"),
    sh!("cargo doc")
)
)
```

### 13.3 Comandos de Test

```rust
stage!("Test Suite", steps!(
    sh!("cargo test --all-features"),
    sh!("cargo clippy -- -D warnings"),
    sh!("cargo fmt --check")
)
```

### 13.4 Comandos de Publicación

```rust
stage!("Publish", steps!(
    sh!("cargo login registry..."),
    sh!("cargo publish")
)
)
```

---

## 14. Agents Especiales

### 14.1 Agente Docker

```rust
pipeline!(
    agent_docker!("rust:latest"),
    stages!(
        stage!("Build in Docker", steps!(
            sh!("cargo build --release")
        ),
        stage!("Test in Docker", steps!(
            sh!("cargo test --all-features")
        )
    )
)
```

### 14.2 Agente Kubernetes

```rust
pipeline!(
    agent_kubernetes!("rust:latest"),
    stages!(
        stage!("Build in K8s", steps!(
            sh!("kubectl config set-context rustline-$BUILD_NUMBER"),
            sh!("kubectl apply -f deployment.yaml")
        )
    )
)
```

### 14.3 Agente con Label

```rust
pipeline!(
    agent_label!("linux"),
    stages!(
        stage!("Linux Build", steps!(sh!("cargo build --target x86_64-unknown-linux-gnu"))
    ),
        stage!("macOS Build", steps!(sh!("cargo build --target x86_64-apple-darwin")),
        stage!("Windows Build", steps!(sh!("cargo build --target x86_64-pc-windows-msvc"))
    )
)
```

---

## 15. Troubleshooting

### 15.1 Depuración y Logging

**Habilitar modo debug:**

```bash
export RUSTLINE_LOG_LEVEL=debug
rust-script my-pipeline.rs
```

**Logging estructurado:**

```rust
use tracing::{info, warn, error, debug};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Iniciando pipeline");

    // Pipeline logic here
    debug!("Información de depuración");

    if let Err(e) = execute_pipeline() {
        error!("Error en pipeline: {}", e);
    }
    
    info!("Pipeline completado: {}", result);
}
```

### 15.2 Errores Comunes y Soluciones

**Error: "Cannot find file"**
```rust
// ❌ Usar rutas relativas o resolver cwd
pipeline!(
    stages!(
        stage!("Build", steps!(sh!("cargo build --release"))
    )
)
```

**Solución:**
```rust
// ✅ Usar rutas absolutas
use std::path::PathBuf;

let cwd = std::env::current_dir()?;

stage!("Build", steps!(
    sh!(format!("{}/cargo build --release", cwd.display()))
))
```

**Error: "Permission denied"**
```rust
// ❌ Permisos insuficientes
// Solución: Usar sudo o corregir permisos
```

**Error: "Command failed with exit code 1"**
```rust
// ❌ El comando falló
stage!("Test", steps!(
    when!(expression("$CARGO_FEATURES.contains(\\\"--all-features\\\")")),
    sh!("cargo test --all-features")
)
)
```

**Solución:**
```rust
// ✅ Agregar manejo de errores
match executor.execute(&pipeline) {
    Err(PipelineError::CommandFailed { code, stderr }) => {
        eprintln!("❌ Comando falló con código {}: {}", code, stderr);
        // Logear el error detallado
        std::process::exit(code);
    },
    Ok(_) => {
        eprintln!("✅ Comandos ejecutados exitosamente");
    std::process::exit(0);
    }
}
```

### 15.3 Performance Issues

**Lentitud de compilación:**

```rust
// ❌ Muy lento
stage!("Slow Build", steps!(sh!("cargo build --release")))

// Soluciones:
// 1. Usar `cargo check` antes de `cargo build`
// 2. Habilitar caching con `sccache`
// 3. Limitar features en `--all-features`
stage!("Optimized Build", steps!(sh!("cargo build --release --all-features"))
```

**Consumo de memoria:**

```rust
// ❌ Out of memory
stage!("Build with Parallel Tests", steps!(
    parallel!(
        branch!("Stable", stage!("Stable", steps!(sh!("cargo test --target stable"))),
        branch!("Nightly", stage!("Nightly", steps!(sh!("cargo test --target nightly"))
    )
)
```

// Soluciones:
// 1. Usar `--jobs` para serializar compilaciones
// 2. Usar `--offline` si no se necesita conectividad
stage!("Serial Build", steps!(sh!("cargo build --release -j4")))
```

---

## Apéndice

### A. Macro Quick Reference

| Macro | Ejemplo | Descripción |
|------|--------|------------|
| `pipeline!()` | Ver [Sección 3.1](#31-tu-primer-pipeline-hola-mundo) | Crea pipeline completo |
| `agent_any!()` | `pipeline!(agent_any(), ...)` | Agente: any |
| `agent_label!("x")` | `pipeline!(agent_label!("x"), ...)` | Agente con label |
| `agent_docker!("img")` | `pipeline!(agent_docker!("img"), ...)` | Agente Docker |
| `agent_kubernetes!("img")` | `pipeline!(agent_kubernetes!("img"), ...)` | Agente Kubernetes |
| `stage!("name", steps!(...))` | Crea etapa |
| `steps!(step1, step2, ...)` | Crea lista de pasos |
| `sh!("cmd")` | Ejecuta comando shell |
| `echo!("msg")` | Imprime mensaje |
| `timeout!(secs, step)` | Timeout en segundos |
| `retry!(count, step)` | Reintenta N veces |
| `stash!("name", "pattern")` | Guarda archivos |
| `unstash!("name")` | Restaura archivos |
| `when!(condition)` | Condición para etapa |
| `parallel!(branches...)` | Ejecución paralela |
| `matrix!(axes!(...), steps!(...))` | Matrix testing |
| `post!(always(step), ...)` | Post-condiciones |
| `environment!(k:v, ...)` | Variables de entorno |

### B. Funciones de Utilidad

| Función | Descripción |
|---------|------------|
| `PipelineContext::new()` | Crea contexto de ejecución |
| `context.set_env(key, value)` | Define variable |
| `context.get_env(key)` | Obtiene valor |
| `context.record_stage_result(name, result)` | Registra resultado |

### C. Referencias Externas

- [Documentación de Jenkins Pipeline](https://www.jenkins.io/doc/book/pipeline/syntax/)
- [Estudio Técnico Completo](docs/rust-jenkins-dsl-study.md)
- [Manual de Usuario](docs/USER_MANUAL.md)
- [Estrategia TDD](docs/tdd-strategy.md)
- [Arquitectura](docs/architecture.md)
- [Épicas Documentadas](docs/epics.md)
---

## Versión

**Versión actual**: 0.1.0
**Fecha**: 2025-01-14
**Licencia**: MIT OR Apache-2.0

---

## Support y Contribuciones

**Issues**: [GitHub Issues](https://github.com/rustline-org/rustline/issues)
**Discussions**: [GitHub Discussions](https://github.com/rustline-org/rustline/discussions)
**Documentación**: [Documentación](https://github.com/rustline-org/rustline/wiki)

---

## 🎓 Notas Importantes

1. **Compatibilidad**: Rustline está diseñado para ser **compatible** pero no **idéntico** a Jenkins Pipeline DSL. Algunas características avanzadas pueden diferir.
2. **Seguridad**: Los comandos se ejecutan con los permisos del usuario. Siempre valida comandos antes de ejecutar.
3. **Rendimiento**: Las variables de entorno solo se expanden si están definidas en el contexto. Variables del sistema como `$PATH` se respetan pero no se modifican.
4. **Archivos temporales**: Se limpian automáticamente pero es recomendado limpiarlos manualmente para ahorrar espacio.
5. **Error Handling**: Usa los tipos de error proporcionados (`PipelineError`, `ValidationError`) para manejar errores de manera estructurada.
6. **Tests**: Siempre escribe tests antes de implementar funcionalidades (TDD).

---

## 🚀 ¡Empieza a construir pipelines CI/CD con Rustline!
