//! Pipeline CI: Build + Test + Verify
//!
//! Ejecuta un pipeline CI/CD completo usando el DSL de Pipeliner.
//!
//! Run: cargo run --example ci_build

use pipeliner_core::{Pipeline, PipelineRunner, Stage, Step};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CI Build Pipeline ===\n");

    let pl = Pipeline::new()
        .with_name("CI Build Pipeline")
        .with_stage(
            Stage::new("Prerequisites")
                .with_step(Step::shell("which git"))
                .with_step(Step::shell("which cargo"))
                .with_step(Step::echo("Prerequisites verified!")),
        )
        .with_stage(
            Stage::new("Build")
                .with_step(Step::shell("cargo build --release"))
                .with_step(Step::echo("Build completed!")),
        )
        .with_stage(
            Stage::new("Test")
                .with_step(Step::shell("cargo test --all"))
                .with_step(Step::echo("All tests passed!")),
        )
        .with_stage(
            Stage::new("Verify")
                .with_step(Step::shell("ls -lh target/release/pipeliner 2>/dev/null || ls -lh target/debug/pipeliner"))
                .with_step(Step::echo("Binary verified!")),
        );

    println!("Pipeline: {}", pl.name().unwrap_or_default());
    println!("Stages: {}", pl.stages.len());
    for (i, stage) in pl.stages.iter().enumerate() {
        println!("  {}. {} ({} steps)", i + 1, stage.name, stage.steps.len());
    }
    
    // Run the pipeline
    let mut runner = PipelineRunner::new();
    println!("\n--- Running Pipeline ---");
    let results = runner.run_async(&pl).await?;
    
    println!("\n--- Results ---");
    println!("Success: {}", results.success);
    println!("Duration: {}ms", results.duration_ms);
    println!("Stages executed: {}", results.stages_executed);
    println!("Steps executed: {}", results.steps_executed);
    
    if results.success {
        println!("\n✅ Pipeline completed successfully!");
    } else {
        println!("\n❌ Pipeline failed!");
        if let Some(err) = results.error {
            println!("Error: {}", err);
        }
    }
    
    Ok(())
}
