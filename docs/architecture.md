# Arquitectura y Alto Rendimiento

Este documento describe la arquitectura del proyecto Rustline, enfocándose en principios de alto rendimiento, excelencia operativa y mantenibilidad a largo plazo.

## Principios Arquitectónicos

### 1. Hexagonal Architecture (Ports and Adapters)

**Objetivo**: Desacoplar el dominio del pipeline de los detalles de implementación de executors.

```
┌─────────────────────────────────────────────────────────┐
│                     Application Layer                     │
│  ┌────────────────────────────────────────────────────┐ │
│  │              Pipeline Domain Logic                  │ │
│  │  - Pipeline, Stage, Step definitions                │ │
│  │  - DSL macros and builders                         │ │
│  │  - Validation and business rules                    │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                         │
                         │ Ports (Traits)
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│ Local        │  │ Docker       │  │ Kubernetes   │
│ Executor     │  │ Executor     │  │ Executor     │
│ Adapter      │  │ Adapter      │  │ Adapter      │
└──────────────┘  └──────────────┘  └──────────────┘
        │                │                │
        └────────────────┼────────────────┘
                         ▼
                ┌──────────────┐
                │ CI/CD Backends│
                │ (GitHub, Git │
                │  Lab)        │
                └──────────────┘
```

**Beneficios**:
- Testability: Domain logic sin dependencias externas
- Flexibilidad: Agregar nuevos executors sin cambiar el dominio
- Maintainability: Cambios en adapters no afectan el core

**Implementación**:

```rust
// Port: Trait en el dominio
pub trait PipelineExecutor: Send + Sync {
    fn execute(&self, pipeline: &Pipeline) -> Result<PipelineResult, PipelineError>;

    fn capabilities(&self) -> ExecutorCapabilities;
    fn health_check(&self) -> HealthStatus;
}

// Adapter: Implementación concreta
pub struct LocalExecutor {
    cwd: PathBuf,
    env: HashMap<String, String>,
    config: ExecutorConfig,
}

impl PipelineExecutor for LocalExecutor {
    fn execute(&self, pipeline: &Pipeline) -> Result<PipelineResult, PipelineError> {
        // Implementación específica para ejecución local
    }
}
```

### 2. Zero-Copy y Ownership

**Objetivo**: Minimizar copias de datos para maximizar rendimiento.

**Principios**:
- Usar `&str` en lugar de `String` cuando sea posible
- Usar `Cow<str>` para strings que pueden ser prestadas o owned
- Evitar clonaciones en el hot path de ejecución
- Aprovechar el ownership de Rust para transferir datos sin copias

```rust
// ❌ Mal: Copias innecesarias
pub struct Stage {
    pub name: String,           // Copiada al pasar
    pub steps: Vec<Step>,        // Copiada al pasar
}

// ✅ Bien: Usa referencias cuando es seguro
pub struct Stage<'a> {
    pub name: Cow<'a, str>,      // Puede ser &str o String
    pub steps: Vec<Step>,        // Vec ya usa heap
}

// ✅ Mejor: Ownership transferido sin copias
pub struct Pipeline {
    pub stages: Vec<Stage>,
}

impl Pipeline {
    pub fn into_stages(self) -> Vec<Stage> {
        // Transferencia de ownership, zero-copy
        self.stages
    }
}
```

**Benchmark: Zero-copy vs Cloning**
```rust
#[bench]
fn bench_string_clone(b: &mut Bencher) {
    let s = "a".repeat(1000);
    b.iter(|| {
        let _ = s.clone(); // Copia 1000 bytes
    });
}

#[bench]
fn bench_string_borrow(b: &mut Bencher) {
    let s = "a".repeat(1000);
    b.iter(|| {
        let _: &str = &s; // Solo puntero (8 bytes)
    });
}
```

### 3. Lazy Evaluation y Compute-on-Demand

**Objetivo**: Postponer computaciones hasta que sea necesario.

**Aplicaciones**:
- Resolución de variables de entorno
- Expansión de expresiones en condiciones `when`
- Parsing de configuración

