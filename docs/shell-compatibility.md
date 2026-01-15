# Implementación de Compatibilidad con Jenkins: Comando sh y Variables de Entorno

Este documento describe la implementación de compatibilidad con el comando `sh` de Jenkins en Rustline, incluyendo:

1. Gestión de archivos temporales
2. Expansión de variables de entorno
3. Comandos multi-línea
4. Compatibilidad con scripts Jenkins Pipeline
5. Variables especiales de Jenkins

---

## 1. Visión General

El objetivo es lograr que pipelines definidos con el DSL de Rustline sean **completamente compatibles** con Jenkins Pipeline al ser ejecutados en Jenkins, manteniendo la misma semántica y funcionalidad.

### Principios Clave

1. **Determinismo**: Mismo input = mismo resultado en todos los entornos
2. **Pureza**: Efectos secundarios despreciables no afectan resultado
3. **Observabilidad**: Logging completo de todas las operaciones
4. **Reproducibilidad**: Ejecutar múltiples veces con mismo resultado
5. **Performance**: Minimizar overhead en la ejecución

---

## 2. Expansión de Variables de Entorno

### 2.1 Variables de Sistema

Jenkins utiliza estas variables del sistema para configurar el comportamiento:

| Variable | Descripción | Valor por defecto |
|----------|-----------|-----------|
| `WORKSPACE` | Directorio raíz del workspace | `PWD` |
| `NODE_NAME` | Nombre del nodo | `master` |
| `BUILD_ID` | ID único del build | `PIPELINE_NUMBER` | Número incremental |
| `BUILD_TAG` | Tag del build | `PIPELINE_ID` | ID único de esta ejecución |
| `STAGE_NAME` | Nombre de la etapa actual | `RUSTLINE_STAGE_NAME` | Nombre de la etapa (Build, Test, etc) |
| `STAGE_RESULT` | Resultado de la etapa actual | `SUCCESS`, `FAILURE`, `UNSTABLE`, `ABORTED` |
| `GIT_BRANCH` | Rama Git actual | `main`, `develop`, `feature/*` |
| `GIT_COMMIT` | SHA del commit actual | `git rev-parse HEAD` |
| `GIT_AUTHOR` | Autor del commit actual | `git log -1 --pretty=format:'%an %ae <%s>'` |
| `GIT_MESSAGE` | Mensaje del commit actual | `git log -1 --pretty=format:'%s'` |
| `GIT_URL` | URL del repositorio Git | `git config --get remote.origin.url` |
| `JOB_NAME` | Nombre del job Jenkins actual | `rustline-ci` |
| `BUILD_URL` | URL del build | `BUILD_NUMBER` | Número del build incremental | |
| `RUN_CHANGES_DISPLAY` | Cambios desde el último build (en orden cronológico inverso) |
| `RUN_CHANGES_TEXT` | Lista de cambios desde el último build |

### 2.2 Variables de Pipeline

| Variable | Descripción | Uso típico |
|----------|-----------|-----------|
| `DEPLOY_ENV` | Entorno de despliegue | `production`, `staging`, `dev` |
| `APP_VERSION` | Versión de la aplicación | `1.0.0` |
| `CARGO_INCREMENTAL` | Número de build incremental (1, 2, 3...) | `1` |
| `DOCKER_REGISTRY` | Registro Docker para imágenes | `registry.example.com` |
| `DOCKER_TAG` | Tag de imagen Docker | `latest`, `v1.0.0` |
| `PIPELINE_DISPLAY_NAME` | Nombre para mostrar en logs | `my-app CI/CD Pipeline` |
| `PIPELINE_DISPLAY_URL` | URL para mostrar en logs | `https://example.com/build/123` |

### 2.3 Variables de Stage

| Variable | Descripción | Ejemplo |
|----------|-----------|-----------|
| `STAGE_NAME` | Nombre de la etapa | `Build`, `Test`, `Deploy`, etc. |
| `STAGE_RESULT` | Resultado de la etapa actual | `SUCCESS`, `FAILURE`, `UNSTABLE` | `ABORTED` |

### 2.4 Variables Especiales

| Variable | Descripción | Uso en Jenkins |
|----------|-----------|-----------|
| `SCM` | Fuente de control de código | `git` |
| `CREDENTIALS_ID` | ID de credenciales | `git` |
| `BUILD_TAG` | Etiqueta del build | `v1.0.0` |
| `CHANGE_ID` | ID único de cambio | `git rev-parse HEAD` |
| `CHANGE_TITLE` | Título del cambio | `feat: add feature` |
| `CHANGE_URL` | URL del cambio | `https://github.com/user/repo/pull/123` |
| `CHANGE_AUTHOR` | Autor del cambio | `git config --get user.name <email@domain.com>` |
| `CHANGE_MESSAGE` | Mensaje del cambio | `fix: critical bug` |

