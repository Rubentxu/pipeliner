# Pipeliner

<div align="center">

**A Rust-based pipeline orchestration library with Jenkins-compatible DSL**

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Rubentxu/pipeliner/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-121%20passing-green.svg)](#test-suite)
[![Crates](https://img.shields.io/badge/crates-8-blue.svg)](#crate-structure)

</div>

---

## Overview

Pipeliner is a **type-safe pipeline orchestration library** written in Rust that provides a Jenkins-compatible DSL (Domain Specific Language) for defining CI/CD pipelines. It combines the expressiveness of Jenkins Pipeline with Rust's safety guarantees and performance.

### Key Features

- **DSL-First Design**: Define pipelines with intuitive `pipeline!`, `stage!`, and `steps!` macros - no executor configuration needed
- **Zero-Config Execution**: Use `run!` or `run_sync!` macros to execute pipelines immediately
- **Type Safety**: All pipeline definitions are validated at compile time
- **Jenkins Compatibility**: Familiar syntax for Jenkins users, with Rust's safety guarantees
- **Multi-Backend Execution**: Run locally, in Docker, Kubernetes, or Podman seamlessly
- **Rust-Script Integration**: Execute pipelines directly with `rust-script` for maximum portability
- **Event Sourcing**: Built-in event store and event bus for observability
- **Extensible Plugin System**: Add custom steps, agents, and executors

---

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/Rubentxu/pipeliner.git
cd pipeliner

# Run tests to verify everything works
cd crates && cargo test --workspace
```

### Your First Pipeline

Create a file named `my_pipeline.rs`:

```rust
#!/usr/bin/env rust-script
//!
//! # My First Pipeliner Pipeline
//!
//! Run with: rust-script my_pipeline.rs
//!

use pipeliner_core::prelude::*;

fn main() {
    let pipeline = pipeline! {
        agent { any() }
        stages {
            stage!("Checkout", steps!(
                echo!("📦 Cloning repository..."),
                sh!("git clone https://github.com/myorg/myrepo.git")
            ))
            stage!("Build", steps!(
                echo!("🔨 Building project..."),
                sh!("cargo build --release")
            ))
            stage!("Test", steps!(
                echo!("🧪 Running tests..."),
                sh!("cargo test")
            ))
            stage!("Deploy", steps!(
                echo!("🚀 Deploying to production..."),
                sh!("kubectl apply -f k8s/")
            ))
        }
        post {
            success(echo!("✅ Pipeline succeeded!")),
            failure(echo!("❌ Pipeline failed!"))
        }
    };

    run!(pipeline);  // No executor needed - the macro handles everything!
}
```

Run it:

```bash
rust-script my_pipeline.rs
```

> **Note:** The `run!` macro automatically creates a `LocalExecutor`, runs your pipeline, and handles errors. For non-async contexts, use `run_sync!(pipeline)` instead.

---

## Pipeline DSL

Pipeliner's Domain Specific Language (DSL) lets you define pipelines with intuitive Rust macros. The DSL is **recommended** for most use cases - it's concise, expressive, and requires no executor configuration.

### Core Macros

| Macro | Description |
|-------|-------------|
| `pipeline!` | Define a complete pipeline with agents, stages, and post-actions |
| `stage!` | Define a stage with one or more steps |
| `steps!` | Group multiple steps together |
| `sh!` | Execute a shell command |
| `echo!` | Print a message |
| `retry!` | Retry a step N times |
| `timeout!` | Execute with a timeout |
| `dir!` | Execute steps in a directory |
| `run!` | Execute a pipeline (async) |
| `run_sync!` | Execute a pipeline (blocking) |

### Complete Pipeline Example

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
            echo!("📦 Building application..."),
            sh!("cargo build --release"),
            echo!("✅ Build complete!")
        ))
        stage!("Test", steps!(
            echo!("🧪 Running tests..."),
            sh!("cargo test --lib"),
            sh!("cargo test --doc")
        ))
        stage!("Deploy", steps!(
            echo!("🚀 Deploying to production..."),
            sh!("./deploy.sh ${VERSION}"),
            echo!("✅ Deployment complete!")
        ))
    }
    post {
        success(echo!("🎉 Pipeline succeeded!")),
        failure(echo!("❌ Pipeline failed!")),
        always(echo!("📊 Execution finished"))
    }
};

run!(pipeline);  // Execute with automatic error handling
```

### Step Types

```rust
use pipeliner_core::prelude::*;

let stage = stage!("Example Stage", steps!(
    // Print a message
    echo!("This is an informational message"),

    // Execute shell command
    sh!("cargo build --release"),

    // Retry failed step (3 attempts)
    retry!(3, sh!("flaky-command")),

    // Timeout after 5 minutes
    timeout!(300, sh!("long-running-task")),

    // Execute in directory
    dir!("./scripts", steps!(
        sh!("./setup.sh"),
        sh!("./run.sh")
    ))
));
```

### Post-Conditions

```rust
pipeline! {
    agent { any() }
    stages {
        stage!("Build", steps!(sh!("cargo build")))
    }
    post {
        always(echo!("Always runs - cleanup, notifications, etc.")),
        success(echo!("Runs when pipeline succeeds")),
        failure(echo!("Runs when pipeline fails")),
        unstable(echo!("Runs when pipeline is unstable"))
    }
}
```

### Parameters and Environment

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
            sh!("echo Deploying ${VERSION} to ${ENVIRONMENT}"),
            sh!("./deploy.sh ${VERSION} ${ENVIRONMENT}")
        ))
    }
};