```rust
// ❌ Mal: Evaluación eager
pub struct Environment {
    vars: HashMap<String, String>,
}

impl Environment {
    pub fn resolve(&self, expr: &str) -> String {
        // Evalúa inmediatamente aunque no se use
        self.expand_variables(expr)
    }
}

// ✅ Bien: Lazy evaluation
pub struct Environment {
    vars: HashMap<String, String>,
}

impl Environment {
    pub fn resolve_lazy(&self, expr: &str) -> LazyResolution {
        LazyResolution {
            env: self,
            expr: expr.to_string(),
        }
    }
}

pub struct LazyResolution<'a> {
    env: &'a Environment,
    expr: String,
}

impl LazyResolution<'_> {
    pub fn eval(&self) -> String {
        // Solo evalúa cuando se llama a eval()
        self.env.expand_variables(&self.expr)
    }
}

// Uso
let resolved = env.resolve_lazy("${BUILD_NUMBER}"); // No evalúa aún
// ... más tarde ...
let value = resolved.eval(); // Evalúa solo cuando se necesita
```

### 4. Composability y Small Primitives

**Objetivo**: Construir funcionalidades complejas a partir de primitivas simples.

**Ejemplo**: Composición de steps

```rust
// Primitivas simples
pub fn sh(cmd: &str) -> Step;
pub fn retry(count: usize, step: Step) -> Step;
pub fn timeout(secs: u64, step: Step) -> Step;

// Composición para funcionalidad compleja
let step = retry!(
    3,
    timeout!(
        30,
        sh!("curl https://api.example.com/data")
    )
);

// Es equivalente a:
// 1. Intentar el comando hasta 3 veces
// 2. Cada intento tiene timeout de 30 segundos
// 3. Retorna success si algún intento tiene éxito
```

### 5. Type-Level Guarantees

**Objetivo**: Aprovechar el sistema de tipos para prevenir errores en tiempo de compilación.

**Newtype Pattern**:

```rust
// Usa newtypes para evitar confusión
#[derive(Debug, Clone, PartialEq)]
pub struct StageName(String);

impl StageName {
    pub fn new(name: impl Into<String>) -> Result<Self, ValidationError> {
        let name = name.into();
        if name.is_empty() {
            return Err(ValidationError::EmptyName);
        }
        if name.len() > 100 {
            return Err(ValidationError::NameTooLong);
        }
        Ok(StageName(name))
    }
}

#[derive(Debug, Clone)]
pub struct Stage {
    pub name: StageName,  // Type-safe: no se puede confundir con otros strings
}

// Compile-time error: cannot pass String where StageName is expected
let stage = Stage {
    name: "Build".to_string(),  // ❌ Error
};

let stage = Stage {
    name: StageName::new("Build").unwrap(),  // ✅ Correcto
};
```

**State Machine con Types**:

```rust
// Estados del pipeline
pub struct PipelineNotBuilt;
pub struct PipelineBuilt;
pub struct PipelineRunning;
pub struct PipelineCompleted;

pub struct Pipeline<S> {
    _state: PhantomData<S>,
    // ... campos del pipeline
}

impl Pipeline<PipelineNotBuilt> {
    pub fn stage(mut self, stage: Stage) -> PipelineBuilder<Self> {
        PipelineBuilder::new(stage, self)
    }
}

// Solo se puede ejecutar después de build
impl Pipeline<PipelineBuilt> {
    pub fn execute(self, executor: &dyn PipelineExecutor) -> Result<PipelineResult, PipelineError> {
        // ...
    }
}

// Compile-time error: cannot execute not built pipeline
let pipeline = Pipeline::new();
let _ = pipeline.execute(&executor);  // ❌ Error: Pipeline doesn't have execute method
```

---

## Capas de Arquitectura

### 1. Domain Layer (Core)

**Ubicación**: `src/pipeline/`