### 2.5 Variables Globales

| Variable | Descripción | Ejemplo |
|----------|-----------|-----------|
| `JENKINS_URL` | URL de Jenkins | `https://ci.jenkins.io`` |
| `JENKINS_HOME` | Directorio de Jenkins | `/var/jenkins_home` |
| `JENKINS_INSTANCE` | Instancia de Jenkins | `https://ci.jenkins.io/` |

---

## 3. Implementación en Rust

### 3.1 Módulo de Shell

```rust
// src/executor/shell/mod.rs

use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use crate::pipeline::{PipelineError, StageResult};
use tracing::{debug, info, warn, error};

/// Configuration for shell command execution
#[derive(Debug, Clone, Default)]
pub struct ShellConfig {
    /// Current working directory
    pub cwd: std::path::PathBuf,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Shell to use (default: sh)
    pub shell: String,

    /// Timeout in seconds
    pub timeout: Option<u64>,
}

impl ShellConfig {
    /// Creates a new shell configuration with defaults
    pub fn new() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_default(),
            env: std::env::vars().collect(),
            shell: "sh".to_string(),
            timeout: None,
        }
    }

    /// Sets the current working directory
    pub fn with_cwd(mut self, cwd: impl Into<std::path::PathBuf>) -> Self {
        self.cwd = cwd.into();
        self
    }

    /// Sets environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Sets shell to use
    pub fn with_shell(mut self, shell: impl Into<String>) -> Self {
        self.shell = shell.into();
        self
    }

    /// Sets timeout
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = if timeout == 0 { None } else { Some(timeout) };
        self
    }
}

/// Shell command executor
#[derive(Debug, Clone)]
pub struct ShellExecutor {
    config: Arc<ShellConfig>,
}

impl ShellExecutor {
    /// Creates a new shell executor
    pub fn new(config: ShellConfig) -> Self {
        Self { config: Arc::new(config) }
    }

    /// Executes a shell command
    pub fn execute_command(
        &self,
        command: &str,
        context: &PipelineContext,
    ) -> Result<ShellResult, PipelineError> {
        info!(command = %command, "Executing shell command");

        let output = self.execute_shell_command_impl(command, context)?;

        if !output.status.success() {
            return Err(PipelineError::CommandFailed {
                code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(ShellResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::fromenv8_lossy(&output.stderr).to_string(),
            status: output.status,
        })
    }

    /// Expands environment variables in a command string
    fn expand_variables(&self, input: &str, env: &HashMap<String, String>) -> String {
        let mut result = input.to_string();

        // Pattern to match ${VAR_NAME} or ${VAR_NAME:-default}
        let re = regex::Regex::new(r"\$\{[^}]+\}\}\}").unwrap();

        // Replace variables in order from most specific to least specific
        let mut matches: Vec<(std::cmp::Reverse, std::borrow::Cow<'_>, String, bool)> = Vec::new();

        for (pattern, is_default) in [
            // Environment variables
            (r"\$\{[^}]+\}\}", false),
            (r"\$\{[^}+:default\}\}", true),
            // Workspace variables
            (r"\$\{workspace}+\}\}\}", false),
            (r"\$\{node_name}+\}\}\}", false),
            // Stage variables
            (r"\$\{stage_name}+\}\}\}", false),
            // Pipeline variables
            (r"\$\{pipeline_id}+\}\}\}", false),
        // Job variables
            (r"\$\{build_number}+\}\}\}", false),
            // Build variables
            (r"\$\{build_tag}+\}\}\}", false),
            // Special Jenkins variables
            (r"\$\{BUILD_DISPLAY_NAME}+\}\}\}", false),
        (r"\$\{BUILD_DISPLAY_URL}+\}\}\}", false),
            (r"\$\{RUN_CHANGES_TEXT}+\}\}\}", false),
        // (r"\$\{RUN_CHANGES_DISPLAY}+\}\}\}\}", false),
            // Custom variables
        (r"\$\{CUSTOM_VAR}+\}\}\}", false),
        (r"\{\doble\${var}\}\}\} ", false),
            (r"\{\{escapado\}\\${var\}\}\}", false),
            (r"\${not_defined_var:-default\}\}\}", false),
            (r"\${var:-default\}\}\}", false),
        (r"\${var:?\}\}\}", false),
        (r"\${var}\}\}\} ", false),
            (r"\${var:=value\}\}\}", false),
            (r"\${var:+value\}\}\}", false),
            (r"\${var:-value\}\}\}", false),
        (r"\${var:++value\}\}\}", false),
            (r"\${var:=?value\}\}\}", false),
            (r"\$\{var:?value\}\}\}\}", false),
            (r"\$\{var:+value\}\}\} ", false),
        ] {
            if let Ok(matched) = re.captures()[1] {
                if let Some(var_name) = matched.name(1) {
                    let default_value = if is_default { "default".to_string() } else { "" };

                    if let Some(value) = matches.get(2) {
                        // Expand variable with value or use default
                        let expanded_value = if value.is_empty() {
                            default_value.clone()
                        } else {
                            value.clone()
                        };

                        result = result.replace(matched.as_str(), &expanded_value);
                    }
                }
            }

            result
        }
    }

    /// Captures stdout and stderr
    fn execute_shell_command_impl(
        &self,
        command: &str,
        context: &PipelineContext,
    ) -> Result<std::process::Output, PipelineError> {
        let cmd = Command::new(&self.config.shell)
            .arg("-c")
            .arg(command);

        // Apply environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        for (key, value) in &context.env {
                cmd.env(key, value);
            }

        // Set working directory
        let cwd = &self.config.cwd;
        if !cwd.as_os_str().is_empty() {
            cmd.current_dir(&cwd);
        }

        // Execute command
        debug!("Executing command: {:?}", command);
        let output = cmd.output()?;

        if let Err(e) = output {
            return Err(PipelineError::Io(e.to_string()));
        }

        output
    }
}
```

