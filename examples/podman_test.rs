#!/usr/bin/env rust-script
//!
//! # Rustline Podman Container Test
//!
//! Este ejemplo prueba la ejecución de pipelines usando Podman en lugar de Docker.
//!
//! ## Uso
//!
//! ```bash
//! rust-script examples/podman_test.rs
//! ```
//!
//! ## Dependencias
//!
//! ```cargo
//! [package]
//! name = "rustline-podman-test"
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

    println!("=== Rustline Podman Container Test ===\n");

    // Check if Podman is available
    let podman_version = std::process::Command::new("podman")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "not found".to_string());

    println!("Podman version: {}", podman_version.trim());

    if podman_version.trim() == "not found" {
        println!("\n⚠️  Podman no está instalado.");
        println!("   Instalar con: sudo apt install podman");
        println!("   O usar Docker: rust-script examples/docker_test.rs");
        return;
    }

    // Create executor with Podman runtime
    let executor = ContainerExecutor::with_podman().with_default_image("alpine:latest");

    println!("\n1. Health Check:");
    let health = executor.health_check();
    println!("   Status: {:?}", health);
    println!();

    println!("2. Capabilities:");
    let caps = executor.capabilities();
    println!("   can_execute_shell: {}", caps.can_execute_shell);
    println!("   can_run_docker: {}", caps.can_run_docker);
    println!();

    println!("3. Creating pipeline with Podman agent...");
    let pipeline = Pipeline::builder()
        .name("podman-test-pipeline")
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
                    Step::shell("echo '=== Building with Podman ==='"),
                    Step::shell("echo 'Container OS:' && cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2"),
                    Step::shell("echo 'Working directory:' && pwd"),
                    Step::shell("echo 'User:' && whoami"),
                    Step::shell("echo 'Podman version:' && podman --version"),
                ],
            ),
            Stage::new(
                "Test",
                vec![
                    Step::shell("echo '=== Testing with Podman ==='"),
                    Step::shell("echo '2 + 2 = 4'"),
                    Step::shell("uname -a"),
                ],
            ),
        ])
        .build_unchecked();

    println!("4. Executing pipeline in Podman...");
    match executor.execute(&pipeline) {
        Ok(result) => {
            println!("\n5. Pipeline result: {:?}", result);
            println!("\n Podman test completed successfully!");
        }
        Err(e) => {
            println!("\n Pipeline failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