**Responsabilidades**:
- Definición de tipos del dominio (Pipeline, Stage, Step, Agent, etc.)
- Lógica de negocio y validación
- Macros del DSL
- Invariantes del dominio

**Estructura**:
```
src/pipeline/
├── mod.rs              // Re-exporta tipos públicos
├── types.rs            // Structs y enums del dominio
├── agent.rs            // Tipos de agentes
├── stages.rs           // Tipos de stages
├── steps.rs            // Tipos de steps
├── environment.rs      // Variables de entorno
├── parameters.rs       // Parámetros del pipeline
├── triggers.rs         // Triggers de ejecución
├── options.rs          // Opciones del pipeline
└── validation.rs       // Lógica de validación
```

**Reglas**:
- Sin dependencias externas (no network, filesystem)
- 100% testable sin mocks
- Pureza: funciones deterministas

### 2. Application Layer

**Ubicación**: `src/executor/`

**Responsabilidades**:
- Coordinación de la ejecución de pipelines
- Implementación de traits de ports
- Orquestación de stages y steps
- Manejo de errores y recovery

**Estructura**:
```
src/executor/
├── mod.rs              // Re-exports
├── trait.rs            // PipelineExecutor trait
├── local.rs            // LocalExecutor
├── docker.rs           // DockerExecutor
├── kubernetes.rs       // KubernetesExecutor
└── context.rs          // PipelineContext
```

**Reglas**:
- Implementa traits del domain
- Puede tener dependencias externas (filesystem, network)
- Manejo robusto de errores con `thiserror`

### 3. Infrastructure Layer

**Ubicación**: `src/infrastructure/`

**Responsabilidades**:
- Backends CI/CD (GitHub Actions, GitLab CI)
- Integración con herramientas externas
- Configuración y setup
- Logging y telemetría

**Estructura**:
```
src/infrastructure/
├── mod.rs
├── github_actions.rs  // GitHubActionsBackend
├── gitlab_ci.rs       // GitLabCIBackend
├── logging.rs         // Configuración de tracing
├── metrics.rs         // Exportador de métricas
└── config.rs          // Carga de configuración
```

**Reglas**:
- Implementa adapters externos
- Maneja protocolos específicos (YAML, HTTP, etc.)
- Abstracta detalles técnicos de dominio

### 4. Interface Layer (DSL Macros)

**Ubicación**: `src/macros.rs`

**Responsabilidades**:
- Definición de macros declarativas
- Validación en tiempo de compilación
- Generación de código ergonómico

**Estructura**:
```
src/macros.rs          // Todas las macros
src/lib.rs             // Re-exports para crate root
```

**Reglas**:
- Validación exhaustiva de inputs
- Mensajes de error claros
- Zero-allocation cuando es posible

---

## Patrones de Diseño Clave

### 1. Builder Pattern

**Uso**: Construcción de objetos complejos con múltiples configuraciones opcionales.

```rust
pub struct PipelineBuilder {
    agent: Option<AgentType>,
    stages: Vec<Stage>,
    environment: Environment,
    parameters: Parameters,
    options: PipelineOptions,
    post: Vec<PostCondition>,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            agent: None,
            stages: Vec::new(),
            environment: Environment::new(),
            parameters: Parameters::new(),
            options: PipelineOptions::default(),
            post: Vec::new(),
        }
    }

    pub fn agent(mut self, agent: AgentType) -> Self {
        self.agent = Some(agent);
        self
    }

    pub fn stage(mut self, stage: Stage) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn environment<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Environment),
    {
        f(&mut self.environment);
        self
    }

    pub fn build(self) -> Result<Pipeline, BuildError> {
        Ok(Pipeline {
            agent: self.agent.ok_or(BuildError::MissingAgent)?,
            stages: self.stages,
            environment: self.environment,
            parameters: self.parameters,
            options: self.options,
            post: self.post,
        })
    }
}

// Uso fluido
let pipeline = PipelineBuilder::new()
    .agent(AgentType::Any)
    .stage(stage!("Build", steps!(sh!("cargo build"))))
    .stage(stage!("Test", steps!(sh!("cargo test"))))
    .environment(|env| {
        env.set("CARGO_INCREMENTAL", "0");
    })
    .build()?;
```

