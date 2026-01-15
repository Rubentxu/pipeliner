#!/usr/bin/env rust-script
//! ```cargo
//! [package]
//! name = "rustline-pipeline"
//! version = "0.1.0"
//! edition = "2024"
//!
//! [dependencies]
//! rustline = { path = "/home/rubentxu/Proyectos/rust/rustline" }
//! ```

use rustline::pipeline::{AgentType, DockerConfig, Pipeline, Stage, Step};
use rustline::PipelineExecutor;
use rustline::infrastructure::docker::DockerExecutor;

fn main() {
    println!("=== Rustline Pipeline Demo ===\n");

    let pipeline = Pipeline::builder()
        .name("demo-pipeline")
        .agent(AgentType::Docker(DockerConfig {
            image: "rust:latest".to_string(),
            registry: None,
            args: vec![],
            environment: std::collections::HashMap::new(),
        }))
        .environment(
            rustline::pipeline::Environment::new()
                .set("PROJECT_NAME", "my-awesome-project")
                .set("VERSION", "1.0.0")
        )
        .stages(vec![
            Stage::new(
                "Checkout",
                vec![
                    Step::echo("Checking out repository..."),
                    Step::shell("echo 'Cloning git repository...'"),
                    Step::shell("pwd"),
                    Step::shell("echo 'PROJECT: ${PROJECT_NAME}'"),
                    Step::shell("echo 'VERSION: ${VERSION}'"),
                ],
            ),
            Stage::new(
                "Build",
                vec![
                    Step::echo("Building the project..."),
                    Step::shell("echo 'Compiling Rust code...'"),
                    Step::shell("cargo --version 2>/dev/null || echo 'Cargo not in container'"),
                    Step::shell("echo 'Build completed successfully!'"),
                ],
            ),
            Stage::new(
                "Test",
                vec![
                    Step::echo("Running tests..."),
                    Step::shell("echo 'Running unit tests...'"),
                    Step::shell("echo 'All tests passed!'"),
                    Step::shell("echo 'Test coverage: 85%'"),
                ],
            ),
            Stage::new(
                "Deploy",
                vec![
                    Step::echo("Deploying to production..."),
                    Step::shell("echo 'Deploying ${PROJECT_NAME} v${VERSION}'"),
                    Step::shell("echo 'Deployment complete!'"),
                ],
            ),
        ])
        .build_unchecked();

    println!("Pipeline definition created:");
    println!("  Name: {}", pipeline.name.clone().unwrap_or_default());
    println!("  Stages: {}", pipeline.stages.len());
    for (i, stage) in pipeline.stages.iter().enumerate() {
        println!("    {}: {} ({} steps)", i + 1, stage.name, stage.steps.len());
    }
    println!();

    println!("Executing with Docker executor...\n");

    let executor = DockerExecutor::new();
    match executor.execute(&pipeline) {
        Ok(result) => {
            println!("\n=== Pipeline Result ===");
            println!("Status: {:?}", result);
            println!("\n Pipeline executed successfully!");
        }
        Err(e) => {
            println!("\n❌ Pipeline failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
    println!();

    println!("Executing with Docker executor...\n");

    let executor = DockerExecutor::new();
    match executor.execute(&pipeline) {
        Ok(result) => {
            println!("\n=== Pipeline Result ===");
            println!("Status: {:?}", result);
            println!("\n Pipeline executed successfully!");
        }
        Err(e) => {
            println!("\n❌ Pipeline failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
