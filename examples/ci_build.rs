//! Pipeline CI: Build + Test + Verify
//!
//! Ejecuta un pipeline CI/CD completo usando el DSL declarativo de Pipeliner.
//!
//! Run: cargo run --example ci_build
//!
//! Este ejemplo usa la sintaxis declarativa estilo Jenkinsfile/Groovy:
//!
//! ```rust,ignore
//! pipeline! {
//!     name = "CI Pipeline"
//!     stages {
//!         stage!("Build") {
//!             steps {
//!                 sh!("cargo build")
//!             }
//!         }
//!     }
//! }
//! ```

use pipeliner_core::{Pipeline, Stage, Step, PipelineRunner};
use pipeliner_macros::pipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CI Build Pipeline ===\n");

    // DSL declarativo estilo Jenkinsfile
    let pl = pipeline! {
        name = "CI Build Pipeline"
        
        stages {
            stage!("Prerequisites") {
                steps {
                    sh!("which git")
                    sh!("which cargo")
                    echo!("Prerequisites verified!")
                }
            }
            
            stage!("Build") {
                steps {
                    sh!("cargo build --release")
                    echo!("Build completed!")
                }
            }
            
            stage!("Test") {
                steps {
                    sh!("cargo test --all")
                    echo!("All tests passed!")
                }
            }
            
            stage!("Verify") {
                steps {
                    sh!("ls -lh target/release/pipeliner 2>/dev/null || ls -lh target/debug/pipeliner")
                    echo!("Binary verified!")
                }
            }
        }
    };

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