### 2. Trait Objects vs Generics

**Decisiones**:

| Situación | Enfoque | Justificación |
|-----------|---------|---------------|
| Ejecutores dinámicos (configurados en runtime) | Trait Objects | Flexibilidad, no se conoce tipo en compile-time |
| Pasos internos (composición de steps) | Generics | Zero-cost, monomorfización |
| Plugins externos | Trait Objects | Carga dinámica de crates |
| Validation logic | Generics | Compile-time type checking |

**Ejemplo: Generics para Steps**

```rust
// Generic: Compile-time dispatch, zero-cost
pub struct Step<S: StepImpl> {
    inner: S,
}

pub trait StepImpl: Send + Sync {
    fn execute(&self, context: &PipelineContext) -> Result<(), PipelineError>;
}

pub struct ShellStep {
    command: String,
}

impl StepImpl for ShellStep {
    fn execute(&self, context: &PipelineContext) -> Result<(), PipelineError> {
        // Implementación
    }
}

// En compile-time, esto se especializa para cada tipo de step
fn execute_steps<S: StepImpl>(steps: &[Step<S>], context: &PipelineContext) {
    for step in steps {
        step.inner.execute(context);
    }
}
```

**Ejemplo: Trait Objects para Executors**

```rust
// Dynamic: Runtime dispatch, flexibilidad
pub trait PipelineExecutor: Send + Sync {
    fn execute(&self, pipeline: &Pipeline) -> Result<PipelineResult, PipelineError>;
}

// Se puede elegir en runtime qué executor usar
fn run_pipeline(pipeline: &Pipeline, executor: &dyn PipelineExecutor) {
    executor.execute(pipeline);
}

fn main() {
    let pipeline = create_pipeline();

    // Selección dinámica basada en configuración
    let executor: Box<dyn PipelineExecutor> = match config.backend {
        Backend::Local => Box::new(LocalExecutor::new()),
        Backend::Docker => Box::new(DockerExecutor::new()),
        Backend::Kubernetes => Box::new(KubernetesExecutor::new()),
    };

    executor.execute(&pipeline)?;
}
```

### 3. Strategy Pattern

**Uso**: Algoritmos intercambiables para diferentes contextos.

```rust
pub trait CacheStrategy: Send + Sync {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn set(&self, key: &str, value: &[u8]);
    fn invalidate(&self, key: &str);
}

// Estrategia: FileSystem cache
pub struct FileSystemCache {
    dir: PathBuf,
}

impl CacheStrategy for FileSystemCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.dir.join(key);
        fs::read(path).ok()
    }

    fn set(&self, key: &str, value: &[u8]) {
        let path = self.dir.join(key);
        fs::write(path, value).ok();
    }

    fn invalidate(&self, key: &str) {
        let path = self.dir.join(key);
        fs::remove_file(path).ok();
    }
}

// Estrategia: Memory cache
pub struct MemoryCache {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl CacheStrategy for MemoryCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.data.read().unwrap().get(key).cloned()
    }

    fn set(&self, key: &str, value: &[u8]) {
        self.data.write().unwrap().insert(key.to_string(), value.to_vec());
    }

    fn invalidate(&self, key: &str) {
        self.data.write().unwrap().remove(key);
    }
}

// Uso: Inyección de estrategia
pub struct Executor<C: CacheStrategy> {
    cache: C,
}

impl<C: CacheStrategy> Executor<C> {
    pub fn new(cache: C) -> Self {
        Self { cache }
    }
}

fn main() {
    // Se puede cambiar la estrategia sin modificar el executor
    let executor = Executor::new(FileSystemCache::new("/tmp/cache"));
    // o
    let executor = Executor::new(MemoryCache::new());
}
```

### 4. Observer Pattern (Event Emitter)

**Uso**: Notificar eventos de ejecución a múltiples listeners.