### 3.2 Archivos Temporales

```rust
// src/executor/shell/temp.rs

use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Write};
use uuid::Uuid;
use tracing::{info, warn, error};

/// Temporary file manager
pub struct TempFileManager {
    workspace_dir: PathBuf,
    pipeline_id: String,
}

impl TempFileManager {
    /// Creates a new temporary file manager
    pub fn new(workspace_dir: PathBuf, pipeline_id: String) -> Self {
        Self {
            workspace_dir,
            pipeline_id,
        }
    }

    /// Creates a persistent temporary file
    pub fn create_temp_file(
        &self,
        name: &str,
    content: &str,
    ) -> Result<(String, PathBuf), PipelineError> {
        let temp_dir = self.workspace_dir.join("@tmp");

        // Create @tmp directory if it doesn't exist
        fs::create_dir_all(&temp_dir)?;

        // Generate unique filename
        let file_name = format!("{}-{}-{}", name, self.pipeline_id, Uuid::new_v4());

        let file_path = temp_dir.join(&file_name);

        // Write content to file
        fs::write(&file_path, content)?;

        info!("Created temporary file: {:?}", file_path);

        Ok((file_name, file_path))
    }

    /// Creates a file in @libs/ directory
    pub fn create_libs_file(
        &self,
        name: &str,
        content: &str,
    ) -> Result<PathBuf, PipelineError> {
        let libs_dir = self.workspace_dir.join("@libs");
        fs::create_dir_all(&libs_dir)?;

        let file_path = libs_dir.join(name);
        fs::write(&file_path, content)?;

        info!("Created libs file: {:?}", file_path);

        Ok(file_path)
    }

    /// Cleanup all temporary files for this pipeline
    pub fn cleanup_pipeline_files(&self) -> Result<(), PipelineError> {
        // Remove @libs/ files for this pipeline
        let libs_dir = self.workspace_dir.join("@libs");
        let pattern = format!("{}-*", self.pipeline_id);

        if libs_dir.exists() {
            for entry in fs::read_dir(&libs_dir)? {
                if let Some(name) = entry.file_name() {
                    if name.contains(&self.pipeline_id) {
                        let path = libs_dir.join(&name);

                        if path.exists() {
                            info!("Removing temporary file: {:?}", path);
                            fs::remove_file(path)?;
                        }
                    }
                }
            }
        }

        // Remove @tmp/ files for this pipeline
        let temp_dir = self.workspace_dir.join("@tmp");
        let temp_pattern = format!("{}-*", self.pipeline_id);

        if temp_dir.exists() {
            for entry in fs::read_dir(&temp_dir)? {
                if let Some(name) = entry.file_name() {
                    if name.contains(&self.pipeline_id) {
                        let path = temp_dir.join(&name);

                        if path.exists() {
                            info!("Removing temporary file: {:?}", path);
                            fs::remove_file(path)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Drop for TempFileManager {
    fn drop(&mut self) {
        let _ = self.cleanup_pipeline_files();
    }
}
```

