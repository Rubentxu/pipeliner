# Pipeliner

A Rust-based pipeline orchestration library built with Hexagonal Architecture, designed for building robust, maintainable, and testable pipeline systems.

## Overview

Pipeliner provides a flexible and extensible framework for defining and executing pipelines with stages, steps, and plugins. It follows clean architecture principles to ensure separation of concerns and maximum flexibility.

## Features

- **Pipeline Definition**: Create complex pipelines with stages and steps
- **Plugin System**: Extensible plugin architecture for adding custom functionality
- **Artifact Management**: Handle artifacts and intermediate outputs between stages
- **Concurrency Support**: Efficient execution with proper concurrency control
- **Error Handling**: Robust error handling and recovery mechanisms
- **Configuration**: Flexible configuration system for customizing pipeline behavior
- **CLI Interface**: Built-in command-line interface for managing pipelines

## Architecture

Pipeliner follows **Hexagonal Architecture** (also known as Ports and Adapters), organized into three main layers:

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
│  (Use Cases, Services, Orchestration)                       │
├─────────────────────────────────────────────────────────────┤
│                    Domain Layer                             │
│  (Entities, Business Rules, Interfaces)                     │
├─────────────────────────────────────────────────────────────┤
│                 Infrastructure Layer                        │
│  (External Systems, Database, HTTP Clients, CLI)            │
└─────────────────────────────────────────────────────────────┘
```

### Domain Layer

Contains the core business logic and entities:

- `Pipeline`: The main pipeline structure
- `Stage`: Individual stages in a pipeline
- `Step`: Executable units within stages
- `Agent`: Execution agents that run steps

### Application Layer

Implements use cases and orchestrates the domain:

- Pipeline execution orchestration
- Plugin management
- Artifact handling
- Error recovery

### Infrastructure Layer

Adapters for external systems and interfaces:

- Configuration management
- CLI interface
- Plugin executors
- Storage adapters

## Crate Structure

```
rustline/
├── src/
│   ├── cli/                 # Command-line interface
│   ├── config/              # Configuration management
│   ├── executor/            # Step and plugin execution
│   ├── pipeline/            # Core pipeline logic
│   └── lib.rs               # Library root
├── crates/
│   ├── pipeliner-cli/       # CLI application
│   └── pipeliner-core/      # Core library
├── tests/                   # Integration tests
└── docs/                    # Documentation
```

## Installation

### From Source

```bash
git clone https://github.com/pipeliner-org/pipeliner.git
cd pipeliner
cargo build --release
```

### From Crates.io

```bash
cargo install pipeliner
```

## Usage

### Basic Pipeline Definition

```rust
use pipeliner::prelude::*;

let pipeline = Pipeline::builder()
    .name("my-pipeline")
    .stage(Stage::builder("build")
        .step(Step::builder("compile")
            .command("cargo build")
            .build())
        .step(Step::builder("test")
            .command("cargo test")
            .build())
        .build())
    .stage(Stage::builder("deploy")
        .step(Step::builder("deploy")
            .command("kubectl apply -f k8s/")
            .build())
        .build())
    .build();
```

### Running a Pipeline

```rust
use pipeliner::executor::PipelineExecutor;

let executor = PipelineExecutor::new();
executor.execute(&pipeline).await?;
```

### Using Plugins

```rust
use pipeliner::pipeline::plugins::PluginRegistry;

let mut registry = PluginRegistry::default();
registry.register("docker", DockerPlugin::new());
registry.register("kubernetes", KubernetesPlugin::new());
```

## Configuration

Create a `pipeliner.yaml` configuration file:

```yaml
pipeline:
  name: my-pipeline
  stages:
    - name: build
      steps:
        - name: compile
          command: cargo build --release
        - name: test
          command: cargo test

execution:
  concurrency: 4
  retry:
    max_attempts: 3
    delay: 5s

artifacts:
  path: ./target/pipeliner
  retention: 7d
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please read [CONTRIBUTING.md](docs/CONTRIBUTING.md) for details on our code of conduct and development process.

## Development Setup

```bash
# Install dependencies
cargo fetch

# Run tests
cargo test

# Run lints
cargo clippy

# Build documentation
cargo doc --no-deps
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
