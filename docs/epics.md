# Épicas del Proyecto Rustline - DSL de Jenkins Pipeline en Rust

Este documento define las épicas del proyecto organizadas por sprints, siguiendo el enfoque TDD (Test-Driven Development) con Rust, priorizando alto rendimiento y excelencia operacional.

## Visión General

El proyecto tiene como objetivo implementar un DSL en Rust que replique la sintaxis y semántica del DSL de Jenkins Pipeline, ejecutable mediante rust-script para proporcionar capacidades de automatización de pipelines CI/CD directamente desde el ecosistema Rust.

**Principios de Desarrollo:**
- **TDD**: Red → Green → Refactor para cada funcionalidad
- **Alto Rendimiento**: Optimizaciones desde el diseño inicial
- **Excelencia Operacional**: Observabilidad, mantenibilidad, testabilidad
- **Seguridad de Tipos**: Aprovechar el sistema de tipos de Rust
- **Ergonomía**: Sintaxis familiar para desarrolladores Jenkins

---

## Épica 1: Fundamentos del DSL (Sprints 1-3)

**Objetivo**: Establecer los cimientos del DSL con estructuras de datos básicas y macros declarativas.

### Sprint 1: Estructuras de Datos Fundamentales

**Story Points**: 21
**Duración**: 2 semanas

#### US-1.1: Definición de tipos fundamentales del pipeline

**Descripción**: Implementar las estructuras de datos que representan los conceptos básicos del DSL de Jenkins Pipeline.

**Criterios de Aceptación**:
- [ ] Struct `Pipeline` con campos para agent, stages, environment, parameters, triggers, options, post
- [ ] Enum `AgentType` con variantes: Any, Label(String), Docker(DockerConfig), Kubernetes(KubernetesConfig)
- [ ] Struct `Stage` con campos: name, agent, steps, when, post
- [ ] Enum `StepType` con variantes: Shell(String), Echo(String), Retry, Timeout, Stash, Unstash, Input, Dir
- [ ] Enum `StageResult` con variantes: Success, Failure, Unstable, Skipped
- [ ] Enum `PostCondition` con variantes: Always, Success, Failure, Unstable, Changed
- [ ] Todos los tipos implementan `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`
- [ ] Validación de datos en constructores (nombres no vacíos, etc.)

**Tests TDD**:
```rust
#[cfg(test)]
mod pipeline_struct_tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = Pipeline::new();
        assert!(pipeline.stages.is_empty());
    }

    #[test]
    fn test_agent_types() {
        let any_agent = AgentType::Any;
        let label_agent = AgentType::Label("linux".to_string());
        // ...
    }
}
```

**Tareas**:
1. Crear módulo `types.rs` en `src/pipeline/`
2. Definir structs y enums con derivaciones necesarias
3. Implementar validadores en métodos `new()`
4. Escribir tests unitarios para cada tipo
5. Añadir documentación KDoc a todos los tipos públicos
6. Verificar que todos los tests pasen (`cargo test`)

#### US-1.2: Implementación de Environment y Parameters

**Descripción**: Implementar la gestión de variables de entorno y parámetros del pipeline.

**Criterios de Aceptación**:
- [ ] Struct `Environment` con HashMap<String, String>
- [ ] Struct `Parameters` con tipos: BooleanParameter, StringParameter, ChoiceParameter
- [ ] Resolución de variables de entorno con soporte para expansiones (`${VAR}`)
- [ ] Validación de nombres de parámetros (sin espacios, caracteres válidos)
- [ ] Valores por defecto para parámetros opcionales

**Tests TDD**:
```rust
#[test]
fn test_environment_variable_resolution() {
    let mut env = Environment::new();
    env.set("BUILD_NUMBER", "42");
    assert_eq!(env.resolve("${BUILD_NUMBER}"), "42");
}

#[test]
fn test_parameter_validation() {
    let param = StringParameter::new("test-param", Some("default"), false);
    assert!(param.is_valid());
}
```

#### US-1.3: Estructuras de opciones y triggers

**Descripción**: Implementar configuración de opciones de pipeline y triggers de ejecución.

**Criterios de Aceptación**:
- [ ] Struct `PipelineOptions` con campos: timeout, retry, buildDiscarder, skipDefaultCheckout
- [ ] Enum `Trigger` con variantes: Cron, PollSCM, Upstream, Manual
- [ ] Validación de timeouts (mínimo 1 segundo)
- [ ] Validación de cron expressions
- [ ] Validación de configuración de retry (count positivo)

**Tests TDD**:
```rust
#[test]
fn test_timeout_validation() {
    let options = PipelineOptions::default().with_timeout(Duration::from_secs(60));
    assert!(options.timeout().is_some());
}

#[test]
fn test_retry_count_validation() {
    assert!(PipelineOptions::default().with_retry(0).is_err());
}
```

### Sprint 2: Macros Declarativas Básicas

**Story Points**: 34
**Duración**: 2.5 semanas

#### US-1.4: Macro `pipeline!` base

**Descripción**: Implementar la macro principal para definir pipelines con sintaxis declarativa.

**Criterios de Aceptación**:
- [ ] Macro `pipeline!` que acepta agent, stages, post
- [ ] Validación en tiempo de compilación de estructura del pipeline
- [ ] Generación de código eficiente sin clonaciones innecesarias
- [ ] Mensajes de error claros para sintaxis incorrecta
- [ ] Soporte para versiones opcionales de bloques (environment, parameters)