run_sync!(pipeline);  // Blocking execution for scripts
```

---

## Pipeliner vs Jenkins Pipeline DSL

Pipeliner provides a Rust-native alternative to Jenkins Pipeline with significant advantages:

### Syntax Comparison

| Feature | Jenkins Pipeline | Pipeliner |
|---------|------------------|-----------|
| **Language** | Groovy-based DSL | Native Rust |
| **Type Safety** | Dynamic typing | Full compile-time type checking |
| **IDE Support** | Limited | Full Rust IDE support (IntelliJ, VSCode) |
| **Testing** | Scripted, limited | TDD/BDD with native Rust testing |
| **Execution** | JVM only | Any Rust runtime (local, Docker, K8s) |
| **Dependencies** | Jenkins + plugins | No external dependencies |

### Pipeline Definition

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

### Stages and Steps

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
        success { echo 'Deployed!' }
        failure { echo 'Failed!' }
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
        success(echo!("Deployed!")),
        failure(echo!("Failed!"))
    }
};
```

### Key Advantages of Pipeliner

| Aspect | Benefit |
|--------|---------|
| **Type Safety** | Catch errors at compile time, not runtime |
| **Performance** | Native Rust execution, no JVM overhead |
| **Testing** | Write unit/integration tests with `cargo test` |
| **Portability** | Run pipelines anywhere Rust runs |
| **Tooling** | Use Rust's ecosystem (cargo, clippy, rust-analyzer) |
| **Safety** | Memory safety guarantees, no null pointer exceptions |
| **Concurrency** | Fearless async/await concurrency |
| **Versioning** | Semantic versioning of pipeline definitions |

### Migration from Jenkins

Pipeliner is designed to be familiar to Jenkins users while providing Rust benefits:

```rust
// Jenkins: agent any
AgentType::any()

// Jenkins: agent { docker 'rust:latest' }
AgentType::docker("rust:latest")

// Jenkins: sh 'command'
Step::shell("command")

// Jenkins: echo 'message'
Step::echo("message")

// Jenkins: timeout(time: 10, unit: 'MINUTES') { ... }
Step::timeout(std::time::Duration::from_secs(600), inner_step)

// Jenkins: retry(3) { ... }
Step::retry(3, inner_step)

// Jenkins: dir('path') { ... }
Step::dir(PathBuf::from("path"), inner_step)
```

---

## DSL Reference

### Pipeline Definition

```rust
use rustline::prelude::*;

let pipeline = pipeline! {
    agent { any() },  // or docker("rust:latest"), kubernetes("default"), etc.
    environment {
       ("DEBUG", "1"),
        ("ENV", "production")
    }
    parameters {
        string("VERSION", "1.0.0"),
        boolean("DEPLOY_ENABLED", true)
    }
    stages {
        stage!("Build", steps!(
            sh!("cargo build --release"),
            sh!("cargo test --lib")
        ))
        stage!("Deploy", steps!(
            echo!("Deploying version ${VERSION}"),
            sh!("./deploy.sh ${VERSION}")
        ))
    }
};
```

### Stages and Steps

```rust
stage!("StageName", steps!(
    echo!("A message step"),
    sh!("shell command to execute"),
    dir!("./path", steps!(
        sh!("command in directory")
    )),
    retry!(3, sh!("fallible command")),
    timeout!(30, sh!("long running command"))
))
```

### Post-Conditions

```rust
post {
    always(echo!("Always runs")),
    success(echo!("Runs on success")),
    failure(echo!("Runs on failure")),
    unstable(echo!("Runs when unstable"))
}
```

---

## Architecture

