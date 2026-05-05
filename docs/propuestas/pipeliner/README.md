# Features de Pipeline Engine: GISS → Pipeliner

**Fecha**: 2026-05-05
**Alcance**: Tecnología de ejecución de pipelines y sus extensiones
**Excluido**: Orquestación, Docker, K8s, Frontend

---

## Resumen

Análisis de las features del motor de pipeline de GISS que amplían la capacidad de **ejecución** y **extensibilidad** de Pipeliner. Nos centramos exclusivamente en lo que pasa **dentro** del pipeline: cómo se define, cómo se ejecuta, cómo se extiende con librerías y módulos, y cómo se reporta progreso.

---

## 1. Visión General: Qué Tiene Cada Proyecto

### Pipeliner (Rust) — Lo que YA tiene

```
pipeliner-core/       Pipeline, Stage, Step, Agent, Parameters, Environment
pipeliner-executor/   LocalExecutor, PipelineContext, ExecutionStrategy
pipeliner-events/     EventBus, EventStore (in-memory)
pipeliner-macros/     pipeline!, stage!, steps!, sh!, echo!, retry!, timeout!, dir!
pipeliner-worker/     WorkerPool, JobQueue, Scheduler
pipeliner-cli/        CLI con subcomandos
```

**Step types actuales**: `echo`, `shell`, `retry(n, step)`, `timeout(secs, step)`, `dir(path, steps)`

### GISS (Groovy) — Lo que aporta

```
PipelineRuntime          Ciclo de vida completo de ejecución (init→libs→engine→scm→load→run)
LibraryLoader            Carga dinámica de librerías con 3 source retrievers
15 Módulos               core, maven, gradle, container, helm, scanner, policies...
CDI Pattern              Inyección de dependencias en vars/ steps
Stage Markers            __STAGE__ JSON en stdout para progreso estructurado
CustomGroovyScriptEngine Motor de scripting con imports/transformers
when/errorHandler/is     Steps de control de flujo avanzados
```

---

## 2. Features a Portar

### Feature 1: Sistema de Librerías con Source Retrievers

**Por qué importa**: Es el mecanismo central de extensibilidad. Permite cargar funcionalidad adicional en runtime desde distintas fuentes (git, local, JARs). Pipeliner no tiene nada equivalente.

**Cómo funciona en GISS**:

```
LibraryConfiguration
├── name: "my-lib"
├── sourceRetrieverType: "GitSource" | "LocalSource" | "LocalLib"
├── sourcePath: URL o path local
├── defaultVersion: "main"
└── modules: [LibraryModule(name, path)]
```

Cada librería se descompone en tres capas que se cargan en orden fijo:
1. `src/` → clases compiladas al classpath
2. `vars/*.groovy` → instanciadas como variables globales del pipeline
3. `resources/` → registradas como recursos accesibles

Las librerías se **deduplican** por nombre (`libRecords`). Cargar la misma librería dos veces es no-op.

**Propuesta para Pipeliner**:

```rust
// pipeliner-library/src/lib.rs

/// Contrato para recuperar artefactos de librería desde distintas fuentes
#[async_trait]
pub trait SourceRetriever: Send + Sync {
    async fn retrieve(&self, config: &LibraryConfig) -> Result<LibraryArtifacts, LibraryError>;
}

/// Clona un repo git a un directorio temporal
pub struct GitSource {
    pub remote_url: String,
    pub default_branch: String,
}

/// Resuelve un path local absoluto
pub struct LocalSource {
    pub base_path: PathBuf,
}

/// Resuelve JARs o directorios recursivamente
pub struct LocalLib {
    pub base_path: PathBuf,
}

/// Artefactos extraídos de una librería
pub struct LibraryArtifacts {
    pub source_files: Vec<PathBuf>,    // src/ → Rust modules
    pub step_files: Vec<PathBuf>,      // vars/ → Custom steps
    pub resource_files: Vec<PathBuf>,  // resources/ → Static resources
}

/// Loader principal con caché y deduplicación
pub struct LibraryLoader {
    retrievers: HashMap<RetrieverType, Box<dyn SourceRetriever>>,
    cache: LibraryCache,
    loaded: HashSet<String>,  // Dedup por "name@version"
}

impl LibraryLoader {
    pub async fn load(&mut self, config: &LibraryConfig) -> Result<LibraryArtifacts, LibraryError> {
        let key = format!("{}@{}", config.name, config.version());
        if self.loaded.contains(&key) {
            return Ok(self.cache.get(&key));  // No-op si ya cargada
        }

        let retriever = self.retrievers.get(&config.retriever_type)?;
        let artifacts = retriever.retrieve(config).await?;
        self.cache.insert(key, &artifacts);
        self.loaded.insert(key);
        Ok(artifacts)
    }
}
```