**Tests TDD**:
```rust
#[test]
fn test_basic_pipeline_macro() {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Build", steps!(sh!("cargo build")))
        )
    );
    assert_eq!(pipeline.stages.len(), 1);
}

#[test]
#[should_panic(expected = "at least one stage required")]
fn test_pipeline_without_stages_fails() {
    let _ = pipeline!(agent_any());
}
```

**Tareas**:
1. Crear archivo `src/macros.rs`
2. Implementar macro `pipeline!` con `macro_rules!`
3. Añadir tests de compilación para casos válidos e inválidos
4. Probar expansión de macro con `cargo expand`
5. Verificar que tests de integración pasen

#### US-1.5: Macros `stage!`, `steps!`, `sh!`, `echo!`

**Descripción**: Implementar macros auxiliares para definir etapas y pasos del pipeline.

**Criterios de Aceptación**:
- [ ] Macro `stage!` que crea instancias de `Stage` con nombre y steps
- [ ] Macro `steps!` que crea vectores de steps
- [ ] Macro `sh!` para comandos de shell
- [ ] Macro `echo!` para mensajes de log
- [ ] Todas las macros validan entrada en tiempo de compilación
- [ ] Mensajes de error descriptivos

**Tests TDD**:
```rust
#[test]
fn test_stage_macro() {
    let stage = stage!("Test", steps!(sh!("cargo test")));
    assert_eq!(stage.name, "Test");
    assert_eq!(stage.steps.len(), 1);
}

#[test]
fn test_steps_macro() {
    let steps = steps!(sh!("echo 1"), sh!("echo 2"));
    assert_eq!(steps.len(), 2);
}
```

#### US-1.6: Macro `post!` con condiciones

**Descripción**: Implementar macro para definir condiciones post-ejecución.

**Criterios de Aceptación**:
- [ ] Macro `post!` que acepta always, success, failure, unstable, changed
- [ ] Validación de sintaxis en tiempo de compilación
- [ ] Generación de código optimizado sin branching redundante
- [ ] Soporte para múltiples condiciones simultáneas

**Tests TDD**:
```rust
#[test]
fn test_post_macro() {
    let post = post!(
        always(sh!("cleanup")),
        success(sh!("notify")),
        failure(sh!("alert"))
    );
    assert_eq!(post.len(), 3);
}
```

### Sprint 3: Primer Motor de Ejecución

**Story Points**: 34
**Duración**: 2.5 semanas

#### US-1.7: Trait `PipelineExecutor` y `PipelineContext`

**Descripción**: Definir la interfaz abstracta para ejecutores de pipeline y el contexto de ejecución.

**Criterios de Aceptación**:
- [ ] Trait `PipelineExecutor` con método `execute()`
- [ ] Struct `PipelineContext` con entorno de ejecución y estado compartido
- [ ] Implementación mock de `PipelineExecutor` para tests
- [ ] Thread-safety con `Arc<Mutex<PipelineContext>>`
- [ ] Soporte para logs estructurados

**Tests TDD**:
```rust
#[test]
fn test_pipeline_context_thread_safety() {
    let context = Arc::new(Mutex::new(PipelineContext::new()));
    let handle = thread::spawn(move || {
        context.lock().unwrap().set_env("TEST", "value");
    });
    handle.join().unwrap();
    assert_eq!(context.lock().unwrap().get_env("TEST"), Some("value".to_string()));
}
```

#### US-1.8: `LocalExecutor` con ejecución de pasos

**Descripción**: Implementar executor local que ejecuta comandos en el sistema operativo.

**Criterios de Aceptación**:
- [ ] Implementación de `PipelineExecutor` para `LocalExecutor`
- [ ] Ejecución de comandos shell con captura de stdout/stderr
- [ ] Propagación correcta de códigos de salida
- [ ] Timeout por comando con threads
- [ ] Manejo de errores con `thiserror`
- [ ] Streaming de salida en tiempo real

**Tests TDD**:
```rust
#[test]
fn test_shell_command_execution() {
    let executor = LocalExecutor::new();
    let result = executor.execute_command("echo 'test'");
    assert!(result.is_ok());
    assert!(result.unwrap().contains("test"));
}

#[test]
fn test_command_failure_propagation() {
    let executor = LocalExecutor::new();
    let result = executor.execute_command("exit 1");
    assert!(result.is_err());
}
```

#### US-1.9: Integración completa: Pipeline → Executor

**Descripción**: Conectar el DSL con el executor para ejecutar pipelines completos.

**Criterios de Aceptación**:
- [ ] Ejecución secuencial de stages
- [ ] Propagación de errores entre stages
- [ ] Ejecución de post-conditions según resultado
- [ ] Cálculo final de `PipelineResult` (Success/Failure)
- [ ] Tests de integración completos

**Tests TDD**:
```rust
#[test]
fn test_full_pipeline_execution() {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Stage1", steps!(sh!("echo 'stage1'"))),
            stage!("Stage2", steps!(sh!("echo 'stage2'")))
        )
    );

    let executor = LocalExecutor::new();
    let result = executor.execute(&pipeline);
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), PipelineResult::Success));
}
```

---

## Épica 2: Características Avanzadas del DSL (Sprints 4-6)

**Objetivo**: Implementar funcionalidades avanzadas que diferencian el DSL de herramientas simples.

### Sprint 4: Bloques Post y When

**Story Points**: 21
**Duración**: 2 semanas

#### US-2.1: Evaluación completa de post-conditions

**Descripción**: Implementar evaluación de todas las condiciones post-ejecución con lógica correcta.

