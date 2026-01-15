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

use rustline::PipelineExecutor;
use rustline::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Rustline Pipeline Demo ===\n");
    println!("This pipeline is defined using rustline's Jenkins-compatible DSL.\n");

    let pipeline = pipeline! {
        agent {
            any()
        }
        stages {
            stage!("Checkout", steps!(
                echo!("Cloning repository..."),
                sh!("echo 'Git clone complete'"),
                sh!("pwd")
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
            always(echo!("Cleanup complete"))
            success(echo!("Pipeline succeeded!"))
            failure(echo!("Pipeline failed!"))
        }
    };

    println!("Pipeline created with DSL macros:");
    println!("  Stages: {}", pipeline.stages.len());
    for (i, stage) in pipeline.stages.iter().enumerate() {
        println!(
            "    {}: {} ({} steps)",
            i + 1,
            stage.name,
            stage.steps.len()
        );
    }
    println!();

    println!("Executing pipeline with local executor...\n");

    let executor = LocalExecutor::new();
    match executor.execute(&pipeline) {
        Ok(result) => {
            println!("\n=== Pipeline Result ===");
            println!("Status: {:?}", result);
            println!("\n Pipeline executed successfully using DSL macros!");
        }
        Err(e) => {
            eprintln!("\n Pipeline failed: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