Pipeliner follows **Hexagonal Architecture** (Ports & Adapters) with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Application Layer                            │
│   PipelineExecutor │ PluginManager │ ExecutionStrategy              │
├─────────────────────────────────────────────────────────────────────┤
│                          Domain Layer                                │
│   Pipeline │ Stage │ Step │ Agent │ Parameters │ Environment        │
├─────────────────────────────────────────────────────────────────────┤
│                      Infrastructure Layer                            │
│   DockerExecutor │ K8sExecutor │ PodmanExecutor │ CLI │ REST API    │
└─────────────────────────────────────────────────────────────────────┘
```

### Domain Layer

The core business logic entities:

- **Pipeline**: Main pipeline structure with stages, parameters, and environment
- **Stage**: Individual execution stages with conditional execution
- **Step**: Executable units (shell, echo, retry, timeout, dir)
- **Agent**: Execution targets (any, docker, kubernetes, podman)
- **Parameters**: Input parameters with type validation

### Application Layer

Use cases and orchestration:

- **PipelineExecutor**: Executes pipelines with proper error handling
- **PluginRegistry**: Manages custom plugins and extensions
- **ExecutionStrategy**: Parallel, sequential, and matrix execution

### Infrastructure Layer

External adapters:

- **DockerExecutor**: Run steps in Docker containers
- **K8sExecutor**: Execute in Kubernetes pods
- **PodmanExecutor**: Native Podman support
- **gRPC/REST API**: Programmatic access
- **CLI**: Command-line interface

---

## Crate Structure

```
pipeliner/
├── crates/
│   ├── pipeliner-core/        # Pipeline DSL types and validation
│   ├── pipeliner-executor/    # Pipeline execution engine
│   ├── pipeliner-infrastructure/ # Docker, Podman, K8s providers
│   ├── pipeliner-worker/      # Job scheduling and worker pool
│   ├── pipeliner-events/      # Event sourcing infrastructure
│   ├── pipeliner-api/         # gRPC and REST API layer
│   ├── pipeliner-cli/         # Command-line interface
│   └── pipeliner-macros/      # Procedural macros for DSL
├── docs/                      # Documentation (Spanish & English)
│   ├── USER_MANUAL.md
│   ├── architecture.md
│   ├── jenkins-sh-compatibility.md
│   ├── rust-script-integration.md
│   └── tdd-strategy.md
├── examples/                  # Runnable examples
│   ├── mi_pipeline.rs         # Spanish example with rust-script
│   ├── pipeline_example.rs    # English DSL example
│   ├── docker_test.rs         # Docker integration
│   └── podman_test.rs         # Podman integration
└── tests/                     # Integration tests
```

---

## Test Suite

All 121 unit tests pass across the workspace:

```bash
cd crates && cargo test --workspace
```

| Crate | Tests | Status |
|-------|-------|--------|
| pipeliner-core | 43 | ✅ |
| pipeliner-executor | 22 | ✅ |
| pipeliner-infrastructure | 5 | ✅ |
| pipeliner-worker | 19 | ✅ |
| pipeliner-events | 15 | ✅ |
| pipeliner-api | 10 | ✅ |
| pipeliner-cli | 7 | ✅ |
| **Total** | **121** | **✅ All passing** |

---

## Configuration

Create a `pipeliner.yaml` for advanced configuration:

```yaml
pipeline:
  name: my-ci-pipeline
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

## Appendix: Programmatic API

While the **DSL is recommended** for most use cases, Pipeliner also provides a programmatic API for advanced use cases requiring fine-grained control.

### Using LocalExecutor Directly

For scenarios requiring custom execution handling:

```rust
use pipeliner_executor::LocalExecutor;
use pipeliner_core::{Pipeline, Stage, Step, AgentType};

#[tokio::main]
async fn main() {
    let pipeline = Pipeline::builder()
        .name("My Pipeline")
        .with_agent(AgentType::any())
        .with_stage(
            Stage::new("Build")
                .with_step(Step::echo("Starting build..."))
                .with_step(Step::shell("cargo build").with_retry(3))
        )
        .build();

    let executor = LocalExecutor::new();
    let results = executor.execute(&pipeline).await;

    for result in &results {
        println!("[{}] {} - {}", result.stage, result.success, result.output);
    }

    // Check if all steps succeeded
    let all_success = results.iter().all(|r| r.success);
    if all_success {
        println!("✅ Pipeline completed successfully!");
    }
}
```

### Builder Pattern API

All core types support builder methods for programmatic construction:

```rust
use pipeliner_core::{Pipeline, Stage, Step, AgentType};

let pipeline = Pipeline::builder()
    .name("My Pipeline")
    .description("A test pipeline")
    .with_agent(AgentType::docker("rust:1.92"))
    .with_stage(
        Stage::new("Build")
            .with_agent(AgentType::any()) // Override stage agent
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

### When to Use Programmatic API

- Custom executor implementations
- Dynamic pipeline generation based on configuration
- Integration with existing async frameworks
- Fine-grained control over execution results

For most pipelines, the DSL with `run!` or `run_sync!` macros is simpler and recommended.

---

## Contributing

Contributions are welcome! Please read our contributing guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes following [Conventional Commits](https://www.conventionalcommits.org/)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Setup

```bash
# Install dependencies
cd crates && cargo fetch

# Run all tests
cargo test --workspace

# Run lints
cargo clippy --workspace

# Build documentation
cargo doc --no-deps
```

---

## License

Licensed under **MIT OR Apache-2.0**. See the [LICENSE](LICENSE) file for details.

---

<div align="center">

**Built with ❤️ using Rust**

[Repository](https://github.com/Rubentxu/pipeliner) · [Issues](https://github.com/Rubentxu/pipeliner/issues)

</div>