**Criterios de Aceptación**:
- [ ] Evaluación de `always` (siempre ejecuta)
- [ ] Evaluación de `success` (solo si success)
- [ ] Evaluación de `failure` (solo si failure)
- [ ] Evaluación de `unstable` (si unstable o failure)
- [ ] Evaluación de `changed` (si resultado diferente a ejecución anterior)
- [ ] Tests de todos los casos edge

**Tests TDD**:
```rust
#[test]
fn test_post_always_executes() {
    let mut executed = false;
    let post = vec![PostCondition::Always(steps!(sh!("echo test")))];
    // Test que siempre ejecuta independientemente del resultado
    assert!(eval_post(&post, StageResult::Success, &mut executed));
    assert!(executed);
}

#[test]
fn test_post_success_only_on_success() {
    let post = vec![PostCondition::Success(steps!(sh!("echo test")))];
    // Test que solo ejecuta en success
    assert!(eval_post(&post, StageResult::Success, &mut false));
    assert!(!eval_post(&post, StageResult::Failure, &mut false));
}
```

#### US-2.2: Directiva `when` con condiciones

**Descripción**: Implementar la directiva when con condiciones de rama, expresiones y variables.

**Criterios de Aceptación**:
- [ ] Enum `WhenCondition` con variantes: Branch, Tag, Expression, Environment, AllOf, AnyOf
- [ ] Macro `when!` para condiciones de rama
- [ ] Evaluación de condiciones against context
- [ ] Short-circuit evaluation para AllOf/AnyOf
- [ ] Parsing de expresiones booleanas simples

**Tests TDD**:
```rust
#[test]
fn test_when_branch_condition() {
    let condition = WhenCondition::Branch("main".to_string());
    let mut context = PipelineContext::new();
    context.set_env("GIT_BRANCH", "main");
    assert!(condition.evaluate(&context));
}

#[test]
fn test_when_allof_shortcircuit() {
    let condition = WhenCondition::AllOf(vec![
        WhenCondition::Expression("false".to_string()),
        WhenCondition::Expression("true".to_string()),
    ]);
    assert!(!condition.evaluate(&context));
}
```

#### US-2.3: Integración when con stages

**Descripción**: Integrar la directiva when en la lógica de ejecución de stages.

**Criterios de Aceptación**:
- [ ] Evaluar condiciones before de ejecutar stage
- [ ] Skip de stages cuando condition es false
- [ ] Logging de skipped stages
- [ ] Tests de integración con múltiples stages con when

**Tests TDD**:
```rust
#[test]
fn test_stage_skipped_when_condition_false() {
    let stage = Stage::new("Test", steps!(sh!("echo test")))
        .with_when(WhenCondition::Branch("other".to_string()));

    let mut context = PipelineContext::new();
    context.set_env("GIT_BRANCH", "main");

    assert!(!should_execute_stage(&stage, &context));
}
```

### Sprint 5: Steps de Control de Flujo

**Story Points**: 34
**Duración**: 2.5 semanas

#### US-2.4: Implementación de `retry!`

**Descripción**: Implementar lógica de reintentos con exponencial backoff opcional.

**Criterios de Aceptación**:
- [ ] Macro `retry!` con count y step
- [ ] Reintentos secuenciales hasta success o max count
- [ ] Opcional: exponential backoff configurable
- [ ] Logging de reintentos fallidos
- [ ] Preservación del último error
- [ ] Performance: zero-allocation cuando no hay reintentos

**Tests TDD**:
```rust
#[test]
fn test_retry_success_on_third_attempt() {
    let mut attempt = 0;
    let step = Box::new(Step::custom(|| {
        attempt += 1;
        if attempt < 3 {
            Err(PipelineError::CommandFailed("fail".to_string()))
        } else {
            Ok(())
        }
    }));

    let retry = retry!(3, step);
    assert!(retry.execute(&context).is_ok());
    assert_eq!(attempt, 3);
}

#[test]
fn test_retry_exhausted_fails() {
    let step = Box::new(Step::always_failing());
    let retry = retry!(3, step);
    assert!(retry.execute(&context).is_err());
}
```

#### US-2.5: Implementación de `timeout!`

**Descripción**: Implementar lógica de timeout con threads y canales.

**Criterios de Aceptación**:
- [ ] Macro `timeout!` con duration y step
- [ ] Cancelación de comando cuando timeout expira
- [ ] Limpieza de recursos (threads, procesos hijos)
- [ ] Timeout configurable por stage y global
- [ ] Thread-safe con tokio o async-std

**Tests TDD**:
```rust
#[test]
fn test_timeout_cancels_long_command() {
    let step = sh!("sleep 10");
    let timeout = timeout!(1, step);

    let start = Instant::now();
    let result = timeout.execute(&context);
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(elapsed < Duration::from_secs(3)); // + margin
}

#[test]
fn test_timeout_allows_fast_command() {
    let step = sh!("echo test");
    let timeout = timeout!(5, step);
    assert!(timeout.execute(&context).is_ok());
}
```

#### US-2.6: Steps `stash!` y `unstash!`

**Descripción**: Implementar mecanismo para compartir archivos entre stages y agentes.

**Criterios de Aceptación**:
- [ ] Macro `stash!` con nombre y includes pattern
- [ ] Macro `unstash!` con nombre
- [ ] Almacenamiento temporal en directorio configurable
- [ ] Compresión opcional con gz/tar
- [ ] Validación de nombre de stash
- [ ] Tests de roundtrip stash/unstash