### 3.3 Working Directory Management

```rust
// src/executor/shell/workdir.rs

use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use tracing::{info, warn, error};
use crate::pipeline::PipelineError;

/// Working directory manager
pub struct WorkingDirManager {
    workspace_dir: PathBuf,
    job_name: String,
    build_number: String,
}

impl WorkingDirManager {
    /// Creates a new working directory for a pipeline execution
    pub fn new(workspace_dir: PathBuf, job_name: String, build_number: String) -> Self {
        Self {
            workspace_dir,
            job_name,
            build_number,
        }
    }

    /// Gets or creates the working directory for this job
    pub fn get_or_create(&self) -> Result<PathBuf, crate::pipeline::PipelineError> {
        // Use JOB_NAME variable
        let job_name = self.job_name.clone();

        // Create working directory under workspace
        let workspace_dir = &self.workspace_dir;

        // Determine subdirectory strategy
        let work_dir = match job_name.as_str() {
            "pr-branch" | "pr-merge" => workspace_dir.join("pr-branches"),

            "pr-branch" => {
                let branch = std::env::var("PR_NUMBER").unwrap_or("main".to_string());
                workspace_dir.join(format!("pr-branches/{}", branch)),
            }
            _ => workspace_dir.join(job_name),
        };

        // Create directory if it doesn't exist
        fs::create_dir_all(&work_dir)?;

        // Return working directory
        Ok(work_dir)
    }

    /// Cleans up after execution
    pub fn cleanup(&self) -> Result<(), PipelineError> {
        if work_dir.exists() {
            // Remove working directory after execution
            info!("Cleaning up working directory: {:?}", work_dir);
            fs::remove_dir_all(&work_dir)?;
        }
    }
}

impl Default for WorkingDirManager {
    fn default() -> Self {
        Self {
            workspace_dir: std::env::current_dir().unwrap_or_default(),
            job_name: "unknown".to_string(),
            build_number: "1".to_string(),
        }
    }
}
```

---

## 4. Integración con el Pipeline DSL

### 4.1 Uso del Módulo de Shell en PipelineExecutor

El módulo `shell` del executor se integra con el pipeline:

```rust
// En src/executor/local.rs

use super::shell::{ShellExecutor, ShellConfig, ShellCommand, ShellResult};

impl PipelineExecutor for LocalExecutor {
    /// Executes a stage using shell executor
    fn execute_stage(
        &self,
        stage: &Stage,
        context: &PipelineContext,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        let mut shell_executor = ShellExecutor::new(self.config.shell.clone());

        // Change to working directory if specified
        if let Some(agent) = stage.agent.as_ref() {
            if let AgentType::Docker(config) = agent.downcast_ref() {
                shell_executor = shell_executor.with_cwd(config.working_dir.clone());
            }
        }

        // Execute all steps in the stage
        for step in &stage.steps {
            shell_executor.execute_step(step, context)?;
        }

        Ok(StageResult::Success)
    }
}

/// Executes a single step using shell executor
fn execute_step(
    &self,
    step: &Step,
    context: &PipelineContext,
) -> Result<(), PipelineError> {
    match step.step_type {
        StepType::Shell { command } => {
            let cmd = self.expand_variables(&command, &self.config.env)?;

            if let Some(timeout) = self.config.timeout {
                shell_executor = shell_executor.with_timeout(std::time::Duration::from_secs(timeout as u64));
            }

            shell_executor.execute_shell_command(cmd, context)?;
        }
        _ => {
            // Other step types (echo, retry, timeout, etc.)
            unimplemented!("Step type not supported yet")
        }
    }
}
```

### 4.2 Variables Disponibles en el Contexto de Pipeline

El `PipelineContext` ya incluye estas variables por defecto:

```rust
pub struct PipelineContext {
    /// Environment variables
    pub env: HashMap<String, String>,

    /// Current working directory
    pub cwd: PathBuf,

    /// Pipeline ID
    pub pipeline_id: String,

    /// Resultados de stages previos
    pub stage_results: HashMap<String, StageResult>,

    /// Directorio del workspace
    pub workspace_dir: PathBuf,
}
```

