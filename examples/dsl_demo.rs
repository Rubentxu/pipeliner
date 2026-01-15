#!/usr/bin/env rust-script
//! ```cargo
//! [package]
//! name = "rustline-dsl-demo"
//! version = "0.1.0"
//! edition = "2024"
//!
//! [dependencies]
//! rustline = { path = "/home/rubentxu/Proyectos/rust/rustline" }
//! ```

use rustline::pipeline;
use rustline::pipeline::{AgentType, DockerConfig, Pipeline, Stage, Step};
use rustline::PipelineExecutor;
use rustline::infrastructure::docker::DockerExecutor;

fn main() {
    println!("=== Rustline DSL Demo ===\n");
    println!("This pipeline demonstrates rustline's Jenkins-compatible DSL macros.\n");

    let my_pipeline = pipeline! {
        agent {
            docker("rust:latest")
        }
        stages {
            stage!("Checkout", steps!(
                echo!("Cloning repository..."),
                sh!("echo 'Git clone complete'"),
                sh!("pwd && ls -la")
            ))
            stage!("Build", steps!(
                echo!("Building the project..."),
                sh!("echo 'Running cargo build...'"),
                sh!("cargo --version"),
                sh!("echo 'Build completed!'")
            ))
            stage!("Test", steps!(
                echo!("Running tests..."),
                sh!("echo 'All tests passed!'")
            ))
            stage!("Deploy", steps!(
                echo!("Deploying to production..."),
                sh!("echo 'Deployment complete!'")
            ))
        }
        post {
            always(sh!("echo 'Cleanup complete'"))
            success(sh!("echo 'Pipeline succeeded!'"))
            failure(sh!("echo 'Pipeline failed!'"))
        }
    };

    println!("Pipeline created with DSL macros:");
    println!("  Name: {:?}", my_pipeline.name);
    println!("  Agent: {:?}", match &my_pipeline.agent {
        rustline::pipeline::AgentType::Docker(d) => format!("Docker: {}", d.image),
        _ => format!("{:?}", my_pipeline.agent)
    });
    println!("  Stages: {}", my_pipeline.stages.len());
    for (i, stage) in my_pipeline.stages.iter().enumerate() {
        println!("    {}: {} ({} steps)", i + 1, stage.name, stage.steps.len());
    }
    println!();

    println!("Executing pipeline with Docker executor...\n");

    let executor = DockerExecutor::new();
    match executor.execute(&my_pipeline) {
        Ok(result) => {
            println!("\n=== Pipeline Result ===");
            println!("Status: {:?}", result);
            println!("\n Pipeline executed successfully using DSL macros!");
        }
        Err(e) => {
            println!("\n Pipeline failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
        environment {
            PROJECT_NAME = "my-awesome-app"
            VERSION = "1.0.0"
            BUILD_NUMBER = "42"
        }
        options {
            timeout(minutes: 10)
            retry(count: 3)
        }
        stages {
            stage!("Checkout", steps!(
                echo!("Cloning repository..."),
                sh!("echo 'Git clone complete'"),
                sh!("pwd && ls -la"),
                sh!("echo 'PROJECT: ${PROJECT_NAME}'"),
                sh!("echo 'VERSION: ${VERSION}'"),
                sh!("echo 'BUILD: ${BUILD_NUMBER}'")
            ))
            stage!("Build", steps!(
                echo!("Building the project..."),
                sh!("echo 'Running cargo build...'"),
                sh!("cargo --version"),
                sh!("echo 'Build completed!'")
            ))
            stage!("Test", steps!(
                echo!("Running tests..."),
                sh!("echo 'Running unit tests...'"),
                sh!("echo 'Test coverage: 85%'"),
                sh!("echo 'All tests passed!'")
            ))
            stage!("Deploy", steps!(
                echo!("Deploying to production..."),
                sh!("echo 'Deploying ${PROJECT_NAME} v${VERSION}'"),
                sh!("echo 'Deployment complete!'")
            ))
        }
        post {
            always(sh!("echo 'Cleanup complete'"))
            success(sh!("echo 'Pipeline succeeded!'"))
            failure(sh!("echo 'Pipeline failed!'"))
        }
    };

    println!("Pipeline created with DSL macros:");
    println!("  Name: {:?}", my_pipeline.name);
    println!(
        "  Agent: {:?}",
        match &my_pipeline.agent {
            rustline::pipeline::AgentType::Docker(d) => format!("Docker: {}", d.image),
            _ => format!("{:?}", my_pipeline.agent),
        }
    );
    println!("  Stages: {}", my_pipeline.stages.len());
    println!(
        "  Environment variables: {}",
        my_pipeline.environment.vars.len()
    );
    println!();

    println!("Executing pipeline with Docker executor...\n");

    let executor = DockerExecutor::new();
    match executor.execute(&my_pipeline) {
        Ok(result) => {
            println!("\n=== Pipeline Result ===");
            println!("Status: {:?}", result);
            println!("\n Pipeline executed successfully using DSL macros!");
        }
        Err(e) => {
            println!("\n❌ Pipeline failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