**Tests TDD**:
```rust
#[test]
fn test_stash_and_unstash_preserves_files() {
    // Crear archivo de prueba
    let test_file = "test.txt";
    fs::write(test_file, "content")?;

    // Stash
    let stash = stash!("my-stash", "*.txt");
    stash.execute(&context)?;
    fs::remove_file(test_file)?;

    // Unstash
    let unstash = unstash!("my-stash");
    unstash.execute(&context)?;

    // Verificar
    assert!(fs::metadata(test_file).is_ok());
    assert_eq!(fs::read_to_string(test_file)?, "content");
}
```

### Sprint 6: Ejecución Paralela

**Story Points**: 34
**Duración**: 2.5 semanas

#### US-2.7: Soporte para `parallel!`

**Descripción**: Implementar ejecución concurrente de stages independientes.

**Criterios de Aceptación**:
- [ ] Macro `parallel!` con múltiples stages
- [ ] Ejecución concurrente usando threads
- [ ] Collection de resultados de todos los branches
- [ ] Fail-fast option (fallar si alguno falla)
- [ ] Thread-safe access a PipelineContext
- [ ] Limits de concurrency configurables

**Tests TDD**:
```rust
#[test]
fn test_parallel_executes_concurrent_stages() {
    let parallel = parallel!(
        branch!("Stage1", stage!("Stage1", steps!(sh!("sleep 1")))),
        branch!("Stage2", stage!("Stage2", steps!(sh!("sleep 1")))),
        branch!("Stage3", stage!("Stage3", steps!(sh!("sleep 1"))))
    );

    let start = Instant::now();
    let results = parallel.execute(&context)?;
    let elapsed = start.elapsed();

    assert_eq!(results.len(), 3);
    assert!(elapsed < Duration::from_secs(3)); // Concurrent: ~1s, not 3s
}
```

#### US-2.8: Directiva `matrix!`

**Descripción**: Implementar ejecución en matriz para tests multi-configuración.

**Criterios de Aceptación**:
- [ ] Macro `matrix!` con axes y exclude patterns
- [ ] Generación de todas las combinaciones
- [ ] Ejecución de cada combinación como stage separado
- [ ] Filtering con include/exclude expressions
- [ ] Validación de axes (mínimo 1 axis, 1 valor por axis)

**Tests TDD**:
```rust
#[test]
fn test_matrix_generates_combinations() {
    let matrix = matrix!(
        axes!(
            rust = ["stable", "nightly"],
            os = ["linux", "macos"]
        )
    );

    let combinations = matrix.generate_combinations();
    assert_eq!(combinations.len(), 4); // 2 x 2
    assert!(combinations.contains(&vec![
        ("rust".to_string(), "stable".to_string()),
        ("os".to_string(), "linux".to_string())
    ]));
}

#[test]
fn test_matrix_with_exclude_filtering() {
    let matrix = matrix!(
        axes!(
            rust = ["stable", "nightly"],
            os = ["linux", "macos"]
        ),
        exclude!(os == "macos", rust == "nightly")
    );

    let combinations = matrix.generate_combinations();
    assert_eq!(combinations.len(), 3); // 4 - 1 excluded
}
```

#### US-2.9: Optimización de recursos en paralelo

**Descripción**: Optimizar el uso de recursos en ejecución paralela con pooling.

**Criterios de Aceptación**:
- [ ] ThreadPool configurable con tamaño máximo
- [ ] Work-stealing scheduler (opcional con rayon)
- [ ] Semáforo para limitar recursos compartidos (Docker, K8s pods)
- [ ] Telemetry: métricas de uso de recursos
- [ ] Graceful shutdown con cancelación

**Tests TDD**:
```rust
#[test]
fn test_threadpool_limits_concurrency() {
    let pool = ThreadPool::new(2);

    let parallel = parallel!(
        branch!("S1", stage!("S1", steps!(sh!("sleep 1")))),
        branch!("S2", stage!("S2", steps!(sh!("sleep 1")))),
        branch!("S3", stage!("S3", steps!(sh!("sleep 1"))))
    );

    let start = Instant::now();
    pool.execute(parallel)?;
    let elapsed = start.elapsed();

    // Con pool de 2, 3 jobs de 1s cada uno ~2s (no 1s ni 3s)
    assert!(elapsed > Duration::from_secs(1.5));
    assert!(elapsed < Duration::from_secs(2.5));
}
```

---

## Épica 3: Backends de Ejecución y Extensibilidad (Sprints 7-9)

**Objetivo**: Desarrollar backends alternativos para diferentes entornos y mejora de la extensibilidad.

### Sprint 7: Backend GitHub Actions

**Story Points**: 34
**Duración**: 2.5 semanas

#### US-3.1: Parser de DSL a GitHub Actions Workflow

**Descripción**: Traducir pipelines definidos con el DSL a workflows de GitHub Actions.

**Criterios de Aceptación**:
- [ ] Struct `GitHubActionsBackend` que implementa `PipelineExecutor`
- [ ] Mapeo de stages → jobs
- [ ] Mapeo de steps → run/script steps
- [ ] Mapeo de when → condition expressions
- [ ] Mapeo de environment → env variables
- [ ] Generación de YAML válido de GitHub Actions

**Tests TDD**:
```rust
#[test]
fn test_simple_pipeline_to_github_actions() {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Build", steps!(sh!("cargo build"))),
            stage!("Test", steps!(sh!("cargo test")))
        )
    );

    let backend = GitHubActionsBackend::new();
    let workflow = backend.translate(&pipeline)?;

    assert!(workflow.contains("name: Build"));
    assert!(workflow.contains("run: cargo build"));
    assert!(workflow.contains("run: cargo test"));
}
```

