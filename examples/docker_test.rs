#!/usr/bin/env rust-script
//!
//! # Rustline Container Test (Docker/Podman)
//!
//! Este ejemplo prueba la ejecución de pipelines en contenedores Docker o Podman.
//!
//! ## Uso
//!
//! ```bash
//! # Con Docker (por defecto)
//! rust-script examples/docker_test.rs
//!
//! # Con Podman
//! rust-script examples/podman_test.rs
//! ```
//!
//! ## Dependencias
//!
//! ```cargo
//! [package]
//! name = "rustline-container-test"
//! version = "0.1.0"
//! edition = "2024"
//!
//! [dependencies]
//! rustline = { path = "/home/rubentxu/Proyectos/rust/rustline" }
//! tracing-subscriber = "0.3"
//! ```

use rustline::PipelineExecutor;
use rustline::container::{ContainerExecutor, ContainerRuntime};
use rustline::pipeline::{AgentType, DockerConfig, Pipeline, Stage, Step};

fn main() {
    tracing_subscriber::fmt::init();

    println!("=== Rustline Container Test ===\n");

    // Detect available container runtime
    let runtime = if std::process::Command::new("podman")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        println!("Detected: Podman");
        ContainerRuntime::Podman
    } else if std::process::Command::new("docker")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        println!("Detected: Docker");
        ContainerRuntime::Docker
    } else {
        println!(" No se detecto ningun runtime de contenedores (Docker/Podman)");
        println!("   Instalar Docker: https://docs.docker.com/get-docker/");
        println!("   Instalar Podman: https://podman.io/getting-started/");
        std::process::exit(1);
    };

    let executor = ContainerExecutor::new()
        .with_runtime(runtime)
        .with_default_image("alpine:latest");

    println!("\n1. Health Check:");
    let health = executor.health_check();
    println!("   Status: {:?}", health);
    println!();

    println!("2. Capabilities:");
    let caps = executor.capabilities();
    println!("   can_execute_shell: {}", caps.can_execute_shell);
    println!("   can_run_containers: {}", caps.can_run_docker);
    println!();

    println!("3. Creating pipeline with container agent...");
    let pipeline = Pipeline::builder()
        .name("container-test-pipeline")
        .agent(AgentType::Docker(DockerConfig {
            image: "alpine:latest".to_string(),
            registry: None,
            args: vec![],
            environment: std::collections::HashMap::new(),
        }))
        .stages(vec![
            Stage::new(
                "Build",
                vec![
                    Step::shell("echo '=== Building in Container ==='"),
                    Step::shell("echo 'Container OS:' && cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2"),
                    Step::shell("echo 'Working directory:' && pwd"),
                    Step::shell("echo 'User:' && whoami"),
                ],
            ),
            Stage::new(
                "Test",
                vec![
                    Step::shell("echo '=== Testing in Container ==='"),
                    Step::shell("echo '2 + 2 = 4'"),
                    Step::shell("uname -a"),
                ],
            ),
        ])
        .build_unchecked();

    println!("4. Executing pipeline in container...");
    match executor.execute(&pipeline) {
        Ok(result) => {
            println!("\n5. Pipeline result: {:?}", result);
            println!("\n Container test completed successfully!");
        }
        Err(e) => {
            println!("\n Pipeline failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