**Invariants críticos**:
- `sourcePath` es obligatorio (sin él → error descriptivo)
- GitSource URL **no** se resuelve con `Path::new()` — es una URL remota
- Orden de carga: `src/` → steps → resources
- Deduplicación por `name@version`

**Complejidad**: HARD (3-4 semanas)

---

### Feature 2: Plugin/Step Registry con CDI Pattern

**Por qué importa**: En GISS, cada módulo expone pasos de pipeline (`maven.build()`, `gitTool.clone()`, `errorHandler.call()`) que se resuelven por CDI. Pipeliner necesita un mecanismo equivalente para que los steps no estén hardcoded.

**Cómo funciona en GISS**:

```groovy
// vars/config.groovy
class config {
    def load(List profiles = []) {
        def client = getContext().getComponent(IConfigClient.class)  // CDI lookup
        client.load(profiles)
    }
}

// vars/gitTool.groovy
class gitTool {
    def cloneInAgent() { sh "git clone ${scm.url}" }
    def createTag(tag) { sh "git tag -a ${tag} -m '${tag}'" }
    def tagExists(tag) { return sh(returnStdout: true, script: "git tag -l ${tag}") }
}

// vars/errorHandler.groovy
class errorHandler {
    def call(Closure code) {
        try { code() } catch (Exception e) { log.error(e); throw e }
    }
}

// vars/when.groovy — control de flujo condicional
class when {
    def call(expr, Closure code) { if (expr) code() }
    def not(expr, Closure code) { if (!expr) code() }
    def any(exprs, Closure code) { if (exprs.any()) code() }
    def all(exprs, Closure code) { if (exprs.every()) code() }
}

// vars/is.groovy — environment detection
class is {
    def integration() { env.ENV == 'integration' }
    def certification() { env.ENV == 'certification' }
    def preproduction() { env.ENV == 'preproduction' }
    def production() { env.ENV == 'production' }
}
```

**Propuesta para Pipeliner**:

```rust
// pipeliner-core/src/context.rs

/// Contexto de ejecución compartido entre steps
pub struct PipelineContext {
    env: HashMap<String, String>,
    components: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    step_registry: StepRegistry,
}

impl PipelineContext {
    /// CDI lookup: obtener un componente por tipo
    pub fn get_component<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|c| c.downcast_ref::<T>())
    }

    /// Registrar un componente (CDI register)
    pub fn register_component<T: 'static + Send + Sync>(&mut self, component: T) {
        self.components.insert(TypeId::of::<T>(), Box::new(component));
    }

    /// Lookup de step por nombre (para librerías dinámicas)
    pub fn get_step(&self, name: &str) -> Option<Arc<dyn StepFactory>> {
        self.step_registry.get(name)
    }
}

/// Registry de steps extensibles
pub struct StepRegistry {
    steps: HashMap<String, Arc<dyn StepFactory>>,
}

/// Trait para factorías de steps (cada módulo implementa esto)
pub trait StepFactory: Send + Sync {
    fn name(&self) -> &str;
    fn create(&self, args: &[Value]) -> Result<Step, StepError>;
}
```

**Steps de control de flujo que aporta GISS**:

```rust
// pipeliner-core/src/steps/flow.rs

/// errorHandler.call(closure) — wrap con manejo de errores
pub fn error_handler<F>(f: F) -> Step
where F: FnOnce() -> Result<(), StepError> + Send + 'static
{
    Step::from_closure(move |_| {
        match f() {
            Ok(()) => StepResult::success(),
            Err(e) => {
                tracing::error!(error = %e, "Error handled by errorHandler");
                StepResult::failed(e)
            }
        }
    })
}

/// when(expr, code) — ejecución condicional
pub fn when<F>(condition: bool, step: Step) -> Step {
    if condition { step } else { Step::skip("when condition false") }
}

/// when.not(expr, code)
pub fn when_not<F>(condition: bool, step: Step) -> Step {
    when(!condition, step)
}

/// when.any(exprs, code)
pub fn when_any(conditions: &[bool], step: Step) -> Step {
    when(conditions.iter().any(|&c| c), step)
}

/// when.all(exprs, code)
pub fn when_all(conditions: &[bool], step: Step) -> Step {
    when(conditions.iter().all(|&c| c), step)
}

/// is.integration(), is.production() — environment detection
pub struct EnvironmentCheck<'a> {
    env: &'a HashMap<String, String>,
}

impl<'a> EnvironmentCheck<'a> {
    pub fn integration(&self) -> bool { self.env.get("ENV").map(|v| v == "integration").unwrap_or(false) }
    pub fn certification(&self) -> bool { self.env.get("ENV").map(|v| v == "certification").unwrap_or(false) }
    pub fn preproduction(&self) -> bool { self.env.get("ENV").map(|v| v == "preproduction").unwrap_or(false) }
    pub fn production(&self) -> bool { self.env.get("ENV").map(|v| v == "production").unwrap_or(false) }
}
```

**Complejidad**: MEDIUM (2-3 semanas)

---

### Feature 3: Stage Markers — Progreso Estructurado via stdout

**Por qué importa**: Es el único mecanismo de progreso del pipeline. El executor emite JSON markers por stdout, y un parser los convierte en eventos. Es simple, universal y language-agnostic. Pipeliner ya tiene EventBus pero no tiene este protocolo de marcado.

**Protocolo en GISS**:

```
__STAGE__{"type":"STARTED","name":"Checkout","ts":1709836800}
__STAGE__{"type":"COMPLETED","name":"Checkout","ts":1709836805,"duration":5000,"result":"SUCCESS"}
__STAGE__{"type":"ERROR","name":"Build","ts":1709836810,"message":"Build failed"}
__STAGE__{"type":"SKIPPED","name":"Test","ts":1709836810}
```

**Propuesta para Pipeliner**:

```rust
// pipeliner-events/src/markers.rs

pub const STAGE_MARKER: &str = "__STAGE__";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StageMarker {
    #[serde(rename = "STARTED")]
    Started {
        name: String,
        ts: u64,  // Unix epoch seconds
    },
    #[serde(rename = "COMPLETED")]
    Completed {
        name: String,
        ts: u64,
        #[serde(default)]
        duration: u64,  // milliseconds
        result: StageResult,
    },
    #[serde(rename = "ERROR")]
    Error {
        name: String,
        ts: u64,
        message: String,
    },
    #[serde(rename = "SKIPPED")]
    Skipped {
        name: String,
        ts: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StageResult {
    Success,
    Failure,
    Unstable,
    Aborted,
}

/// Emisor de markers (lo usa el executor al ejecutar stages)
pub struct StageMarkerEmitter;

impl StageMarkerEmitter {
    pub fn started(name: &str) -> String {
        let marker = StageMarker::Started {
            name: name.to_string(),
            ts: now_ts(),
        };
        format!("{}{}", STAGE_MARKER, serde_json::to_string(&marker).unwrap())
    }

    pub fn completed(name: &str, duration_ms: u64, result: StageResult) -> String {
        let marker = StageMarker::Completed {
            name: name.to_string(),
            ts: now_ts(),
            duration: duration_ms,
            result,
        };
        format!("{}{}", STAGE_MARKER, serde_json::to_string(&marker).unwrap())
    }

    pub fn error(name: &str, message: &str) -> String {
        let marker = StageMarker::Error {
            name: name.to_string(),
            ts: now_ts(),
            message: message.to_string(),
        };
        format!("{}{}", STAGE_MARKER, serde_json::to_string(&marker).unwrap())
    }
}

/// Parser de markers (consume stdout línea a línea)
pub struct StageMarkerParser;

impl StageMarkerParser {
    /// Parsea una línea de stdout. Si contiene __STAGE__, devuelve el marker.
    /// Si no, devuelve None (es output normal del comando).
    pub fn parse_line(line: &str) -> Option<StageMarker> {
        if let Some(pos) = line.find(STAGE_MARKER) {
            let json = &line[pos + STAGE_MARKER.len()..];
            serde_json::from_str(json).ok()
        } else {
            None
        }
    }
}
```

**Uso en el executor**:

```rust
// Dentro de LocalExecutor::execute_stage()
fn execute_stage(&self, stage: &Stage, ctx: &PipelineContext) -> StageResult {
    // Emitir STARTED
    println!("{}", StageMarkerEmitter::started(&stage.name));

    let start = Instant::now();
    let result = self.execute_steps(&stage.steps, ctx);
    let duration = start.elapsed().as_millis() as u64;

    match result {
        Ok(()) => {
            println!("{}", StageMarkerEmitter::completed(&stage.name, duration, StageResult::Success));
            StageResult::Success
        }
        Err(e) => {
            println!("{}", StageMarkerEmitter::error(&stage.name, &e.to_string()));
            StageResult::Failure
        }
    }
}
```

**Complejidad**: EASY (1 semana)

---

### Feature 4: Ciclo de Vida del Pipeline Runtime

**Por qué importa**: GISS tiene un orden de inicialización fijo que garantiza que las dependencias estén disponibles antes de ejecutar el script. Pipeliner ejecuta directamente pero no tiene un ciclo de vida estructurado.

**Orden en GISS (INVARIANTE — no reordenar)**:

```
1. WorkerDefinition::fromMap(config)    → Parsear configuración
2. initializeLibraries()                → Resolver y cargar librerías
3. initializeScriptEngine()             → Construir motor de scripting
4. initializeSourceCode()               → Clonar SCM del proyecto
5. loadLibraries(script)                → Inyectar vars de librerías en el binding
6. executeScript(script)                → Ejecutar el DSL del pipeline
```

**Propuesta para Pipeliner**:

```rust
// pipeliner-executor/src/runtime.rs

/// Fases del ciclo de vida del pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    Init,
    LoadLibraries,
    SetupEngine,
    LoadSourceCode,
    BindSteps,
    Execute,
    Completed,
    Failed,
}

/// Resultado de cada fase
pub struct PhaseResult {
    pub phase: LifecyclePhase,
    pub duration: Duration,
    pub output: Option<String>,
}

/// Motor de ejecución con ciclo de vida estructurado
pub struct PipelineRuntime {
    config: WorkerDefinition,
    context: PipelineContext,
    library_loader: LibraryLoader,
    phase_log: Vec<PhaseResult>,
}

impl PipelineRuntime {
    pub async fn run(mut self, pipeline: &Pipeline) -> Result<ExecutionResult, RuntimeError> {
        // Phase 1: Init — parsear y validar configuración
        self.phase(LifecyclePhase::Init, |rt| {
            rt.validate_config()
        }).await?;

        // Phase 2: LoadLibraries — resolver source retrievers
        self.phase(LifecyclePhase::LoadLibraries, |rt| {
            rt.load_all_libraries()
        }).await?;

        // Phase 3: SetupEngine — preparar entorno de ejecución
        self.phase(LifecyclePhase::SetupEngine, |rt| {
            rt.setup_execution_engine()
        }).await?;

        // Phase 4: LoadSourceCode — clonar SCM si está configurado
        self.phase(LifecyclePhase::LoadSourceCode, |rt| {
            rt.load_source_code()
        }).await?;

        // Phase 5: BindSteps — inyectar steps de librerías en el contexto
        self.phase(LifecyclePhase::BindSteps, |rt| {
            rt.bind_library_steps()
        }).await?;

        // Phase 6: Execute — ejecutar el pipeline
        self.phase(LifecyclePhase::Execute, |rt| {
            rt.execute_pipeline(pipeline)
        }).await?;

        Ok(ExecutionResult {
            phases: self.phase_log,
            total_duration: self.phase_log.iter().map(|p| p.duration).sum(),
        })
    }

    async fn phase<F>(&mut self, phase: LifecyclePhase, f: F) -> Result<(), RuntimeError>
    where F: FnOnce(&mut Self) -> Result<(), RuntimeError>
    {
        tracing::info!(phase = ?phase, "Starting lifecycle phase");
        let start = Instant::now();
        let result = f(self);
        let duration = start.elapsed();

        self.phase_log.push(PhaseResult {
            phase,
            duration,
            output: None,
        });

        if result.is_err() {
            self.phase_log.last_mut().unwrap().phase = LifecyclePhase::Failed;
        }

        result
    }
}
```

**Complejidad**: MEDIUM (2 semanas)

---

### Feature 5: Ecosistema de Módulos / Steps de Negocio