Estas variables se llenan automáticamente desde el entorno del sistema y pueden accederse como:
```rust
context.get_env("WORKSPACE");  // -> "/var/jenkins/workspace"
context.get_env("JOB_NAME");    // -> "my-job"
context.get_env("BUILD_NUMBER");  // -> "42"
context.get_env("GIT_COMMIT");  // -> "abc123def"
```

### 4.3 Ejemplo de Pipeline Completo

```rust
#!/usr/bin/env rust-script
//! cargo
//! [dependencies]
//! jenkins-pipeline-dsl = "0.1.0"
//! serde = "1.0"
//! thiserror = "1.0"
//!
//! jenkins_pipeline_dsl = "0.1.0"

use jenkins_pipeline_dsl::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Checkout", steps!(
                sh!("git clone https://github.com/user/repo.git"),
                sh!("git submodule update --init --recursive")
            )),
            stage!("Build", steps!(
                sh!("cargo build --release --locked"),
                stage!("Test", steps!(
                    sh!("cargo test --all-features"),
                    sh!("cargo clippy -- -D warnings"),
                    sh!("cargo doc --no-deps --output-dir ./docs")
                )
            )),
            stage!("Deploy", steps!(
                sh!("cargo publish --registry crates-io"),
                post!(
                    success(sh!("echo '✅ Deployed successfully!'")),
                    failure(sh!("echo '❌ Deployment failed!'"))
                )
            )
        )
    );

    let executor = LocalExecutor::new()
        let result = executor.execute(&pipeline)?;

    match result {
        Ok(_) => println!("✅ Pipeline completed successfully"),
        Err(e) => {
            eprintln!("❌ Pipeline failed: {}", e);
            std::process::exit(1);
        }
    }
}
```

---

## 5. Casos de Uso y Ejemplos

### 5.1 Caso 1: Pipeline Básico

**Objetivo**: Ejecutar un pipeline simple con tres etapas (Checkout, Build, Test) sin variables especiales.

```rust
pipeline!(
    agent_any(),
    stages!(
        stage!("Checkout", steps!(
            sh!("git clone https://github.com/user/repo.git"),
            sh!("git submodule update --init --recursive")
        ),
        stage!("Build", steps!(
            sh!("cargo build --release"),
            stage!("Test", steps!(
                sh!("cargo test --release")
            )
        ),
        post!(
            always(sh!("echo 'Cleanup'"))
        )
    )
)
```

### 5.2 Caso 2: Pipeline con Variables de Entorno

**Objetivo**: Ejecutar un pipeline con variables de entorno específicas.

```rust
pipeline!(
    agent_any(),
    environment!(
        "DEPLOY_ENV" => "production",
        "APP_VERSION" => "1.0.0"
    ),
    stages!(
        stage!("Deploy", steps!(
            sh!("echo 'Deploying ${DEPLOY_ENV}'"),
            sh!("echo 'App version: ${APP_VERSION}'")
        )
        )
)
)
```

### 5.3 Caso 3: Pipeline con When Conditions

**Objetivo**: Ejecutar una etapa solo cuando se cumpla ciertas condiciones.

```rust
stage!("Deploy", steps!(
        when!(branch("main")),
        sh!("./deploy.sh production")
    ))
```

### 5.4 Caso 4: Pipeline con Timeout y Retry

**Objetivo**: Ejecutar comandos con timeout y reintentos automáticos.

```rust
stage!("Long Running Task", steps!(
        timeout!(3600, sh!("./run-long-task.sh"))
    )
```

### 5.5 Caso 5: Pipeline con Archivos Temporales

**Objetivo**: Guardar archivos entre etapas diferentes.

```rust
stage!("Generate", steps!(
        sh!("echo 'Generating files...'"),
        stash!("*.rs"),
        stage!("Test", steps!(
            unstash!("*.rs"),
            sh!("cargo test")
        )
    )
```

---

## 6. Tests de Compatibilidad

### 6.1 Unit Tests