```rust
pub trait PipelineEventListener: Send + Sync {
    fn on_pipeline_start(&self, pipeline: &Pipeline);
    fn on_stage_start(&self, stage: &Stage);
    fn on_stage_complete(&self, stage: &Stage, result: &StageResult);
    fn on_pipeline_complete(&self, pipeline: &Pipeline, result: &PipelineResult);
}

// Listener: Logging
pub struct LoggingListener;

impl PipelineEventListener for LoggingListener {
    fn on_stage_start(&self, stage: &Stage) {
        info!("Starting stage: {}", stage.name);
    }

    fn on_stage_complete(&self, stage: &Stage, result: &StageResult) {
        info!("Stage '{}' completed with result: {:?}", stage.name, result);
    }
}

// Listener: Metrics
pub struct MetricsListener {
    metrics: MetricsCollector,
}

impl PipelineEventListener for MetricsListener {
    fn on_stage_complete(&self, stage: &Stage, result: &StageResult) {
        let duration = stage.start_time.elapsed();
        self.metrics.record_stage_duration(&stage.name, duration);
        self.metrics.record_stage_result(&stage.name, result);
    }
}

// Emitter de eventos
pub struct EventDispatcher {
    listeners: Vec<Box<dyn PipelineEventListener>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    pub fn add_listener(&mut self, listener: Box<dyn PipelineEventListener>) {
        self.listeners.push(listener);
    }

    fn notify_stage_start(&self, stage: &Stage) {
        for listener in &self.listeners {
            listener.on_stage_start(stage);
        }
    }
}

// Uso
let mut dispatcher = EventDispatcher::new();
dispatcher.add_listener(Box::new(LoggingListener));
dispatcher.add_listener(Box::new(MetricsListener::new()));

// Durante ejecución
dispatcher.notify_stage_start(&stage);
// execute stage...
dispatcher.notify_stage_complete(&stage, &result);
```

---

## Alto Rendimiento

### 1. Profiling y Benchmarking

**Herramientas**:
- `cargo-flamegraph`: Flame graphs para identificar hotspots
- `cargo-criterion`: Benchmarks precisos y comparativos
- `perf` (Linux): Profiling a nivel de CPU
- `heaptrack`: Profiling de memoria

**Benchmarking Continuo**:

```rust
// benches/pipeline_execution.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_pipeline_with_n_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_stages");

    for n in [1, 5, 10, 50, 100].iter() {
        let stages: Vec<_> = (0..*n)
            .map(|i| stage!(format!("Stage{}", i), steps!(sh!("echo test"))))
            .collect();

        let pipeline = pipeline!(
            agent_any(),
            stages!(stages),
            post!(always(sh!("echo done")))
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            n,
            |b, _| {
                b.iter(|| {
                    let executor = LocalExecutor::new();
                    black_box(executor.execute(black_box(&pipeline)))
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_pipeline_with_n_stages);
criterion_main!(benches);
```

**Target de rendimiento**:
- Startup time: < 10ms
- Parsing de pipeline (100 stages): < 50ms
- Overhead de ejecución: < 100ms (sin contar comandos externos)
- Memory footprint: < 50MB (pipeline mediano)

### 2. Caching y Memoization

**Caching de compilación Cargo**:

```rust
pub struct CargoCache {
    cache_dir: PathBuf,
    hash_file: PathBuf,
}

impl CargoCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        let hash_file = cache_dir.join("cache_hash");
        Self { cache_dir, hash_file }
    }

    pub fn compute_cache_key(&self, lock_file: &Path) -> Result<String, CacheError> {
        let contents = fs::read_to_string(lock_file)?;
        let hash = Sha256::digest(contents);
        Ok(format!("cargo-{}", hex::encode(hash)))
    }

    pub fn restore_cache(&self, key: &str) -> Result<bool, CacheError> {
        let cache_path = self.cache_dir.join(key);
        if !cache_path.exists() {
            return Ok(false);
        }

        // Restore $CARGO_HOME/cache
        let home = std::env::var("CARGO_HOME").unwrap_or_else(|_| {
            dirs::home_dir()
                .map(|h| h.join(".cargo"))
                .unwrap()
                .to_string_lossy()
                .to_string()
        });

        // Copiar cache al directorio Cargo
        // ...

        Ok(true)
    }

    pub fn save_cache(&self, key: &str) -> Result<(), CacheError> {
        let cache_path = self.cache_dir.join(key);

        // Guardar $CARGO_HOME/cache
        // ...

        Ok(())
    }
}
```