#### US-3.2: Mapeo avanzado a GitHub Actions

**Descripción**: Implementar mapeo de características avanzadas del DSL a GitHub Actions.

**Criterios de Aceptación**:
- [ ] Mapeo de `timeout!` → `timeout-minutes`
- [ ] Mapeo de `retry!` → `continue-on-error` + manual retry
- [ ] Mapeo de `parallel!` → matrix strategy
- [ ] Mapeo de `when!` → `if` condition
- [ ] Mapeo de `post!` → always/continue-on-error

**Tests TDD**:
```rust
#[test]
fn test_parallel_to_matrix_strategy() {
    let pipeline = pipeline!(
        agent_any(),
        parallel!(
            branch!("Linux", stage!("Linux", steps!(sh!("cargo test")))),
            branch!("Mac", stage!("Mac", steps!(sh!("cargo test")))),
            branch!("Windows", stage!("Windows", steps!(sh!("cargo test"))))
        )
    );

    let backend = GitHubActionsBackend::new();
    let workflow = backend.translate(&pipeline)?;

    assert!(workflow.contains("strategy:"));
    assert!(workflow.contains("matrix:"));
}
```

#### US-3.3: Validación de workflows generados

**Descripción**: Validar que los YAMLs generados sean válidos y completos.

**Criterios de Aceptación**:
- [ ] Validación de YAML syntax
- [ ] Validación de que todos los steps tienen `run` o uses
- [ ] Validación de que jobs no tienen ciclos de dependencia
- [ ] Chequeo de límites de GitHub Actions (max jobs, max steps)
- [ ] Tests de casos edge (pipeline vacío, stages sin steps, etc.)

**Tests TDD**:
```rust
#[test]
fn test_workflow_yaml_is_valid() {
    let workflow = generate_workflow_yaml(&pipeline)?;
    let parsed: serde_yaml::Value = serde_yaml::from_str(&workflow)?;
    assert!(parsed.is_mapping());
}

#[test]
#[should_panic(expected = "cycle detected")]
fn test_detects_job_dependency_cycle() {
    let pipeline = create_cyclic_pipeline(); // S1 → S2 → S1
    let backend = GitHubActionsBackend::new();
    let _ = backend.translate(&pipeline)?;
}
```

### Sprint 8: Backend GitLab CI y Docker

**Story Points**: 34
**Duración**: 2.5 semanas

#### US-3.4: Backend GitLab CI

**Descripción**: Implementar backend para traducir DSL a `.gitlab-ci.yml`.

**Criterios de Aceptación**:
- [ ] Struct `GitLabCIBackend` que implementa `PipelineExecutor`
- [ ] Mapeo de stages → GitLab stages
- [ ] Mapeo de steps → script commands
- [ ] Mapeo de environment → variables
- [ ] Mapeo de parallel → parallel:jobs
- [ ] Mapeo de when → rules

**Tests TDD**:
```rust
#[test]
fn test_pipeline_to_gitlab_ci() {
    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("build", steps!(sh!("cargo build"))),
            stage!("test", steps!(sh!("cargo test")))
        )
    );

    let backend = GitLabCIBackackend::new();
    let gitlab_ci = backend.translate(&pipeline)?;

    assert!(gitlab_ci.contains("stages:"));
    assert!(gitlab_ci.contains("- build"));
    assert!(gitlab_ci.contains("- test"));
    assert!(gitlab_ci.contains("script:"));
}
```

#### US-3.5: Docker Executor

**Descripción**: Implementar executor que ejecuta stages dentro de contenedores Docker.

**Criterios de Aceptación**:
- [ ] Struct `DockerExecutor` que implementa `PipelineExecutor`
- [ ] Soporte para `agent docker! { image: "rust:latest" }`
- [ ] Creación de contenedores por stage
- [ ] Montaje de volúmenes para workspace
- [ ] Limpieza de contenedores después de ejecución
- [ ] Soporte para build args y environment variables

**Tests TDD**:
```rust
#[test]
fn test_docker_executor_runs_in_container() {
    let agent = AgentType::Docker(DockerConfig {
        image: "alpine:latest".to_string(),
        ..Default::default()
    });

    let stage = stage!("Test", steps!(sh!("echo 'in container'")));

    let executor = DockerExecutor::new();
    let result = executor.execute_stage(&stage, &context)?;

    assert!(matches!(result, StageResult::Success));
}

#[test]
fn test_docker_cleanup() {
    let executor = DockerExecutor::new();
    executor.execute(&pipeline)?;

    // Verificar que no quedan contenedores zombies
    let containers = list_docker_containers()?;
    assert!(!containers.iter().any(|c| c.name.contains("rustline")));
}
```

#### US-3.6: Kubernetes Executor

**Descripción**: Implementar executor para ejecutar en clusters Kubernetes.

**Criterios de Aceptación**:
- [ ] Struct `KubernetesExecutor` que implementa `PipelineExecutor`
- [ ] Soporte para `agent kubernetes! { pod: PodSpec }`
- [ ] Creación de Pods por stage
- [ ] ConfigMaps y Secrets para environment
- [ ] Logs streaming desde pods
- [ ] Cleanup de pods y recursos