**Por qué importa**: GISS tiene 15 módulos que encapsulan steps reutilizables. Pipeliner tiene `sh!` y `echo!` pero no steps de dominio. Portar estos como crates de steps amplía masivamente la utilidad del pipeline.

**Módulos de GISS y su mapping a Rust**:

| Módulo GISS | Var | Funciones | Crate Propuesto |
|-------------|-----|-----------|----------------|
| **core** | `config` | `load(profiles?)` | `pipeliner-steps-core` |
| **core** | `gitTool` | `cloneInAgent()`, `createTag()`, `tagExists()` | `pipeliner-steps-git` |
| **core** | `errorHandler` | `call(closure)` | `pipeliner-steps-core` |
| **core** | `restClient` | `create(steps)` | `pipeliner-steps-http` |
| **core** | `when` / `is` | Condicionales, detección de entorno | `pipeliner-steps-core` |
| **core** | `workspace` | `checkWatchedFiles()` | `pipeliner-steps-core` |
| **core** | `loggerTool` | `debug/info/warn/error/fatal` | `pipeliner-steps-core` |
| **core** | `asdfTool` | `install(tool,ver)`, `current(tool)` | `pipeliner-steps-tooling` |
| **core** | `releaseStages` | `releaseSnapshot()`, `releaseMaster()` | `pipeliner-steps-release` |
| **maven** | `mavenStages` | `build()`, `test()`, `publish()` | `pipeliner-steps-maven` |
| **gradle** | `gradleStages` | `build()`, `test()`, `publish()` | `pipeliner-steps-gradle` |
| **container** | `container` | `build()`, `promote()` | `pipeliner-steps-container` |
| **helm** | `helmStages` | `build()`, `test()`, `deploy()`, `promote()` | `pipeliner-steps-helm` |
| **scanner** | `scannerStages` | `execute()`, `executeUtf8()` | `pipeliner-steps-scanner` |
| **policies** | `policiesStages` | `validatePolicies()` | `pipeliner-steps-policy` |
| **notification** | `notifications` | `sendEmailsGenericReports()` | `pipeliner-steps-notify` |
| **artifact_repository** | `artifactRepository` | `upload/download/searchArtifact` | `pipeliner-steps-artifact` |
| **api_management** | `apiManagementStages` | `deploy()`, `build()`, `publish()` | `pipeliner-steps-api` |

**Ejemplo: Módulo Maven como crate**:

```rust
// pipeliner-steps-maven/src/lib.rs

pub struct MavenSteps {
    goals: Vec<String>,
    profiles: Vec<String>,
    properties: HashMap<String, String>,
}

impl MavenSteps {
    pub fn build() -> Vec<Step> {
        vec![
            echo!("📦 Maven build..."),
            sh!("mvn clean package -B"),
        ]
    }

    pub fn test() -> Vec<Step> {
        vec![
            echo!("🧪 Maven test..."),
            sh!("mvn test -B"),
        ]
    }

    pub fn publish() -> Vec<Step> {
        vec![
            echo!("📤 Maven publish..."),
            sh!("mvn deploy -DskipTests -B"),
        ]
    }

    pub fn with_profiles(profiles: &[&str]) -> MavenBuilder {
        MavenBuilder::new().profiles(profiles)
    }
}

/// Se usa como step custom en el pipeline
pub fn maven_build() -> Step {
    Step::composed("maven-build", MavenSteps::build())
}

pub fn maven_test() -> Step {
    Step::composed("maven-test", MavenSteps::test())
}

pub fn maven_publish() -> Step {
    Step::composed("maven-publish", MavenSteps::publish())
}
```

**Uso en DSL**:

```rust
use pipeliner_steps_maven::*;
use pipeliner_core::prelude::*;

let pipeline = pipeline! {
    agent { any() }
    stages {
        stage!("Build", steps!(
            maven_build()
        ))
        stage!("Test", steps!(
            maven_test()
        ))
        stage!("Publish", steps!(
            maven_publish()
        ))
    }
};
```

**Complejidad**: EASY por módulo (1 semana cada uno) / 8-12 semanas total

---

### Feature 6: Configuración de Pipeline — WorkerDefinition como Modelo

**Por qué importa**: WorkerDefinition es el objeto central que configura TODO sobre cómo se ejecuta un pipeline: credenciales, librerías, variables de entorno, SCM, y cloud configs. Pipeliner usa structs simples pero no tiene un modelo unificado.

