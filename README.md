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

- **Jenkins-Compatible DSL**: Define pipelines using familiar `pipeline!`, `stage!`, and `steps!` macros
- **Type Safety**: All pipeline definitions are validated at compile time
- **Multi-Backend Execution**: Run pipelines locally, in Docker, Kubernetes, or Podman
- **Hexagonal Architecture**: Clean separation between domain, application, and infrastructure
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

use rustline::LocalExecutor;
use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = pipeline! {
        agent {
            docker("rust:latest")
        }
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

    let executor = LocalExecutor::new();
    executor.execute(&pipeline)?;
    Ok(())
}
```

Run it:

```bash
rust-script my_pipeline.rs
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