**Memoization de expresiones**:

```rust
use lru::LruCache;

pub struct ExpressionEvaluator {
    cache: Arc<Mutex<LruCache<String, String>>>,
}

impl ExpressionEvaluator {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }

    pub fn eval(&self, expr: &str, env: &Environment) -> String {
        // Check cache
        if let Some(cached) = self.cache.lock().unwrap().get_mut(expr) {
            return cached.clone();
        }

        // Evaluar expresión
        let result = self.eval_internal(expr, env);

        // Guardar en cache
        self.cache.lock().unwrap().put(expr.to_string(), result.clone());

        result
    }
}
```

### 3. Parallelism y Concurrency

**Ejecución paralela de stages independientes**:

```rust
use rayon::prelude::*;

pub fn execute_stages_parallel(
    stages: &[Stage],
    context: &PipelineContext,
    max_concurrency: usize,
) -> Vec<StageExecutionResult> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(max_concurrency)
        .build()
        .unwrap();

    pool.install(|| {
        stages
            .par_iter()
            .map(|stage| {
                let result = execute_stage(stage, context);
                StageExecutionResult {
                    stage_name: stage.name.clone(),
                    result,
                }
            })
            .collect()
    });
}
```

**Async I/O para commands**:

```rust
use tokio::process::Command;

pub async fn execute_command_async(
    cmd: &str,
    timeout: Duration,
) -> Result<String, CommandError> {
    let output = tokio::time::timeout(
        timeout,
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output(),
    )
    .await??;

    if !output.status.success() {
        return Err(CommandError::NonZeroExit(
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

### 4. Memory Management

**Evitar clonaciones en hot paths**:

```rust
// ❌ Mal: Clona en cada iteración
for stage in &pipeline.stages {
    let name = stage.name.clone(); // Clonación innecesaria
    println!("Executing stage: {}", name);
}

// ✅ Bien: Usa referencia
for stage in &pipeline.stages {
    println!("Executing stage: {}", stage.name); // No clona
}
```

**Allocation pooling**:

```rust
use object_pool::growable::GrowPool;

pub struct StringPool {
    pool: GrowPool<String>,
}

impl StringPool {
    pub fn new() -> Self {
        Self {
            pool: GrowPool::new(|| String::with_capacity(256)),
        }
    }

    pub fn acquire(&self) -> StringHolder {
        let string = self.pool.pull();
        StringHolder { string, pool: self }
    }
}

pub struct StringHolder<'a> {
    string: object_pool::PullGuard<'a, String>,
    pool: &'a StringPool,
}

impl<'a> Drop for StringHolder<'a> {
    fn drop(&mut self) {
        // Clear antes de retornar al pool
        self.string.clear();
        // Retorna automáticamente al pool
    }
}
```

### 5. Zero-Copy Parsing

**Parsing de YAML sin copiar strings**:

```rust
use serde::de::DeserializeOwned;
use serde_yaml::Value;

pub fn parse_yaml_no_copy<T: DeserializeOwned>(
    input: &str,
) -> Result<T, ParseError> {
    serde_yaml::from_str(input)
}