**Tests TDD**:
```rust
#[test]
fn test_kubernetes_executor_creates_pod() {
    let pod_spec = PodSpec {
        image: "rust:latest".to_string(),
        resources: ResourceRequests {
            cpu: "1".to_string(),
            memory: "1Gi".to_string(),
        },
    };

    let executor = KubernetesExecutor::new().with_namespace("test");
    executor.execute(&pipeline)?;

    // Verificar que el pod fue creado y limpiado
    let pods = list_pods("test")?;
    assert!(!pods.iter().any(|p| p.name.contains("rustline")));
}
```

### Sprint 9: Sistema de Plugins y Extensibilidad

**Story Points**: 21
**Duración**: 2 semanas

#### US-3.7: Sistema de plugins custom steps

**Descripción**: Permitir a usuarios definir steps personalizados reutilizables.

**Criterios de Aceptación**:
- [ ] Trait `CustomStep` con método `execute()`
- [ ] Registro de steps custom en PipelineContext
- [ ] Invocación desde DSL con `custom_step!("name", args...)`
- [ ] Documentación de API para crear plugins
- [ ] Examples de plugins útiles (slack, email, etc.)

**Tests TDD**:
```rust
struct SlackNotification {
    webhook_url: String,
}

impl CustomStep for SlackNotification {
    fn execute(&self, _context: &PipelineContext) -> Result<(), PipelineError> {
        // Envía notificación a Slack
        Ok(())
    }
}

#[test]
fn test_custom_step_registration() {
    let mut context = PipelineContext::new();
    context.register_custom_step("slack", Box::new(SlackNotification::new("url")));

    let step = custom_step!("slack", message: "Pipeline failed");
    assert!(step.execute(&context).is_ok());
}
```

#### US-3.8: Shared Libraries (funcionalidad Jenkins)

**Descripción**: Implementar mecanismo para compartir código y steps entre pipelines.

**Criterios de Aceptación**:
- [ ] Struct `SharedLibrary` con collection de steps y functions
- [ ] Carga de libraries desde crates o repositorios Git
- [ ] Compartición de lógica común (deploy, notificaciones, etc.)
- [ ] Versionado de libraries
- [ ] Tests de integración con multiple libraries

**Tests TDD**:
```rust
#[test]
fn test_shared_library_step() {
    let lib = SharedLibrary::from_crates("rustline-shared-lib");

    let pipeline = pipeline!(
        agent_any(),
        stages!(
            stage!("Deploy", steps!(
                lib.step!("production_deploy", env: "prod")
            ))
        )
    );

    let result = executor.execute(&pipeline)?;
    assert!(matches!(result, PipelineResult::Success));
}
```

#### US-3.9: Optimización y caching del LocalExecutor

**Descripción**: Mejorar el rendimiento del executor local con caching inteligente.

**Criterios de Aceptación**:
- [ ] Caching de dependencias Cargo en `$CARGO_HOME/cache`
- [ ] Caching de compilaciones incrementales en `target/`
- [ ] Soporte para cache keys basadas en hash de `Cargo.lock`
- [ ] Restore de cache antes de stage
- [ ] Save de cache después de stage
- [ ] Métricas de cache hit/miss rate

**Tests TDD**:
```rust
#[test]
fn test_cargo_cache_speeds_up_build() {
    let executor = LocalExecutor::new().with_cache(CacheConfig::default());

    // Primer build: cold cache
    let start = Instant::now();
    executor.execute(&pipeline)?;
    let first_build = start.elapsed();

    // Segundo build: warm cache
    let start = Instant::now();
    executor.execute(&pipeline)?;
    let second_build = start.elapsed();

    assert!(second_build < first_build / 2); // Al menos 2x más rápido
}

#[test]
fn test_cache_key_based_on_cargo_lock() {
    let executor = LocalExecutor::new();

    // Modificar Cargo.lock debe invalidar cache
    modify_cargo_lock()?;
    let cache_key = executor.generate_cache_key();

    assert!(!executor.cache_exists(&cache_key));
}
```

---

## Épica 4: Ecosistema, Documentación y Excelencia Operativa (Sprints 10-12)

**Objetivo**: Establecer el ecosistema alrededor del DSL con documentación, herramientas y excelencia operativa.

### Sprint 10: Observabilidad y Telemetría

**Story Points**: 21
**Duración**: 2 semanas

#### US-4.1: Logs estructurados con tracing

**Descripción**: Implementar logging estructurado para debugging y observabilidad.

**Criterios de Aceptación**:
- [ ] Integración con crate `tracing`
- [ ] Spans por cada stage y step
- [ ] Eventos para inicio/finalización de ejecución
- [ ] Campos estructurados (stage_name, step_type, duration, result)
- [ ] Filtros configurables de log level
- [ ] Export a JSON para integración con ELK/Datadog

**Tests TDD**:
```rust
#[test]
fn test_tracing_spans_for_stages() {
    let subscriber = tracing_subscriber::fmt()
        .with_test_writer()
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        executor.execute(&pipeline).unwrap();
    });

    // Verificar logs generados
    let logs = capture_test_logs();
    assert!(logs.contains("stage=Build"));
    assert!(logs.contains("step_type=Shell"));
}
```

#### US-4.2: Métricas de ejecución

**Descripción**: Recolectar métricas detalladas de ejecución de pipelines.

**Criterios de Aceptación**:
- [ ] Métricas: duración total, duración por stage, duración por step
- [ ] Métricas: cache hit/miss rate, memory usage, CPU usage
- [ ] Integración con Prometheus (exportador HTTP)
- [ ] Etiquetas por pipeline, stage, result
- [ ] Histogramas para percentiles (p50, p95, p99)
- [ ] Tests de integración con Prometheus

