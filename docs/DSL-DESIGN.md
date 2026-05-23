# Pipeliner DSL - Diseño para cercanía a Jenkins Pipeline

## Estado Actual

```rust
pipeline! {
    name = "CI"
    stages {
        stage!("Build") {
            steps {
                sh!("cargo build")
                echo!("Done!")
            }
        }
    }
}
```

## Objetivo: Jenkins Pipeline DSL 1:1

```groovy
// Jenkins Groovy
pipeline {
    agent any
    environment {
        FOO = 'bar'
    }
    options {
        timeout(time: 1, unit: 'HOURS')
        skipDefaultCheckout()
    }
    stages {
        stage('Build') {
            steps {
                sh 'make build'
                echo 'Done!'
            }
        }
        stage('Test') {
            when {
                expression { env.BRANCH_NAME != 'main' }
            }
            steps {
                timeout(time: 30, unit: 'MINUTES') {
                    retry(3) {
                        sh 'make test'
                    }
                }
            }
        }
        stage('Deploy') {
            steps {
                script {
                    def foo = "bar"
                }
            }
        }
    }
    post {
        always {
            echo 'Cleanup'
        }
        success {
            echo 'Deploy successful!'
        }
        failure {
            echo 'Deploy failed!'
        }
    }
}
```

## Plan de Implementación

### 1. Estructura del Pipeline

```rust
pipeline! {
    name = "CI"
    
    // Agent global
    agent = any              // o agent = docker("rust:latest")
    
    // Environment global
    environment {
        FOO = "bar"
        VERSION = "1.0"
    }
    
    // Options globales
    options {
        timeout = 60          // minutos
        retry = 3
    }
    
    // Triggers
    triggers {
        cron = "H H * * *"
        poll_scm = "H/5 * * * *"
    }
    
    stages {
        // ...
    }
    
    // Post condiciones
    post {
        always { /* steps */ }
        success { /* steps */ }
        failure { /* steps */ }
    }
}
```

### 2. Steps Mejorados

```rust
// Shell commands
sh!("make build")
sh("make test")           // sin ! también válido

// Echo
echo!("Build complete!")
log!(level = "INFO", "message")

// Retry
retry!(3) {
    sh!("make test")
}

// Timeout
timeout!(minutes = 30) {
    sh!("make test")
}

// Dir
dir!("/tmp") {
    sh!("ls -la")
}

// Input
input!(message = "Continue?")

// Script
script! {
    let x = 42;
    println!("x = {}", x);
}

// Error handling
try! {
    sh!("make build")
} catch {
    echo!("Build failed!")
}

// Stash/Unstash
stash!(name = "artifacts", includes = ["target/**"])
unstash!("artifacts")

// Archive
archive!(artifacts = ["dist/**"], fingerprint = true)

// Checkout
checkout!(scm = "git")

// With credentials
with_credentials!(credential_id = "my-secret") {
    sh!("deploy.sh")
}
```

### 3. Steps Anidados (Jenkins style)

```rust
stage!("Test") {
    steps {
        timeout!(minutes = 30) {
            retry!(3) {
                sh!("cargo test")
            }
        }
    }
}
```

### 4. When Conditions

```rust
stage!("Deploy") {
    when {
        expression = "BRANCH_NAME == 'main'"
        not = { expression = "SKIP_DEPLOY == 'true'" }
        any_of = {
            expression = "ENV == 'prod'"
            expression = "ENV == 'staging'"
        }
    }
    steps {
        sh!("deploy.sh")
    }
}
```

### 5. Post Conditions

```rust
post {
    always {
        echo!("Always runs")
        archive!(artifacts = ["logs/**"])
    }
    success {
        echo!("Pipeline succeeded!")
        input!(message = "Deploy to production?")
    }
    failure {
        echo!("Pipeline failed!")
        slack!(channel = "#alerts", message = "Build failed!")
    }
    unstable {
        echo!("Pipeline unstable")
    }
    changed {
        echo!("Pipeline state changed")
    }
}
```

### 6. Integración con Librerías (Shared Libraries)

```rust
// Declarar librerías al inicio
#library("git@github.com:org/shared-library.git", branch = "main")
#library("shared-lib@v1.0")

pipeline! {
    // Usar steps de librería
    stages {
        stage!("Build") {
            steps {
                // Step custom de librería
                sonar!(project = "my-project", url = "https://sonar.example.com")
                // o con sintaxis Jenkins
                "sonar" {
                    args {
                        project = "my-project"
                        url = "https://sonar.example.com"
                    }
                }
            }
        }
    }
}
```

## Diseño del Parser

### TokenStream Estructura

```
pipeline!
├── name = "CI"
├── agent = any | docker("rust:latest") | kubernetes({...})
├── environment { KEY = "value" }
├── options { timeout = 60 | retry = 3 | ... }
├── triggers { cron = "..." | poll_scm = "..." }
├── stages { ... }
└── post { always { ... } success { ... } failure { ... } }
```

### Tipos de Parser Necesarios

```rust
enum PipelineItem {
    Name(String),
    Agent(AgentExpr),
    Environment(Vec<(String, String)>),
    Options(Vec<OptionExpr>),
    Triggers(Vec<TriggerExpr>),
    Stages(Vec<StageDef>),
    Post(PostCondition),
}

enum StageContent {
    Steps(Vec<StepExpr>),
    Parallel(Vec<StageDef>),
    Matrix(MatrixExpr),
}

enum StepExpr {
    Shell(String),
    Echo(String),
    Log { level: String, message: String },
    Retry { count: usize, body: Vec<StepExpr> },
    Timeout { minutes: usize, body: Vec<StepExpr> },
    Dir { path: String, body: Vec<StepExpr> },
    Input { message: String },
    Script { body: String },
    TryCatch { try_block: Vec<StepExpr>, catch_block: Vec<StepExpr> },
    Stash { name: String, includes: Vec<String> },
    Unstash { name: String },
    Archive { artifacts: Vec<String>, fingerprint: bool },
    Checkout,
    WithCredentials { credential_id: String, body: Vec<StepExpr> },
    Library { name: String, args: Vec<(String, String)> }, // Custom steps
}
```

## Implementación por Fases

### Fase 1: Estructura Base
- [ ] `agent`, `environment`, `options` top-level
- [ ] `stages { }` con `stage!`
- [ ] `steps { }` con `sh!`, `echo!`
- [ ] `parallel { }` dentro de `stage!`

### Fase 2: Steps Comunes
- [ ] `retry!`, `timeout!`, `dir!`
- [ ] `stash!`, `unstash!`
- [ ] `input!`, `script!`
- [ ] Anidación de steps

### Fase 3: Condiciones
- [ ] `when { }` con `expression`
- [ ] `post { }` con `always`, `success`, `failure`

### Fase 4: Librerías
- [ ] `#library()` directive
- [ ] Registro de steps de librería
- [ ] Llamada a steps custom: `step_name!(args)`

### Fase 5: Polimento
- [ ] `agent { }` por stage
- [ ] `environment { }` por stage
- [ ] `options { }` por stage
- [ ] `triggers { }`
- [ ] Documentación completa