**Estructura en GISS**:

```
WorkerDefinition (kind, apiVersion)
└── spec: WorkerSpec
    ├── credentialProviders: List<CredentialProviderConfig>
    ├── libraries: List<LibraryConfiguration>
    │   └── LibraryConfiguration
    │       ├── name, sourcePath, defaultVersion
    │       ├── sourceRetrieverType: "GitSource" | "LocalSource" | "LocalLib"
    │       └── modules: [LibraryModule(name, path)]
    ├── environmentVars: Map<String,String>
    ├── projectSourceCodeManagement: SourceCodeManagementSpec
    │   ├── scmType, url, branch, credentialsId
    │   └── shallowClone, submoduleRecursive
    ├── cloudConfigs: List<CloudConfigSpec>  (kind='Docker' | 'Kubernetes')
    │   └── CloudConfigSpec
    │       ├── kind: "Docker" | "Kubernetes"
    │       ├── agents: [AgentConfig(image, args, envVars)]
    │       └── workspace, volumes, etc.
    └── globalConfigFiles: List<GlobalConfigFiles>
```

**Propuesta para Pipeliner**:

```rust
// pipeliner-core/src/config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub version: String,
    pub spec: PipelineSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    #[serde(default)]
    pub libraries: Vec<LibraryConfig>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub scm: Option<ScmConfig>,
    #[serde(default)]
    pub credentials: Vec<CredentialConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryConfig {
    pub name: String,
    pub source_path: String,
    #[serde(default)]
    pub default_version: Option<String>,
    pub retriever_type: RetrieverType,
    #[serde(default)]
    pub modules: Vec<LibraryModule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetrieverType {
    GitSource,
    LocalSource,
    LocalLib,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScmConfig {
    pub url: String,
    pub branch: String,
    #[serde(default)]
    pub credentials_id: Option<String>,
    #[serde(default = "default_true")]
    pub shallow_clone: bool,
    #[serde(default = "default_true")]
    pub submodule_recursive: bool,
}

// Carga desde YAML/JSON
impl PipelineConfig {
    pub fn from_yaml(yaml: &str) -> Result<Self, ConfigError> {
        serde_yaml::from_str(yaml).map_err(ConfigError::Yaml)
    }

    pub fn from_json(json: &str) -> Result<Self, ConfigError> {
        serde_json::from_str(json).map_err(ConfigError::Json)
    }
}
```

**Complejidad**: MEDIUM (1-2 semanas)

---

### Feature 7: Step Types Adicionales — Lo que GISS tiene y Pipeliner no

| Step Type | GISS | Pipeliner | Propuesta |
|-----------|------|-----------|-----------|
| `sh()` | ✅ | ✅ `sh!` | Ya existe |
| `echo()` | ✅ | ✅ `echo!` | Ya existe |
| `retry()` | ✅ | ✅ `retry!` | Ya existe |
| `timeout()` | ✅ | ✅ `timeout!` | Ya existe |
| `dir()` | ✅ | ✅ `dir!` | Ya existe |
| **`stash/unstash`** | ✅ | ❌ | **Portar** |
| **`input()`** | ✅ | ❌ | **Portar** |
| **`when()`** | ✅ | ❌ | **Portar** |
| **`errorHandler()`** | ✅ | ❌ | **Portar** |
| **`is()`** | ✅ | ❌ | **Portar** |
| **`parallel()`** | ✅ | ✅ `parallel!` | Ya existe |
| **`matrix()`** | ❌ | ✅ `matrix!` | Pipeliner tiene más |
| **`withCredentials()`** | ✅ | ❌ | **Portar** |
| **`checkout()`** | ✅ | ❌ | **Portar** |
| **`archiveArtifacts()`** | ✅ | ❌ | **Portar** |

**Steps a portar**:

```rust
// pipeliner-core/src/steps/

/// stash/unstash — preservar archivos entre stages
pub fn stash(name: &str, pattern: &str) -> Step {
    Step::from_closure(move |ctx| {
        let stash_dir = ctx.temp_dir().join("stash").join(name);
        fs::create_dir_all(&stash_dir)?;
        // Copiar archivos que matchean pattern a stash_dir
        for entry in glob(pattern)? {
            let path = entry?;
            let dest = stash_dir.join(path.file_name().unwrap());
            fs::copy(&path, &dest)?;
        }
        Ok(StepOutput::success(format!("Stashed {} files", pattern)))
    })
}

pub fn unstash(name: &str) -> Step {
    Step::from_closure(move |ctx| {
        let stash_dir = ctx.temp_dir().join("stash").join(name);
        if !stash_dir.exists() {
            return Err(StepError::StashNotFound(name.to_string()));
        }
        // Restaurar archivos del stash al workspace
        for entry in fs::read_dir(&stash_dir)? {
            let src = entry?.path();
            let dest = ctx.workspace().join(src.file_name().unwrap());
            fs::copy(&src, &dest)?;
        }
        Ok(StepOutput::success(format!("Unstashed from {}", name)))
    })
}

/// withCredentials — inyectar credenciales como env vars temporales
pub fn with_credentials(credentials: &[CredentialRef], inner: Step) -> Step {
    Step::from_closure(move |ctx| {
        // Inyectar credenciales en env
        let mut env_backup = Vec::new();
        for cred in credentials {
            let (key, value) = ctx.resolve_credential(cred)?;
            env_backup.push((key.clone(), std::env::var(&key).ok()));
            std::env::set_var(&key, &value);
        }

        // Ejecutar step interior
        let result = inner.execute(ctx);

        // Restaurar env
        for (key, prev) in env_backup {
            match prev {
                Some(v) => std::env::set_var(&key, v),
                None => std::env::remove_var(&key),
            }
        }

        result
    })
}

/// checkout — clonar SCM del proyecto
pub fn checkout(scm: &ScmConfig) -> Step {
    let url = scm.url.clone();
    let branch = scm.branch.clone();
    let shallow = scm.shallow_clone;

    Step::shell(if shallow {
        format!("git clone --depth 1 --branch {} {}", branch, url)
    } else {
        format!("git clone --branch {} {}", branch, url)
    })
}

/// input — pausar y esperar confirmación del usuario
pub fn input(message: &str) -> Step {
    let msg = message.to_string();
    Step::from_closure(move |_| {
        println!("⏸️  {}", msg);
        println!("Press Enter to continue...");
        std::io::stdin().read_line(&mut String::new())?;
        Ok(StepOutput::success("User confirmed"))
    })
}
```

**Complejidad**: EASY (1-2 semanas para todos)

---

### Feature 8: Logging Estructurado dentro del Pipeline

**Por qué importa**: GISS tiene `loggerTool` con niveles (debug, info, warn, error, fatal) que se controla por `logLevel`. Pipeliner usa `tracing` pero no lo expone como step del pipeline.

**GISS**:
```groovy
// vars/loggerTool.groovy
class loggerTool {
    String logLevel = "INFO"  // DEBUG | INFO | WARN | ERROR | FATAL

    def debug(msg) { if (shouldLog("DEBUG")) println "[DEBUG] ${msg}" }
    def info(msg)  { if (shouldLog("INFO"))  println "[INFO]  ${msg}" }
    def warn(msg)  { if (shouldLog("WARN"))  println "[WARN]  ${msg}" }
    def error(msg) { if (shouldLog("ERROR")) println "[ERROR] ${msg}" }
    def fatal(msg) { if (shouldLog("FATAL")) println "[FATAL] ${msg}" }
}
```

**Propuesta para Pipeliner**:

```rust
// pipeliner-core/src/step/log.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    Fatal = 4,
}

pub struct LogStep {
    level: LogLevel,
    message: String,
    min_level: LogLevel,  // Configurable via PipelineConfig
}

impl LogStep {
    pub fn debug(msg: impl Into<String>) -> Step {
        Step::log(LogLevel::Debug, msg.into())
    }

    pub fn info(msg: impl Into<String>) -> Step {
        Step::log(LogLevel::Info, msg.into())
    }

    pub fn warn(msg: impl Into<String>) -> Step {
        Step::log(LogLevel::Warn, msg.into())
    }

    pub fn error(msg: impl Into<String>) -> Step {
        Step::log(LogLevel::Error, msg.into())
    }

    pub fn fatal(msg: impl Into<String>) -> Step {
        Step::log(LogLevel::Fatal, msg.into())
    }
}

// Integración con tracing
fn emit_log(level: LogLevel, msg: &str, ctx: &PipelineContext) {
    match level {
        LogLevel::Debug => tracing::debug!(stage = %ctx.current_stage(), "{}", msg),
        LogLevel::Info  => tracing::info!(stage = %ctx.current_stage(), "{}", msg),
        LogLevel::Warn  => tracing::warn!(stage = %ctx.current_stage(), "{}", msg),
        LogLevel::Error => tracing::error!(stage = %ctx.current_stage(), "{}", msg),
        LogLevel::Fatal => {
            tracing::error!(stage = %ctx.current_stage(), "FATAL: {}", msg);
            // Emitir StageMarker de error también
            println!("{}", StageMarkerEmitter::error(&ctx.current_stage(), msg));
        }
    }
}
```