**Tests TDD**:
```rust
#[test]
fn test_metrics_collection() {
    let metrics = MetricsCollector::new();
    executor.execute(&pipeline).unwrap();

    assert!(metrics.get_metric("pipeline_duration_total").is_some());
    assert!(metrics.get_metric("stage_duration_seconds{stage=\"Build\"}").is_some());
}

#[test]
fn test_prometheus_export() {
    let exporter = PrometheusExporter::bind("127.0.0.1:9090").unwrap();
    executor.execute(&pipeline).unwrap();

    let metrics = fetch_prometheus_metrics("http://127.0.0.1:9090/metrics");
    assert!(metrics.contains("pipeline_duration_seconds"));
}
```

#### US-4.3: Health checks y diagnóstico

**Descripción**: Implementar checks de salud y herramientas de diagnóstico.

**Criterios de Aceptación**:
- [ ] Health check endpoint (para integración con Kubernetes)
- [ ] Diagnóstico de dependencias (rustup, cargo, docker, kubectl)
- [ ] Chequeo de configuración de executor
- [ ] Validación de sintaxis de pipeline sin ejecutar
- [ ] Dry-run mode para testing
- [ ] Tests de health checks

**Tests TDD**:
```rust
#[test]
fn test_health_check() {
    let health = check_health();
    assert!(health.rust_available);
    assert!(health.docker_available);
}

#[test]
fn test_pipeline_syntax_validation() {
    let pipeline = pipeline!(agent_any());
    let errors = validate_syntax(&pipeline);

    // Pipeline sin stages debe fallar
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("at least one stage")));
}

#[test]
fn test_dry_run_mode() {
    let executor = LocalExecutor::new().with_dry_run(true);
    let result = executor.execute(&pipeline).unwrap();

    assert_eq!(result.execution_mode, ExecutionMode::DryRun);
    assert_eq!(result.commands_executed, 0);
}
```

### Sprint 11: Herramientas de Desarrollo

**Story Points**: 34
**Duración**: 2.5 semanas

#### US-4.4: CLI con rust-script integration

**Descripción**: Implementar CLI para ejecutar pipelines desde línea de comandos.

**Criterios de Aceptación**:
- [ ] Subcomando `run` para ejecutar pipelines
- [ ] Subcomando `validate` para validación de sintaxis
- [ ] Subcomando `dry-run` para testing
- [ ] Flags para log level, output format (human, json, yaml)
- [ ] Integración con rust-script para ejecución directa
- [ ] Tests de CLI con assert_cmd

**Tests TDD**:
```rust
#[test]
fn test_cli_run_command() {
    let mut cmd = Command::cargo_bin("rustline").unwrap();
    cmd.arg("run")
       .arg("examples/basic.rs");

    cmd.assert()
       .success()
       .stdout(predicates::str::contains("Pipeline completed successfully"));
}

#[test]
fn test_cli_validate_command() {
    let mut cmd = Command::cargo_bin("rustline").unwrap();
    cmd.arg("validate")
       .arg("examples/invalid.rs");

    cmd.assert()
       .failure()
       .stderr(predicates::str::contains("invalid pipeline"));
}
```

#### US-4.5: Extensiones de IDE (VS Code)

**Descripción**: Crear extensión de VS Code para syntax highlighting y autocomplete.

**Criterios de Aceptación**:
- [ ] Syntax highlighting para macros del DSL
- [ ] Autocomplete para nombres de macros
- [ ] Snippets para patrones comunes (pipeline, stage, steps)
- [ ] Diagnósticos en tiempo real (usando RLS/rust-analyzer)
- [ ] Go to definition para custom steps
- [ ] Publicación en marketplace

**Tests TDD**:
```typescript
// Tests de extensión (Mocha/TypeScript)
describe("Syntax Highlighting", () => {
    it("should highlight pipeline macro", () => {
        const tokens = tokenize("pipeline!(agent_any())");
        assert(tokens.some(t => t.type === "keyword" && t.value === "pipeline!"));
    });
});

describe("Autocomplete", () => {
    it("should suggest macros", () => {
        const suggestions = provideCompletionItems("pipeline!(agen");
        assert(suggestions.includes("agent_any()"));
    });
});
```

#### US-4.6: Format y linting del DSL

**Descripción**: Implementar herramienta para formatear y lintear pipelines.

**Criterios de Aceptación**:
- [ ] Subcomando `fmt` para formatear pipelines
- [ ] Consistencia en indentación (4 spaces)
- [ ] Consistencia en orden de bloques (agent → stages → post)
- [ ] Linter con reglas configurables
- [ ] Integración con pre-commit hooks
- [ ] Tests de formateo

**Tests TDD**:
```rust
#[test]
fn test_formatter_normalizes_indentation() {
    let input = "pipeline!(agent_any(),stages!(stage!(\"Test\",steps!(sh!(\"echo\")))))";
    let formatted = format_pipeline(input);

    assert_eq!(
        formatted,
        r#"pipeline!(
    agent_any(),
    stages!(
        stage!("Test", steps!(sh!("echo")))
    )
)"#
    );
}

#[test]
fn test_linter_detects_issues() {
    let input = r#"
    pipeline!(
        stages!(
            stage!("Test", steps!(sh!("echo"))),
            stage!("Build", steps!(sh!("cargo build")))
        ),
        agent_any()
    )
    "#;

    let issues = lint_pipeline(input);
    assert!(issues.iter().any(|i| i.msg.contains("agent should come before stages")));
}
```

