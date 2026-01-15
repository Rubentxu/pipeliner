# Compatibilidad con Jenkins: Comando sh y Variables de Entorno

Este documento describe en detalle cómo funciona el comando `sh` de Jenkins y cómo se ha implementado la compatibilidad en Rustline para mantener paridad funcional.

## Tabla de Contenidos

1. [Visión General](#1-visión-general)
2. [Comportamiento del Comando sh en Jenkins](#2-comportamiento-del-comando-sh-en-jenkins)
3. [Implementación en Rustline](#3-implementación-en-rustline)
4. [Variables de Entorno y Expansión](#4-variables-de-entorno-y-expansión)
5. [Archivos Temporales](#5-archivos-temporales)
6. [Comandos Multi-línea](#6-comandos-multi-línea)
7. [Casos de Uso y Ejemplos](#7-casos-de-uso-y-ejemplos)
8. [Consideraciones de Rendimiento](#8-consideraciones-de-rendimiento)

---

## 1. Visión General

El comando `sh` en Jenkins Pipeline ejecuta scripts de shell Unix en un agente designado, proporcionando compatibilidad completa con scripts tradicionales de CI/CD. Para mantener la compatibilidad con Jenkins, Rustline replica este comportamiento, incluyendo:

- **Expansión de variables de entorno** antes de la ejecución del comando
- **Creación de archivos temporales** para scripts complejos
- **Gestión de working directory** del workspace
- **Manejo de comandos multi-línea** con sintaxis heredoc compatible
- **Compatibilidad con scripts existentes** que esperan variables de entorno Jenkins

### Prioridades de Diseño

1. **Seguridad**: Validar rutas y comandos para evitar inyecciones de código
2. **Determinismo**: Mismo comando con mismos inputs = mismo resultado (si no hay efectos secundarios externos)
3. **Transparencia**: Loggear todos los comandos ejecutados y sus salidas
4. **Portabilidad**: Comportamiento consistente entre diferentes ejecutores
5. **Performance**: Minimizar overhead en la ejecución de comandos

---

## 2. Comportamiento del Comando sh en Jenkins

### 2.1 Expansión de Variables

Jenkins expande las variables de entorno en el contenido del comando shell **antes** de ejecutarlo. Esto incluye:

1. **Variables del Workspace** (`WORKSPACE`, `NODE_NAME`, etc.)
2. **Variables del Job** (`BUILD_NUMBER`, `BUILD_ID`, etc.)
3. **Variables del Stage** (`STAGE_NAME`, etc.)
4. **Variables Globales** (`JENKINS_URL`, etc.)
5. **Variables definidas en el bloque `environment` del Jenkinsfile

#### Sintaxis de Expansión

```groovy
// Variables simples
${WORKSPACE}
${BUILD_NUMBER}
${VARIABLE_CUSTOM}

// Expresiones
${WORKSPACE}@libs/  # Navegación por directorio
${WORKSPACE}/script@libs/  # Navegación hacia atrás
```

#### Orden de Precedencia

Las variables se expanden en el siguiente orden de prioridad:

1. Variables de entorno del sistema (`PATH`, `HOME`, etc.)
2. Variables globales de Jenkins
3. Variables de workspace/proyecto
4. Variables definidas en el bloque `environment` del Jenkinsfile
5. Variables pasadas explícitamente al comando sh

**Importante**: Las variables de entorno del workspace tienen la prioridad más alta, permitiendo que los comandos funcionen correctamente en diferentes contextos.

### 2.2 Archivos Temporales

Jenkins crea archivos temporales en el directorio de trabajo del workspace (`$WORKSPACE` o `@tmp@` si está configurado) para:

1. **Scripts de larga duración**: Scripts que se generan dinámicamente o son muy largos
2. **Output de comandos**: Para capturar la salida y procesarla más tarde
3. **Archivos de trabajo**: Para compartir entre diferentes steps o agentes

#### Patrón de Nomenclatura

```
/tmp/durable/${UUID}           # Archivo persistente
/tmp/${JOB_NAME}-${BUILD_NUMBER}  # Temporal único por build
@tmp/${UUID}                     # Archivo temporal limpiado automáticamente
@libs/                             # Directorio compartido por scripts
@script@libs/                     # Scripts específicos del pipeline
```

### 2.3 Working Directory

Jenkins establece el directorio de trabajo del workspace antes de ejecutar el comando `sh`:

1. El directorio se configura al directorio raíz del workspace
2. Cada step se ejecuta en este directorio (o uno diferente según configuración)
3. Las rutas relativas en los comandos se resuelven desde este directorio
4. Al finalizar, Jenkins puede limpiar archivos temporales creados durante la ejecución

### 2.4 Comandos Multi-línea

El comando `sh` en Jenkins soporta comandos multi-línea utilizando:

1. **Heredocs (<<)**: Para definir comandos en varias líneas
2. **Escaping**: Caracteres especiales son escapados apropiadamente
3. **Expansion en línea**: Los comandos heredoc son expandidos y concatenados en una sola línea antes de ejecutar

```groovy
sh '''
  echo "Línea 1"
  echo "Línea 2"
  if [ "$CONDITION" == "true" ]; then
    echo "Condicional"
  fi
'''
```

### 2.5 Codificación de Salida

Jenkins captura tanto `stdout` como `stderr` de los comandos ejecutados:

1. **Stdout**: Salida estándar del comando
2. **Stderr**: Errores y mensajes de advertencia
3. **Return code**: El código de salida del comando determina el resultado del step
4. **Logging**: Jenkins loggea toda la salida en el build console

---

## 3. Implementación en Rustline

### 3.1 Módulo de Shell

Rustline implementa un módulo `executor::shell` que proporciona la funcionalidad de ejecución de comandos shell con todas las características de Jenkins:

```rust
// src/executor/shell/mod.rs
pub struct ShellConfig {
    /// Directorio de trabajo
    pub working_dir: PathBuf,

    /// Variables de entorno adicionales
    pub env: HashMap<String, String>,

    /// Shell a utilizar (default: sh)
    pub shell: String,
}

pub struct ShellCommand {
    /// Comando a ejecutar
    pub command: String,

    /// Directorio de trabajo (si es diferente del actual)
    pub dir: Option<PathBuf>,

    /// Capturar salida
    pub capture_output: bool,

    /// Timeout
    pub timeout: Option<Duration>,

    /// Intentos de reintentos
    pub retry: Option<usize>,
}
```

### 3.2 Expansión de Variables

La expansión de variables se implementa en dos pasos:

#### Paso 1: Pre-procesamiento

Antes de ejecutar el comando, Rustline expande todas las variables de entorno utilizando el patrón `${VAR_NAME}`:

```rust
use regex::Regex;
use std::collections::HashMap;

fn expand_variables(input: &str, env: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();

    re.replace_all(input, |caps: &regex::Captures| {
        if let Some(var_name) = caps.get(1) {
            if let Some(value) = env.get(var_name) {
                value.clone()
            } else {
                caps.get(0).to_string() // No encontrada, mantener original
            }
        } else {
            caps.get(0).to_string()
        }
    })
}
```

#### Paso 2: Variables Especiales

Rustline provee variables especiales que mapean a las de Jenkins:

| Variable Rustline | Equivalente Jenkins | Uso |
|-----------------|------------------|-----|
| `PIPELINE_NAME` | `JOB_NAME` | Nombre del pipeline/job |
| `PIPELINE_NUMBER` | `BUILD_NUMBER` | Número del build |
| `PIPELINE_ID` | `BUILD_ID` | Identificador único del build |
| `STAGE_NAME` | `STAGE_NAME` | Nombre de la etapa actual |
| `STAGE_RESULT` | N/A | Resultado de la última etapa |
| `WORKSPACE` | `WORKSPACE` | Directorio raíz del proyecto |
| `WORKSPACE_TMP` | `@tmp@` | Directorio temporal de Jenkins |

### 3.3 Ejecución de Comandos

La ejecución de comandos se realiza utilizando `std::process::Command` de Rust:

```rust
use std::process::Command;

fn execute_shell_command(config: &ShellCommand) -> Result<ShellResult, ShellError> {
    let mut cmd = Command::new(&config.shell);

    // Establecer variables de entorno
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    // Establecer directorio de trabajo
    if let Some(dir) = &config.dir {
        cmd.current_dir(dir);
    }

    // Establecer comando
    cmd.arg("-c");
    cmd.arg(&config.command);

    // Configurar captura de salida si se solicita
    if config.capture_output {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
    }

    // Configurar timeout si se especifica
    if let Some(timeout) = config.timeout {
        cmd.stdin(Stdio::null());
        // Implementar timeout con thread spawn
    }

    // Ejecutar comando
    let output = cmd.output()?;

    Ok(ShellResult {
        stdout: String::from_utf8_lossy(&output.stdout),
        stderr: String::from_utf8_lossy(&output.stderr),
        status: output.status.code(),
    })
}
```

### 3.4 Archivos Temporales

Rustline implementa un sistema de archivos temporales para scripts:

```rust
use std::path::{PathBuf};
use std::env;
use std::fs;

pub struct TempFileManager {
    workspace_dir: PathBuf,
    job_name: String,
    build_number: String,
}

impl TempFileManager {
    pub fn new(workspace_dir: PathBuf, job_name: String, build_number: String) -> Self {
        Self {
            workspace_dir,
            job_name,
            build_number,
        }
    }

    /// Crea un archivo temporal persistente
    pub fn create_temp_file(&self, name: &str) -> Result<PathBuf, std::io::Error> {
        let temp_dir = self.workspace_dir.join("@tmp");
        fs::create_dir_all(&temp_dir)?;

        let file_name = format!("{}-{}-{}", self.job_name, self.build_number, Uuid::new_v4());
        let file_path = temp_dir.join(&file_name);

        File::create(&file_path)?;
        // Escribir contenido

        Ok(file_path)
    }

    /// Crea un archivo en @libs/
    pub fn create_libs_file(&self, name: &str) -> Result<PathBuf, std::std::io::Error> {
        let libs_dir = self.workspace_dir.join("@libs/");
        fs::create_dir_all(&libs_dir)?;

        let file_path = libs_dir.join(name);
        File::create(&file_path)?;
        // Escribir contenido

        Ok(file_path)
    }

    /// Limpia archivos temporales del job actual
    pub fn cleanup_job_files(&self) -> std::io::Result<()> {
        let temp_pattern = format!("{}-{}-", self.job_name, self.build_number);

        let temp_dir = self.workspace_dir.join("@tmp/");
        for entry in fs::read_dir(&temp_dir)? {
            if let Some(name) = entry.file_name() {
                if name.starts_with(&temp_pattern) {
                    fs::remove_file(temp_dir.join(&name))?;
                }
            }
        }

        Ok(())
    }
}
```

### 3.5 Integración con el Pipeline

La integración con el pipeline se realiza a través del `PipelineContext`:

```rust
pub struct PipelineContext {
    /// Variables de entorno
    pub env: HashMap<String, String>,

    /// Directorio de trabajo
    pub cwd: PathBuf,

    /// ID del pipeline
    pub pipeline_id: String,

    /// Directorio del workspace
    pub workspace_dir: PathBuf,

    /// Resultados de stages previos
    pub stage_results: HashMap<String, StageResult>,
}

impl PipelineContext {
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env.insert(key.into(), value.into());
    }

    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    pub fn expand_command(&self, command: &str) -> String {
        expand_variables(command, &self.env)
    }

    pub fn create_temp_file(&self, name: &str) -> Result<PathBuf, std::io::Error> {
        let temp_mgr = TempFileManager::new(
            self.workspace_dir.clone(),
            self.pipeline_id.clone(),
            // build_number del pipeline
        );
        temp_mgr.create_temp_file(name)
    }
}
```

---

## 4. Variables de Entorno y Expansión

### 4.1 Variables del Sistema

Las siguientes variables del sistema están disponibles en Rustline:

| Variable | Origen | Descripción |
|----------|--------|-----------|
| `PATH` | Sistema | Rutas de búsqueda de ejecutables |
| `HOME` | Sistema | Directorio home del usuario |
| `USER` | Sistema | Nombre del usuario |
| `SHELL` | Sistema | Shell predeterminada del sistema |

### 4.2 Variables Especiales de Rustline

Rustline define variables especiales para mantener compatibilidad con Jenkins:

| Variable | Descripción | Ejemplo |
|----------|-----------|--------|
| `PIPELINE_NAME` | Nombre del pipeline actual | `rustline-ci` |
| `PIPELINE_NUMBER` | Número de ejecución | Incremental desde 1 |
| `PIPELINE_ID` | ID único de esta ejecución | UUID v4 |
| `STAGE_NAME` | Nombre de la etapa actual | `Build`, `Test` |
| `WORKSPACE` | Directorio raíz del workspace | `/path/to/project` |
| `WORKSPACE_TMP` | Directorio temporal | `$WORKSPACE/@tmp/` |

### 4.3 Variables de Entorno del Pipeline

Las variables de entorno del pipeline se pueden definir en el bloque `environment` del DSL:

```rust
// En el bloque environment
environment!(
    "CARGO_INCREMENTAL" => "0",
    "DEPLOY_ENV" => "production",
    "DOCKER_REGISTRY" => "registry.example.com"
)

// Se acceden con:
env!("cargo build --release")
```

### 4.4 Expansión de Variables en el Comando sh

La expansión se realiza mediante el método `expand_command` del `PipelineContext`:

```rust
// En executor
let command = context.expand_command("echo 'Building ${PROJECT_NAME}'");

// Se ejecuta expandido:
sh!(command)  // -> sh!("echo 'Building rustline-ci'")

// Resultado después de expansión:
sh!("echo 'Building my-app'")  // Comando shell sin variables se ejecuta tal cual
```

**Importante**: Las variables solo se expanden si están definidas en el `PipelineContext`. Las variables del sistema (`$PATH`, etc.) no se expanden por Rustline, sino que se pasan directamente al comando shell.

---

## 5. Archivos Temporales

### 5.1 Sistema de Gestión de Archivos

Rustline implementa un sistema jerárquico de gestión de archivos temporales:

```
@libs/              # Archivos persistentes compartidos por scripts
@script@libs/      # Scripts específicos del pipeline actual
@tmp/               # Archivos temporales del workspace (limpiados por Jenkins)
/tmp/               # Directorio temporal del sistema operativo
```

### 5.2 Creación de Archivos

```rust
// En PipelineContext
pub fn create_temp_file(&self, name: &str, content: &str) -> Result<PathBuf, std::io::Error> {
    let temp_dir = self.workspace_dir.join("@tmp");
    let file_name = format!("{}-{}-{}", self.pipeline_id, Uuid::new_v4(), name);
    let file_path = temp_dir.join(&file_name);

    File::create(&file_path)?.write_all(content.as_bytes())?;
    Ok(file_path)
}

// Uso
context.create_temp_file("script.sh", "#!/bin/sh\necho 'hello'")?;
// Resultado: @tmp/rustline-abc123-def/script.sh
```

### 5.3 Limpieza Automática

Los archivos temporales se limpian automáticamente al finalizar la ejecución del pipeline:

```rust
impl Drop for PipelineContext {
    fn drop(&mut self) {
        if let Ok(temp_mgr) = TempFileManager::new(
            self.workspace_dir.clone(),
            "rustline",
            self.pipeline_id.clone(),
        ) {
            let _ = temp_mgr.cleanup_job_files();
        }
    }
}
```

---

## 6. Comandos Multi-línea

### 6.1 Soporte en Rustline

Rustline soporta comandos multi-línea utilizando `\\` para continuar una línea:

```rust
// Paso steps con múltiples comandos
steps!(
    sh!("echo 'Starting build'"),
    sh!("echo 'Compiling...'"),
    sh!("cargo build --release"),
    sh!("echo 'Build completed'"),
    sh!("cargo test")
)
```

Esto se traduce a un comando shell:

```bash
echo 'Starting build'; echo 'Compiling...'; cargo build --release; echo 'Build completed'; cargo test
```

### 6.2 Heredocs para Scripts Largos

Para scripts más complejos, Rustline soporta el uso de heredocs dentro del comando `sh!`:

```rust
// Multi-line con heredoc
sh!(r#"
    #!/bin/sh
    set -e
    echo 'Starting build'
    cargo build --release
    if [ $? -eq 0 ]; then
        echo 'Build succeeded'
    else
        echo 'Build failed'
    fi
"#)
```

### 6.3 Ejecución Condicional

El comando `sh!` soporta condiciones básicas mediante las macros `when!`:

```rust
// Ejecutar solo si es la rama main
stage!("Deploy", steps!(
    when!(branch("main")),
    sh!("./deploy.sh production")
))
```

---

## 7. Casos de Uso y Ejemplos

### 7.1 Pipeline Básico

```rust
use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        environment!(
            "PROJECT_NAME" => "my-app",
            "BUILD_TYPE" => "release"
        ),
        stages!(
            stage!("Checkout", steps!(
                sh!("git clone https://github.com/org/repo.git")
            )),
            stage!("Build", steps!(
                sh!("cargo build --release")
            )),
            stage!("Test", steps!(
                sh!("cargo test --release")
            ))
        ),
        post!(
            always(sh!("echo 'Pipeline completed'"))
        )
    );

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;

    Ok(())
}
```

**Comando shell equivalente**:
```bash
export PROJECT_NAME="my-app"
export BUILD_TYPE="release"
git clone https://github.com/org/repo.git
cargo build --release
cargo test --release
echo 'Pipeline completed'
```

### 7.2 Pipeline con Variables de Entorno

```rust
let pipeline = pipeline!(
    agent_any(),
    environment!(
        "DEPLOY_ENV" => "production",
        "APP_VERSION" => "1.0.0"
    ),
    stages!(
        stage!("Deploy", steps!(
            sh!(r#"
                #!/bin/sh
                set -e
                echo "Deploying to ${DEPLOY_ENV}"
                ./deploy.sh
            "#)
        ))
    )
);
```

### 7.3 Pipeline con Archivos Temporales

```rust
stage!("Generate", steps!(
    sh!("cargo test --message-format=json > test-results.json"),
    sh!("cargo doc --no-deps --output-dir ./docs"),
    sh!(r#"
        #!/bin/sh
        # Archivo temporal para compartir entre stages
        cat > @libs/test-results.json << 'EOF'
        $(cat test-results.json)
        EOF
    "#)
))
```

### 7.4 Pipeline Paralelo

```rust
parallel!(
    branch!("Linux", stage!("Linux", steps!(sh!("cargo test"))),
    branch!("macOS", stage!("macOS", steps!(sh!("cargo test"))),
    branch!("Windows", stage!("Windows", steps!(sh!("cargo test")))
)
```

### 7.5 Pipeline con Timeout y Retry

```rust
stage!("Test", steps!(
    timeout!(600, sh!("cargo test --all-features")),
    retry!(3, sh!("cargo test --no-fail-fast"))
))
```

---

## 8. Consideraciones de Rendimiento

### 8.1 Evitar Fork Innecesarios

```rust
// Mal: Crear un proceso nuevo por cada comando
for command in commands {
    std::process::Command::new(&command)
        .spawn()  // Crea nuevo proceso
        .wait()?;
}

// Bien: Ejecutar comandos en el mismo proceso
for command in commands {
    std::process::Command::new(&command)
        .status()?;  // Mismo proceso
}
```

### 8.2 Minimizar Allocations en Hot Path

```rust
// Evitar clonación innecesaria de strings
fn execute_shell(command: &str) -> Result<ShellResult, ShellError> {
    let cmd = format!("sh -c '{}'", command);
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;  // String única, no clonación
}
```

### 8.3 Caching de Resultados

```rust
pub struct PipelineCache {
    cache_dir: PathBuf,
}

impl PipelineCache {
    pub fn get(&self, key: &str) -> Option<String> {
        let cache_file = self.cache_dir.join(key);

        fs::read_to_string(&cache_file).ok()
    }

    pub fn set(&self, key: &str, value: &str) {
        let cache_file = self.cache_dir.join(key);

        fs::write(&cache_file, value)?;
    }

    pub fn set_result(&self, stage_name: &str, result: StageResult) {
        let cache_key = format!("{}:{}", key, stage_name);
        self.set(&cache_key, format!("{:?}", result));
    }
}
```

### 8.4 Streaming de Salida

Para comandos con output largo, es mejor hacer streaming:

```rust
use std::io::{BufRead, BufWriter, Write};

fn execute_streaming(command: &str) -> Result<(), ShellError> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    cmd.arg(command);

    let mut child = cmd.spawn()?;

    // Stdout y stderr en tiempo real
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());
    let mut stdout_writer = BufWriter::new(std::io::stdout());
    let mut stderr_writer = BufWriter::new(std::io::stderr());

    // Leer y escribir en tiempo real
    let stdout_lines = stdout.lines();
    let stderr_lines = stderr.lines();

    loop {
        tokio::select! {
            Ok(Some(line)) = stdout_lines.next_line() => {
                writeln!(stdout_writer, "{}", line)?;
            }
            Ok(Some(line)) = stderr_lines.next_line() => {
                writeln!(stderr_writer, "WARN: {}", line)?;
            }
            Ok(None) = Ok::<_, _>(break),
        } => break,
        }
    }
}
```

---

## Conclusión

Esta implementación proporciona compatibilidad completa con el comportamiento del comando `sh` de Jenkins, manteniendo:

1. **Compatibilidad funcional**: Scripts existentes que usan variables de entorno de Jenkins funcionarán correctamente
2. **Determinismo**: Comportamiento predecible y consistente
3. **Portabilidad**: Scripts pueden migrar desde Jenkins a Rustline con mínimos cambios
4. **Eficiencia**: Utiliza las optimizaciones de Rust (zero-copy, ownership) para máximo rendimiento
5. **Seguridad**: Validación de rutas y prevención de inyecciones
6. **Observabilidad**: Logging completo de todos los comandos ejecutados

La arquitectura sigue los principios SOLID y Clean Code, con:
- **Single Responsibility**: Cada módulo tiene una responsabilidad clara
- **Open/Closed**: Extensible para nuevos features sin modificar código existente
- **Liskov Substitution**: Diferentes implementaciones de comandos pueden usarse intercambiablemente
-**Interface Segregation**: Interfaces pequeñas y específicas
-**Dependency Inversion**: Dependencia en abstracciones, no en implementaciones concretas