**Complejidad**: EASY (3 días)

---

## 3. Resumen: Priorización Final

| # | Feature | Impacto en Pipeline | Complejidad | Tiempo |
|---|---------|---------------------|-------------|--------|
| 1 | **Stage Markers** | Observabilidad del progreso | EASY | 1 semana |
| 2 | **Step Types adicionales** (stash, input, when, withCredentials...) | Funcionalidad inmediata | EASY | 1-2 semanas |
| 3 | **Logging estructurado** (loggerTool) | Debugging y control | EASY | 3 días |
| 4 | **PipelineConfig** (WorkerDefinition adaptado) | Modelo de configuración unificado | MEDIUM | 1-2 semanas |
| 5 | **Ciclo de vida del Runtime** | Ejecución robusta y predecible | MEDIUM | 2 semanas |
| 6 | **CDI/Step Registry** | Extensibilidad dinámica | MEDIUM | 2-3 semanas |
| 7 | **Sistema de Librerías** (GitSource, LocalSource) | Extensibilidad runtime máxima | HARD | 3-4 semanas |
| 8 | **Módulos de negocio** (maven, gradle, helm...) | Ecosistema de steps | EASY c/u | 8-12 semanas total |

**Total estimado**: ~16-24 semanas para todo, pero las features 1-3 (EASY) aportan valor inmediato en 3-4 semanas.

---

## 4. Estructura de Crates Resultante

```
pipeliner/
├── crates/
│   ├── pipeliner-core/            # Pipeline, Stage, Step, Agent, Parameters
│   │   └── steps/                 # Step types built-in: sh, echo, retry, timeout, dir
│   │       └── flow.rs            # when, errorHandler, is, stash, input
│   │       └── log.rs             # loggerTool (debug/info/warn/error/fatal)
│   │   └── config.rs              # PipelineConfig (WorkerDefinition adaptado)
│   │   └── context.rs             # PipelineContext + CDI + StepRegistry
│   │   └── markers.rs             # StageMarkerEmitter + StageMarkerParser
│   │
│   ├── pipeliner-executor/        # Execution engine
│   │   └── runtime.rs             # PipelineRuntime con lifecycle phases
│   │   └── local.rs               # LocalExecutor (mejorado con markers)
│   │
│   ├── pipeliner-library/         # NUEVO: Library loading system
│   │   └── retrievers/            # GitSource, LocalSource, LocalLib
│   │   └── loader.rs              # LibraryLoader con dedup y cache
│   │
│   ├── pipeliner-events/          # EventStore + EventBus (ya existe)
│   │   └── markers.rs             # StageMarker types (STARTED, COMPLETED...)
│   │
│   ├── pipeliner-macros/          # DSL macros (ya existe)
│   ├── pipeliner-infrastructure/  # Container runtimes (ya existe)
│   ├── pipeliner-worker/          # Worker pool (ya existe)
│   ├── pipeliner-api/             # API layer (ya existe)
│   ├── pipeliner-cli/             # CLI (ya existe)
│   │
│   ├── pipeliner-steps-core/      # NUEVO: config, errorHandler, when, is, workspace
│   ├── pipeliner-steps-git/       # NUEVO: gitTool (clone, tag, branch)
│   ├── pipeliner-steps-maven/     # NUEVO: maven build/test/publish
│   ├── pipeliner-steps-gradle/    # NUEVO: gradle build/test/publish
│   ├── pipeliner-steps-container/ # NUEVO: container build/promote
│   ├── pipeliner-steps-helm/      # NUEVO: helm build/test/deploy/promote
│   ├── pipeliner-steps-scanner/   # NUEVO: scanner execute
│   ├── pipeliner-steps-notify/    # NUEVO: notification emails
│   └── pipeliner-steps-http/      # NUEVO: restClient
```