```rust
#[cfg(test)]
mod shell_tests {
    use super::*;

    #[test]
    fn test_shell_config_creation() {
        let config = ShellConfig::new();
        assert_eq!(config.shell, "sh");
        assert_eq!(config.cwd, std::env::current_dir().unwrap_or_default());
        assert!(config.timeout.is_none());
    }

    #[test]
    fn test_shell_executor_creation() {
        let executor = ShellExecutor::new(ShellConfig::new());
        assert_eq!(executor.config.shell, "sh");
    }

    #[test]
    fn test_variable_expansion() {
        let env = std::collections::HashMap::new();
        env.insert("VAR1", "value1");
        env.insert("VAR2", "value2");

        let mut executor = ShellExecutor::new(ShellConfig::new());
        executor.config.env = env;

        let input = "echo 'Value: ${VAR1} and ${VAR2}'";

        let expanded = executor.expand_variables(input, &executor.config.env)?;
        assert_eq!(expanded, "Value: value1 and Value: value2");
    }

    #[test]
    fn test_default_value_expansion() {
        let env = std::collections::HashMap::new();

        let mut executor = ShellExecutor::new(executor.config.clone());

        let input = "echo 'Undefined: ${UNDEFINED_VAR:-default}'";

        let expanded = executor.expand_variables(input, &executor.config.env)?;
        assert_eq!(expanded, "echo 'Undefined: -default'");
    }

    #[test]
    fn test_env_priority() {
        let env = std::collections::HashMap::new();

        env.insert("PATH", "/usr/local/bin");
        env.insert("HOME", std::env::var("HOME").unwrap_or_default());

        let mut executor = ShellExecutor::new(ShellConfig::new());
        executor.config.env = env;

        // Higher priority: system PATH first, then workspace
        let input = "echo $PATH && which cargo";
        let expanded = executor.expand_variables(input, &executor.config.env)?;
        assert!(expanded.contains("/usr/local/bin/"));
    }
}
```

### 6.2 Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_basic_pipeline_execution() {
        let pipeline = pipeline!(
            agent_any(),
            stages!(
                stage!("Hello", steps!(sh!("echo 'Hello World'")),
                stage!("Build", steps!(sh!("cargo build"))
            )
        );

        let executor = LocalExecutor::new();
        let result = executor.execute(&pipeline);

        assert!(result.is_ok());
        match result {
            Ok(result) => {
                assert_eq!(result, StageResult::Success),
            }
            Err(e) => {
                panic!("Pipeline should not fail: {}", e);
            }
        }
    }

    #[test]
    fn test_pipeline_with_environment() {
        let pipeline = pipeline!(
            agent_any(),
            environment!(
                "MY_APP" => "my-app",
                "DEPLOY_ENV" => "staging",
                "BUILD_TYPE" => "release"
            ),
            stages!(
                stage!("Echo Env", steps!(
                    sh!("echo 'App: ${MY_APP}'")
                )
            )
        )
        );

        let executor = LocalExecutor::new();
        let result = executor.execute(&pipeline);

        assert!(result.is_ok());
        match result {
            Ok(result) => {
                assert_eq!(result, StageResult::Success),
            }
            Err(e) => {
                panic!("Pipeline should not fail: {}", e);
            }
        }
    }

    #[test]
    fn test_pipeline_with_when_condition() {
        let pipeline = pipeline!(
            agent_any(),
            stages!(
                stage!("Deploy", steps!(
                    sh!("echo 'Deploying to production'"),
                    when!(branch("main")),
                    sh!("./deploy.sh production")
                ))
            )
        );

        let executor = local::LocalExecutor::new();
        let result = executor.execute(&pipeline);

        assert!(result.is_ok());
        match result {
            Ok(result) => {
                assert_eq!(result, StageResult::Success),
            }
            Err(e) => {
                panic!("Pipeline should not fail: {}", e);
            }
        }
        }
    }

    #[test]
    fn test_variable_expansion_with_defaults() {
        let pipeline = pipeline!(
            agent_any(),
            environment!(
                "UNDEFINED_VAR:-default" => "undefined"
            ),
            stages!(
                stage!("Test", steps!(
                    sh!("echo 'Undefined: ${UNDEFINED_VAR:-default}'")
                )
            )
        )
        );

        let executor = LocalExecutor::new();
        let result = executor.execute(&pipeline);

        assert!(result.is_ok());
        match result {
            Ok(result) => {
                assert_eq!(result, StageResult::Success),
            }
            Err(e) => {
                panic!("Pipeline should not fail: {}", e);
            }
        }
    }
}
```

---

## 7. Manual de Usuario Completo

Para más información sobre cómo usar Rustline para crear pipelines CI/CD compatibles con Jenkins, ver:
- [Manual de Usuario - Rustline CI/CD DSL](docs/USER_MANUAL.md)
- [Ejemplos de Pipelines](docs/examples/)
- [Documentación Técnica](docs/rust-jenkins-dsl-study.md)
- [Comportabilidad con Jenkins](docs/jenkins-sh-compatibility.md)