### Sprint 12: Documentación y Lanzamiento

**Story Points**: 34
**Duración**: 2.5 semanas

#### US-4.7: Guía de inicio rápido (Quick Start)

**Descripción**: Crear guía comprehensiva para nuevos usuarios.

**Criterios de Aceptación**:
- [ ] Instalación desde crates.io
- [ ] Primer pipeline con 3 stages
- [ ] Ejecución con rust-script
- [ ] Ejemplos de características principales
- [ ] Troubleshooting common issues
- [ ] Traducción a español
- [ ] Tests de documentación (doc tests)

**Tests TDD**:
```rust
// Doc tests en el código
/// # Examples
///
/// ```
/// use rustline::prelude::*;
///
/// let pipeline = pipeline!(
///     agent_any(),
///     stages!(
///         stage!("Build", steps!(sh!("cargo build")))
///     )
/// );
///
/// let executor = LocalExecutor::new();
/// executor.execute(&pipeline).unwrap();
/// ```
```

#### US-4.8: Referencia completa de API

**Descripción**: Documentar toda la API pública con ejemplos.

**Criterios de Aceptación**:
- [ ] Documentación de todas las macros públicas
- [ ] Documentación de todos los structs y traits públicos
- [ ] Ejemplos de uso para cada macro
- [ ] Diagramas de arquitectura
- [ ] Glosario de términos
- [ ] Links a código fuente
- [ ] Tests de documentación

**Tareas**:
1. Añadir `///` docs a todos los items públicos
2. Incluir ejemplos en cada macro
3. Verificar que `cargo doc --no-deps` genera sin warnings
4. Crear `docs/api-reference.md`
5. Generar diagramas con Mermaid

#### US-4.9: Templates y ejemplos reales

**Descripción**: Crear plantillas de pipelines para casos comunes.

**Criterios de Aceptación**:
- [ ] Template para proyecto Rust simple (build, test, clippy, fmt)
- [ ] Template para proyecto Rust con workspace
- [ ] Template para web service con Docker
- [ ] Template para library con múltiples targets
- [ ] Ejemplo de pipeline con Docker multi-stage
- [ ] Ejemplo de pipeline con matrix testing
- [ ] Tests que ejecutan todos los ejemplos

**Tests TDD**:
```rust
#[test]
fn test_simple_rust_template_runs() {
    let pipeline = load_template("simple-rust");
    let executor = LocalExecutor::new();
    assert!(executor.execute(&pipeline).is_ok());
}

#[test]
fn test_workspace_template_runs() {
    let pipeline = load_template("workspace");
    let executor = LocalExecutor::new();
    assert!(executor.execute(&pipeline).is_ok());
}
```

#### US-4.10: Lanzamiento v0.1.0

**Descripción**: Preparar y publicar la versión inicial.

**Criterios de Aceptación**:
- [ ] Todos los tests pasan con 100% success rate
- [ ] `cargo clippy -- -D warnings` sin errores
- [ ] `cargo fmt --check` sin cambios
- [ ] `cargo doc --no-deps` sin warnings
- [ ] CHANGELOG.md actualizado
- [ ] Tag v0.1.0 en Git
- [ ] Publicación en crates.io
- [ ] Anuncio en Reddit r/rust, Twitter, Discord

**Checklist de calidad**:
- [ ] Coverage de tests > 80%
- [ ] Benchmarks para performance (criterion)
- [ ] Seguridad: `cargo audit` sin issues
- [ ] Dependencias: todas actualizadas
- [ ] Licencia: Apache-2.0 o MIT
- [ ] README.md completo con badges

---

## Métricas de Éxito del Proyecto

### Métricas de Calidad
- **Test Coverage**: > 80% (medido con tarpaulin)
- **Clippy**: Zero warnings (`cargo clippy -- -D warnings`)
- **Documentation**: 100% de API pública documentada
- **Performance**: Pipeline simple < 1s de startup overhead

### Métricas de Usuario
- **Instalación exitosa**: < 5 minutos
- **Primer pipeline ejecutable**: < 15 minutos
- **Curva de aprendizaje familiaridad Jenkins**: < 1 hora

### Métricas Operativas
- **Uptime de servicio de CLI**: > 99.9%
- **Tiempo de ejecución pipeline**: < 1.1x vs shell scripts nativos
- **Memory footprint**: < 50MB para pipelines medianos

---

## Convenciones de Commit y Branches

### Commits
Formato: `type(scope): description`

Types:
- `feat`: Nueva funcionalidad
- `fix`: Bug fix
- `refactor`: Refactorización sin cambio funcional
- `test`: Agregar o modificar tests
- `docs`: Documentación
- `perf`: Mejora de performance
- `chore`: Mantenimiento

### Branches
- `main`: Rama de desarrollo principal
- `sprint/N`: Rama de sprint actual (ej: `sprint/1`)
- `feature/US-ID`: Rama de user story (ej: `feature/US-1.1`)
- `bugfix/DESC`: Rama de bug fix (ej: `bugfix/memory-leak`)
- `release/vX.Y.Z`: Rama de release

### Pull Requests
- Requieren review de al menos 1 maintainer
- Todos los tests deben pasar
- CI debe ser verde
- Cambios de breaking API requieren discusión en issue

---

## Referencias

- [Estudio Técnico Completo](./rust-jenkins-dsl-study.md)
- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [TDD en Rust](https://blog.yoshuawuyts.com/tdd-in-rust/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