// Parsing con Cow para evitar copiar cuando el input es estático
pub fn parse_template<'a>(
    template: &'a str,
    vars: &HashMap<&str, &'a str>,
) -> Cow<'a, str> {
    if vars.is_empty() {
        Cow::Borrowed(template)
    } else {
        let result = expand_vars(template, vars);
        Cow::Owned(result)
    }
}
```

---

## Excelencia Operativa

### 1. Observabilidad

**Logging estructurado**:

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(context))]
pub fn execute_pipeline(
    pipeline: &Pipeline,
    context: &PipelineContext,
) -> Result<PipelineResult, PipelineError> {
    info!(
        pipeline_name = %pipeline.name,
        stages_count = pipeline.stages.len(),
        "Starting pipeline execution"
    );

    for stage in &pipeline.stages {
        info!(stage_name = %stage.name, "Executing stage");
        let result = execute_stage(stage, context);

        match &result {
            Ok(result) => info!(stage_result = ?result, "Stage completed"),
            Err(e) => error!(error = %e, "Stage failed"),
        }
    }

    Ok(PipelineResult::Success)
}
```

**Métricas con Prometheus**:

```rust
use prometheus::{Counter, Histogram, IntGauge, Registry};

pub struct PipelineMetrics {
    pipeline_total: Counter,
    pipeline_duration: Histogram,
    stages_total: Counter,
    stages_duration: Histogram,
    cache_hits: IntGauge,
    cache_misses: IntGauge,
}

impl PipelineMetrics {
    pub fn new() -> Self {
        Self {
            pipeline_total: register_counter!(
                "rustline_pipeline_executions_total",
                "Total number of pipeline executions"
            ).unwrap(),

            pipeline_duration: register_histogram!(
                "rustline_pipeline_duration_seconds",
                "Pipeline execution duration",
                vec![0.01, 0.1, 0.5, 1.0, 5.0, 30.0]
            ).unwrap(),

            stages_total: register_counter!(
                "rustline_stages_executions_total",
                "Total number of stage executions"
            ).unwrap(),

            stages_duration: register_histogram!(
                "rustline_stage_duration_seconds",
                "Stage execution duration"
            ).unwrap(),

            cache_hits: register_int_gauge!(
                "rustline_cache_hits",
                "Number of cache hits"
            ).unwrap(),

            cache_misses: register_int_gauge!(
                "rustline_cache_misses",
                "Number of cache misses"
            ).unwrap(),
        }
    }

    pub fn record_pipeline_start(&self) {
        self.pipeline_total.inc();
    }

    pub fn record_pipeline_duration(&self, duration: Duration) {
        self.pipeline_duration.observe(duration.as_secs_f64());
    }
}
```

### 2. Error Handling

**Structured errors con `thiserror`**:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationError),

    #[error("Stage '{stage}' failed: {error}")]
    StageFailed {
        stage: String,
        error: Box<StageError>,
    },

    #[error("Command failed with exit code {code}: {stderr}")]
    CommandFailed {
        code: i32,
        stderr: String,
    },

    #[error("Timeout after {duration:?}")]
    Timeout {
        duration: Duration,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum StageError {
    #[error("Step failed: {0}")]
    StepError(#[from] StepError),
}

#[derive(Error, Debug)]
pub enum StepError {
    #[error("Shell command failed: {0}")]
    ShellError(String),
}
```

**Contexto en errores**:

```rust
#[instrument(skip(context))]
pub fn execute_stage_with_context(
    stage: &Stage,
    context: &PipelineContext,
) -> Result<StageResult, PipelineError> {
    let result = execute_stage(stage, context);

    match &result {
        Ok(r) => info!(stage_result = ?r, "Stage completed successfully"),
        Err(e) => {
            error!(
                stage_name = %stage.name,
                error = %e,
                "Stage failed"
            );
        }
    }

    result
}
```

### 3. Configuration Management

**Carga de configuración con validación**:

```rust
use serde::{Deserialize, Validate};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct ExecutorConfig {
    #[validate(length(min = 1))]
    pub workdir: String,

    #[validate(range(min = 1, max = 100))]
    pub max_parallel_stages: usize,

    pub cache: CacheConfig,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CacheConfig {
    pub enabled: bool,

    #[validate(length(min = 1))]
    pub dir: String,

    #[validate(range(min = 1))]
    pub max_size_mb: usize,
}

impl ExecutorConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path)?;
        let config: ExecutorConfig = serde_yaml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }
}
```

### 4. Health Checks

```rust
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub dependencies: Vec<DependencyHealth>,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyHealth {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
}

