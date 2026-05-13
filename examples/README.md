# Pipeliner — Ejemplos y Guía de Uso

Esta carpeta contiene ejemplos funcionales que demuestran todas las capacidades de **Pipeliner**, un motor de pipelines escritos en Rust que funciona de forma similar a como Groovy funciona en Jenkins.

## Tabla de Contenidos

- [Filosofía](#filosofía)
- [Instalación rápida](#instalación-rápida)
- [Estructura de la carpeta](#estructura-de-la-carpeta)
- [Comandos disponibles](#comandos-disponibles)
- [Ejemplo 01: Hola Mundo](#ejemplo-01-hola-mundo)
- [Ejemplo 02: Variables de entorno](#ejemplo-02-variables-de-entorno)
- [Ejemplo 03: Dependencias externas](#ejemplo-03-dependencias-externas)
- [Ejemplo 04: Build y Test](#ejemplo-04-build-y-test)
- [Ejemplo 05: Manejo de errores](#ejemplo-05-manejo-de-errores)
- [Pipeline JSON 01: Pipeline simple](#pipeline-json-01-pipeline-simple)
- [Pipeline JSON 02: Multi-stage](#pipeline-json-02-multi-stage)
- [Tests E2E automatizados](#tests-e2e-automatizados)
- [Cómo funciona internamente](#cómo-funciona-internamente)
- [Comparación con Jenkins/Groovy](#comparación-con-jenkinsgroovy)
- [FAQ](#faq)

---

## Filosofía

Pipeliner ejecuta **scripts Rust como pasos de pipeline**, de la misma forma que Jenkins ejecuta scripts Groovy. La diferencia fundamental:

| Jenkins/Groovy | Pipeliner/Rust |
|----------------|----------------|
| Script Groovy interpretado | Script Rust **compilado** a binario nativo |
| Tipado dinámico | Tipado estático + verificación en compilación |
| Dependencias via `@Grab` | Dependencias via comentarios `//! [dependencies]` |
| `Jenkinsfile` declarativo | `.rs` con `fn main()` — es Rust puro |
| Overhead de la JVM | Binario nativo, sin runtime |

El resultado: **tienes el poder completo de Rust** (acceso a sistema, librerías, tipado fuerte) con la ergonomía de un sistema de pipelines.

---

## Instalación rápida

```bash
# Clonar y compilar
git clone https://github.com/Rubentxu/pipeliner.git
cd pipeliner
cargo build

# El binario principal es target/debug/pipeliner
# Verificar instalación:
./target/debug/pipeliner --version
```

---

## Estructura de la carpeta

```
examples/
├── README.md              ← Este documento
├── run_e2e.sh             ← Suite de tests E2E automatizados
├── scripts/               ← Scripts Rust DSL (como Groovy en Jenkins)
│   ├── 01-hello.rs        ← Hola mundo básico
│   ├── 02-env-vars.rs     ← Variables de entorno del pipeline
│   ├── 03-with-deps.rs    ← Uso de dependencias externas
│   ├── 04-build-and-test.rs ← Simulación completa build + test
│   └── 05-error-handling.rs ← Manejo de errores y códigos de salida
└── pipelines/             ← Definiciones de pipeline en JSON
    ├── 01-simple.json     ← Pipeline simple con 2 stages
    └── 02-with-scripts.json ← Pipeline multi-stage con 3 stages
```

---

## Comandos disponibles

### `pipeliner script <archivo.rs>` — Ejecutar un script Rust

Compila y ejecuta un script `.rs` directamente. Es el equivalente a ejecutar un script Groovy en Jenkins.

```bash
# Uso básico
pipeliner script examples/scripts/01-hello.rs

# Con dependencias adicionales (se suman a las del manifest)
pipeliner script mi_script.rs -d serde -d tokio

# Con argumentos para el script (separados con --)
pipeliner script mi_script.rs -- --mi-arg valor
```

**Qué hace internamente:**
1. Lee el archivo `.rs`
2. Extrae dependencias de los comentarios `//! [dependencies]`
3. Genera un proyecto Cargo temporal
4. Compila con `cargo build --release`
5. Ejecuta el binario resultante
6. **Cachea** el binario compilado (la segunda ejecución es instantánea)

### `pipeliner run --file <pipeline.json>` — Ejecutar un pipeline

Ejecuta un pipeline definido en JSON con stages y steps.

```bash
# Ejecutar un pipeline desde archivo
pipeliner run --file examples/pipelines/01-simple.json

# Definición inline (JSON)
pipeliner run --definition '{"name":"test","stages":[{"name":"build","steps":[{"type":"echo","message":"hi"}]}]}'

# Con filtros y opciones
pipeliner run --file pipeline.json --stages build,test --output json --timeout 300

# Modo dry-run (validar sin ejecutar)
pipeliner run --file pipeline.json --dry-run
```

### `pipeliner validate --file <pipeline.json>` — Validar un pipeline

Verifica que la definición del pipeline es correcta sin ejecutarla.

```bash
pipeliner validate --file examples/pipelines/01-simple.json
# Output: Pipeline is valid
```

### `pipeliner check --file <pipeline.json>` — Comprobar sintaxis

Verifica la sintaxis del pipeline sin ejecutarlo.

```bash
pipeliner check --file examples/pipelines/01-simple.json
# Output: Pipeline syntax is correct
```

### `pipeliner init` — Crear un scaffold de pipeline

Genera un archivo `pipeline.json` con la estructura básica.

```bash
# Con nombre personalizado
pipeliner init --name "mi-pipeline" --output mi-pipeline.json

# Con valores por defecto (genera pipeline.json)
pipeliner init
```

### `pipeliner lint --file <pipeline.json>` — Lint de estilo

Analiza el pipeline buscando problemas de estilo y buenas prácticas.

### `pipeliner doc --file <pipeline.json>` — Generar documentación

Genera documentación del pipeline.

### `pipeliner export --file <pipeline.json> --format json` — Exportar

Exporta la definición del pipeline a diferentes formatos.

### `pipeliner completions --shell bash` — Autocompletado

Genera scripts de autocompletado para tu shell.

```bash
# Bash
pipeliner completions --shell bash >> ~/.bashrc

# Zsh
pipeliner completions --shell zsh >> ~/.zshrc
```

---

## Ejemplo 01: Hola Mundo

**Archivo:** `scripts/01-hello.rs`

### Objetivo

Demostrar el ejemplo más simple posible: un script Rust que imprime un mensaje. Es el "Hello World" de Pipeliner.

### Qué demuestra

- Ejecución básica de un script Rust vía CLI
- El shebang `#!/usr/bin/env pipeliner-run` (para ejecución directa con `chmod +x`)
- Primera compilación (lenta) vs ejecución desde cache (instantánea)

### Código

```rust
#!/usr/bin/env pipeliner-run
fn main() {
    println!("Hello from Pipeliner Rust DSL!");
}
```

### Ejecución

```bash
pipeliner script examples/scripts/01-hello.rs
```

### Salida esperada

```
Hello from Pipeliner Rust DSL!
```

### Notas

- La **primera ejecución** tarda 10-30 segundos porque compila el script con `cargo build --release`
- Las **ejecuciones posteriores** son instantáneas (el binario compilado se cachea)
- Si cambias el script, se recompila automáticamente

---

## Ejemplo 02: Variables de entorno

**Archivo:** `scripts/02-env-vars.rs`

### Objetivo

Mostrar cómo los scripts reciben el **contexto del pipeline** mediante variables de entorno, de la misma forma que en Jenkins los scripts reciben `BUILD_NUMBER`, `JOB_NAME`, etc.

### Qué demuestra

- Lectura de variables de entorno del pipeline (`PIPELINE_NAME`, `PIPELINE_STAGE`, `PIPELINE_STEP`)
- Acceso al sistema de archivos (`current_dir()`)
- Manejo de valores por defecto con `unwrap_or_else`

### Código

```rust
#!/usr/bin/env pipeliner-run
fn main() {
    let pipeline = std::env::var("PIPELINE_NAME").unwrap_or_else(|_| "unknown".to_string());
    let stage = std::env::var("PIPELINE_STAGE").unwrap_or_else(|_| "unknown".to_string());
    let step = std::env::var("PIPELINE_STEP").unwrap_or_else(|_| "unknown".to_string());
    println!("Pipeline: {}", pipeline);
    println!("Stage: {}", stage);
    println!("Step: {}", step);

    let cwd = std::env::current_dir().unwrap();
    println!("Working directory: {}", cwd.display());
}
```

### Ejecución

```bash
pipeliner script examples/scripts/02-env-vars.rs
```

### Salida esperada

```
Pipeline: unknown
Stage: unknown
Step: unknown
Working directory: /home/user/pipeliner
Hostname: localhost
```

### Variables de entorno disponibles

Cuando un script se ejecuta como parte de un pipeline (step tipo `script`), recibe:

| Variable | Descripción | Ejemplo |
|----------|-------------|---------|
| `PIPELINE_NAME` | Nombre del pipeline | `"build-pipeline"` |
| `PIPELINE_STAGE` | Nombre del stage actual | `"build"` |
| `PIPELINE_STEP` | Nombre del step actual | `"compile"` |
| `PIPELINE_ROOT` | Directorio raíz del pipeline | `"/workspace/project"` |
| `PIPELINE_PARAM_*` | Parámetros custom del pipeline | `PIPELINE_PARAM_VERSION=1.0` |

Cuando se ejecuta directamente (`pipeliner script`), estas variables no están definidas y el script muestra `"unknown"` como valor por defecto.

---

## Ejemplo 03: Dependencias externas

**Archivo:** `scripts/03-with-deps.rs`

### Objetivo

Demostrar cómo declarar y usar **dependencias de crates.io** dentro de un script, sin necesidad de crear un proyecto Cargo manual. Es el equivalente a `@Grab` de Groovy.

### Qué demuestra

- Declaración de dependencias via comentarios `//! [dependencies]`
- Uso de `serde_json` para generar salida estructurada
- Formato de manifest compatible con `Cargo.toml`

### Código

```rust
#!/usr/bin/env pipeliner-run
//! [dependencies]
//! serde_json = "1.0"

fn main() {
    let data = serde_json::json!({
        "pipeline": "build-pipeline",
        "status": "running",
        "steps": ["compile", "test", "deploy"]
    });
    println!("{}", serde_json::to_string_pretty(&data).unwrap());
}
```

### Ejecución

```bash
pipeliner script examples/scripts/03-with-deps.rs
```

### Salida esperada

```json
{
  "pipeline": "build-pipeline",
  "status": "running",
  "steps": [
    "compile",
    "test",
    "deploy"
  ]
}
```

### Formato del manifest

Las dependencias se declaran en comentarios al inicio del script:

```rust
#!/usr/bin/env pipeliner-run
//! [dependencies]
//! serde = "1.0"
//! serde_json = "1.0"
//! tokio = { version = "1.0", features = ["full"] }
//! reqwest = { version = "0.11", features = ["json"] }
```

Formatos soportados (idénticos a `Cargo.toml`):

```rust
//! nombr简单 = "versión"                        → Versión simple
//! serde = { version = "1.0", features = ["derive"] }  → Con features
//! tokio = { version = "1.0", features = ["full"] }    → Con features múltiples
```

También se pueden pasar dependencias extra desde la línea de comandos:

```bash
pipeliner script script.rs -d serde -d "tokio = { version = '1.0', features = ['full'] }"
```

### Secciones soportadas

```
//! [dependencies]        → Dependencias de producción
//! [dev-dependencies]    → Dependencias de desarrollo
//! [build-dependencies]  → Dependencias de build
```

---

## Ejemplo 04: Build y Test

**Archivo:** `scripts/04-build-and-test.rs`

### Objetivo

Simular un **pipeline CI/CD real** dentro de un único script Rust: etapa de build, etapa de test, y generación de un reporte JSON. Demuestra que un script puede orquestar múltiples operaciones como haría un `Jenkinsfile`.

### Qué demuestra

- Combinación de manifest dependencies + código Rust completo
- Ejecución de comandos del sistema via `std::process::Command`
- Generación de reportes estructurados (JSON)
- Organización lógica de stages dentro de un script

### Código

```rust
#!/usr/bin/env pipeliner-run
//! [dependencies]
//! serde_json = "1.0"

use std::process::Command;

fn main() {
    println!("=== Build Stage ===");

    let output = Command::new("echo")
        .arg("Compiling project...")
        .output()
        .expect("Failed to execute echo");
    println!("{}", String::from_utf8_lossy(&output.stdout));

    println!("=== Test Stage ===");
    let output = Command::new("echo")
        .arg("Running 42 tests... All passed!")
        .output()
        .expect("Failed to execute echo");
    println!("{}", String::from_utf8_lossy(&output.stdout));

    let report = serde_json::json!({
        "build": "success",
        "tests_passed": 42,
        "tests_failed": 0
    });
    println!("=== Report ===");
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
```

### Ejecución

```bash
pipeliner script examples/scripts/04-build-and-test.rs
```

### Salida esperada

```
=== Build Stage ===
Compiling project...

=== Test Stage ===
Running 42 tests... All passed!

=== Report ===
{
  "build": "success",
  "tests_failed": 0,
  "tests_passed": 42
}
```

### Qué representa

Este ejemplo es la base de un **pipeline CI real**. En un entorno de producción:

- `echo "Compiling project..."` sería `cargo build --release`
- `echo "Running 42 tests..."` sería `cargo test`
- El reporte JSON se enviaría a un dashboard o se archivaría como artefacto

---

## Ejemplo 05: Manejo de errores

**Archivo:** `scripts/05-error-handling.rs`

### Objetivo

Demostrar cómo manejar **errores en la ejecución** y propagar el código de salida correcto. Un pipeline debe poder fallar explícitamente cuando algo sale mal.

### Qué demuestra

- Ejecución de comandos que pueden fallar
- Inspección del estado de salida (`output.status.success()`)
- Captura de stderr del comando ejecutado
- Terminación con código de error (`std::process::exit(1)`)
- Pipeliner detecta el código de salida y reporta el fallo

### Código

```rust
#!/usr/bin/env pipeliner-run
use std::process::Command;

fn main() {
    println!("Attempting risky operation...");

    let output = Command::new("ls")
        .arg("/nonexistent/directory/that/should/not/exist")
        .output()
        .expect("Failed to execute ls");

    if output.status.success() {
        println!("Operation succeeded!");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Operation failed: {}", stderr.trim());
        std::process::exit(1);
    }
}
```

### Ejecución

```bash
pipeliner script examples/scripts/05-error-handling.rs
```

### Salida esperada

```
Attempting risky operation...
Operation failed: ls: cannot access '/nonexistent/directory/...': No such file or directory

Error: Script exited with failure (exit code: 1)
```

### Comportamiento clave

| Código de salida | Pipeliner interpreta |
|-----------------|---------------------|
| `0` | Paso exitoso |
| `1` o cualquier no-cero | Paso fallido |
| Timeout | El paso se cancela |

En un pipeline, si un step devuelve código no-cero, el pipeline se detiene y marca el stage como fallido.

---

## Pipeline JSON 01: Pipeline simple

**Archivo:** `pipelines/01-simple.json`

### Objetivo

Demostrar la definición de un pipeline en formato JSON con stages secuenciales y pasos simples (`echo` y `shell`).

### Qué demuestra

- Estructura mínima de un pipeline JSON
- Step tipo `echo` (imprime un mensaje)
- Step tipo `shell` (ejecuta un comando del sistema)
- Ejecución secuencial de stages

### Contenido

```json
{
  "name": "Simple Build",
  "stages": [
    {
      "name": "greet",
      "steps": [
        { "type": "echo", "message": "Starting pipeline..." }
      ]
    },
    {
      "name": "build",
      "steps": [
        { "type": "shell", "command": "echo 'Building project...'" }
      ]
    }
  ]
}
```

### Ejecución

```bash
pipeliner run --file examples/pipelines/01-simple.json
```

### Tipos de step disponibles

| Tipo | Descripción | Ejemplo |
|------|-------------|---------|
| `echo` | Imprime un mensaje | `{"type": "echo", "message": "Hello"}` |
| `shell` | Ejecuta un comando shell | `{"type": "shell", "command": "make build"}` |
| `script` | Ejecuta un script Rust | `{"type": "script", "content": "fn main() { ... }"}` |
| `retry` | Reintenta un step N veces | `{"type": "retry", "count": 3, "step": {...}}` |
| `timeout` | Step con timeout | `{"type": "timeout", "duration": 60, "step": {...}}` |
| `dir` | Cambia de directorio | `{"type": "dir", "path": "./src", "steps": [...]}` |
| `when` | Ejecución condicional | `{"type": "when", "condition": {...}, "steps": [...]}` |
| `log` | Log a nivel específico | `{"type": "log", "level": "info", "message": "..."}` |

---

## Pipeline JSON 02: Multi-stage

**Archivo:** `pipelines/02-with-scripts.json`

### Objetivo

Demostrar un pipeline con **3 stages** (prepare → build → test), cada uno con múltiples steps. Simula un flujo CI/CD real.

### Qué demuestra

- Pipeline con más de 2 stages
- Múltiples steps por stage
- Mezcla de tipos de step (`echo` + `shell`)
- Flags útiles: `--stages`, `--output`, `--dry-run`

### Ejecución

```bash
# Ejecutar todo el pipeline
pipeliner run --file examples/pipelines/02-with-scripts.json

# Solo ejecutar el stage "build"
pipeliner run --file examples/pipelines/02-with-scripts.json --stages build

# Solo validar (sin ejecutar)
pipeliner run --file examples/pipelines/02-with-scripts.json --dry-run

# Salida en formato JSON
pipeliner run --file examples/pipelines/02-with-scripts.json --output json

# Con timeout global
pipeliner run --file examples/pipelines/02-with-scripts.json --timeout 120
```

---

## Tests E2E automatizados

**Archivo:** `run_e2e.sh`

### Objetivo

Suite de tests **end-to-end** que verifica automáticamente que todos los comandos CLI, scripts y pipelines funcionan correctamente.

### Qué testea

| Sección | Tests | Qué verifica |
|---------|-------|-------------|
| CLI Basics | 4 | `--help`, `--version`, existencia de comandos |
| Script Execution | 5 | Cada script de ejemplo (01-05) |
| Pipeline JSON | 2 | Ambos pipelines JSON |
| CLI Commands | 6 | `validate`, `check`, `init`, errores |
| **Total** | **17** | |

### Ejecución

```bash
# Opción 1: Directamente
bash examples/run_e2e.sh

# Opción 2: Con permisos de ejecución
chmod +x examples/run_e2e.sh
./examples/run_e2e.sh
```

### Salida esperada

```
========================================
Pipeliner E2E Test Suite
========================================

--- CLI Basics ---
PASS: help shows successfully
PASS: help mentions script command
PASS: help mentions run command
PASS: version shows successfully

--- Rust Script Execution ---
PASS: hello script prints greeting
PASS: env-vars script shows pipeline context
PASS: deps script outputs JSON
PASS: build-test script runs stages
PASS: error-handling script exits with code 1

--- Pipeline JSON Execution ---
PASS: simple pipeline runs
PASS: multi-stage pipeline runs

--- CLI Commands ---
PASS: validate accepts valid pipeline
PASS: check accepts valid pipeline
PASS: init creates pipeline file
PASS: init file contains pipeline name
PASS: nonexistent script fails
PASS: non-rs file fails

========================================
Results: 17 passed, 0 failed, 0 skipped
========================================
```

### Cómo funciona

El script usa dos funciones de aserción:

- `assert_exit <desc> <expected_code> <cmd...>` — Verifica código de salida
- `assert_output <desc> <pattern> <cmd...>` — Verifica que la salida contiene un patrón

---

## Cómo funciona internamente

### Flujo de `pipeliner script archivo.rs`

```
archivo.rs
    │
    ▼
┌──────────────────────────┐
│ 1. Leer archivo           │
│ 2. Parsear manifest //!   │
│ 3. ¿Cache hit?            │
│    ├─ Sí → usar binario   │
│    └─ No → compilar       │
│       ├─ Generar Cargo.toml│
│       ├─ cargo build       │
│       └─ Cachear binario   │
│ 4. Ejecutar binario       │
│ 5. Capturar stdout/stderr │
│ 6. Retornar resultado     │
└──────────────────────────┘
    │
    ▼
  Salida + código de exit
```

### Flujo de `pipeliner run --file pipeline.json`

```
pipeline.json
    │
    ▼
┌──────────────────────────┐
│ 1. Parsear JSON           │
│ 2. Validar estructura     │
│ 3. Para cada stage:       │
│    └─ Para cada step:     │
│       ├─ echo → imprimir  │
│       ├─ shell → ejecutar │
│       └─ script → compilar│
│ 4. Recolectar resultados  │
│ 5. Reportar estado        │
└──────────────────────────┘
```

### Sistema de caché

Los scripts compilados se cachean en un directorio temporal basándose en un hash SHA1 del:

- Contenido del script
- Dependencias declaradas
- Ruta del script

Si no cambias el script, la segunda ejecución es **instantánea** (reutiliza el binario cacheado).

---

## Comparación con Jenkins/Groovy

### Jenkins (Groovy)

```groovy
pipeline {
    agent any
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
        }
    }
    post {
        failure {
            echo 'Build failed!'
        }
    }
}
```

### Pipeliner (Rust DSL)

```rust
#!/usr/bin/env pipeliner-run
//! [dependencies]
//! serde_json = "1.0"

use std::process::Command;

fn main() {
    // Stage: Build
    println!("=== Build Stage ===");
    let build = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .expect("Failed to run cargo build");

    if !build.success() {
        eprintln!("Build failed!");
        std::process::exit(1);
    }

    // Stage: Test
    println!("=== Test Stage ===");
    let test = Command::new("cargo")
        args(["test"])
        .status()
        .expect("Failed to run cargo test");

    if !test.success() {
        eprintln!("Tests failed!");
        std::process::exit(1);
    }

    println!("Pipeline completed successfully!");
}
```

### Pipeliner (JSON Pipeline)

```json
{
  "name": "CI Pipeline",
  "stages": [
    {
      "name": "Build",
      "steps": [
        { "type": "shell", "command": "cargo build --release" }
      ]
    },
    {
      "name": "Test",
      "steps": [
        { "type": "shell", "command": "cargo test" }
      ]
    }
  ]
}
```

### ¿Cuándo usar cada uno?

| Enfoque | Ideal para |
|--------------------|
| **Rust DSL** (`.rs`) | Lógica compleja, condicionales, acceso a APIs, procesamiento de datos |
| **JSON Pipeline** | Pipelines declarativos simples, configuración, flujos predecibles |

---

## FAQ

### ¿La primera ejecución es lenta?

Sí. La primera vez que ejecutas un script, Pipeliner compila un proyecto Cargo temporal con `cargo build --release`. Dependiendo de las dependencias, puede tardar 10-60 segundos. Las ejecuciones posteriores son instantáneas gracias al caché.

### ¿Puedo usar cualquier crate de crates.io?

Sí. Declara las dependencias en los comentarios `//! [dependencies]` igual que en `Cargo.toml`:

```rust
//! [dependencies]
//! reqwest = { version = "0.11", features = ["json"] }
//! serde = { version = "1.0", features = ["derive"] }
//! tokio = { version = "1.0", features = ["full"] }
```

### ¿Puedo ejecutar un script directamente sin `pipeliner script`?

Sí, si le das permisos de ejecución y el shebang está presente:

```bash
chmod +x examples/scripts/01-hello.rs
./examples/scripts/01-hello.rs
```

### ¿Dónde se cachean los binarios compilados?

En `/tmp/pipeliner-script-binaries/`. Se puede limpiar con:

```bash
rm -rf /tmp/pipeliner-script-binaries/ /tmp/pipeliner-script-cache/
```

### ¿Cómo paso variables de entorno a un script?

Variables del sistema:

```bash
export MY_VAR=valor
pipeliner script mi_script.rs
```

Dentro del pipeline (JSON), las variables de entorno se inyectan automáticamente:

```bash
pipeliner run --file pipeline.json --env production
```

### ¿Puedo usar `async/await` en mis scripts?

Sí, pero necesitas declarar `tokio` como dependencia y usar `#[tokio::main]`:

```rust
#!/usr/bin/env pipeliner-run
//! [dependencies]
//! tokio = { version = "1.0", features = ["full"] }

#[tokio::main]
async fn main() {
    println!("Async Rust script!");
    // Puedes usar .await aquí
}
```

### ¿Qué versión de Rust necesito?

Rust 1.92+ (Edition 2024). Verifica con:

```bash
rustc --version
```