pub fn check_health() -> HealthStatus {
    let dependencies = vec![
        DependencyHealth {
            name: "rust".to_string(),
            available: check_rust_available(),
            version: get_rust_version(),
        },
        DependencyHealth {
            name: "docker".to_string(),
            available: check_docker_available(),
            version: get_docker_version(),
        },
        DependencyHealth {
            name: "kubectl".to_string(),
            available: check_kubectl_available(),
            version: get_kubectl_version(),
        },
    ];

    let all_available = dependencies.iter().all(|d| d.available);

    HealthStatus {
        status: if all_available { "healthy" } else { "degraded" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        dependencies,
        uptime: get_uptime(),
    }
}
```

### 5. Graceful Shutdown

```rust
use tokio::signal;
use tokio::sync::Semaphore;

pub struct GracefulShutdown {
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    running_tasks: Semaphore,
}

impl GracefulShutdown {
    pub fn new(max_tasks: usize) -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        Self {
            shutdown_tx,
            running_tasks: Semaphore::new(max_tasks),
        }
    }

    pub async fn acquire_task_slot(&self) -> Result<TaskGuard, ShutdownError> {
        tokio::select! {
            permit = self.running_tasks.acquire() => {
                Ok(TaskGuard {
                    permit: permit.unwrap(),
                    shutdown_tx: self.shutdown_tx.clone(),
                })
            }
            _ = self.shutdown_tx.subscribe().recv() => {
                Err(ShutdownError::ShuttingDown)
            }
        }
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

pub struct TaskGuard<'a> {
    permit: SemaphorePermit<'a>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl Drop for TaskGuard<'_> {
    fn drop(&mut self) {
        // Permit es liberado automáticamente
    }
}
```

---

## Testing y Validación de Arquitectura

### Arquitectura Tests

```rust
#[test]
fn test_domain_layer_has_no_external_dependencies() {
    // Verifica que el dominio no dependa de filesystem, network, etc.
    use rustline::pipeline::Pipeline;

    let pipeline = Pipeline::builder()
        .agent(AgentType::Any)
        .build()
        .unwrap();

    // Operaciones del dominio no requieren recursos externos
    assert_eq!(pipeline.stages.len(), 0);
}

#[test]
fn test_executors_are_interchangeable() {
    use rustline::executor::{LocalExecutor, DockerExecutor, PipelineExecutor};

    let pipeline = create_simple_pipeline();

    let local = LocalExecutor::new();
    let docker = DockerExecutor::new();

    // Ambos implementan el mismo trait
    assert!(local.execute(&pipeline).is_ok());
    assert!(docker.execute(&pipeline).is_ok());
}
```

### Performance Tests

```rust
#[test]
fn test_parsing_performance() {
    let large_pipeline = generate_large_pipeline(1000);

    let start = Instant::now();
    let _ = parse_pipeline(&large_pipeline);
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_millis(100));
}

#[test]
fn test_memory_usage() {
    let pipeline = generate_large_pipeline(1000);

    let memory_before = get_memory_usage();
    let _ = execute_pipeline(&pipeline);
    let memory_after = get_memory_usage();

    let memory_used = memory_after - memory_before;

    // Menos de 100MB para pipeline de 1000 stages
    assert!(memory_used < 100 * 1024 * 1024);
}
```

---

## Referencias

- [The Rust Book - Performance](https://doc.rust-lang.org/book/ch20-00-advanced-features.html)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [Zero-Copy in Rust](https://manishearth.github.io/blog/2017/01/07/rust-vs-c-zero-cost-abstractions/)
- [Tracing](https://docs.rs/tracing/)
- [Prometheus Client](https://docs.rs/prometheus/)
- [Thiserror](https://docs.rs/thiserror/)
- [Rayon](https://docs.rs/rayon/)
