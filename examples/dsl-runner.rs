//! Pipeliner DSL Runner - Execute .dsl files directly
//!
//! Usage:
//!     cargo run --example dsl-runner -- examples/dsl/ci.dsl

use std::env;
use std::path::Path;

use pipeliner_core::dsl::load_pipeline_file;
use pipeliner_core::PipelineRunner;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Pipeliner DSL Runner");
        println!();
        println!("Usage:");
        println!("  {} <pipeline.dsl>", args[0]);
        println!();
        println!("Examples:");
        println!("  {} examples/dsl/ci.dsl", args[0]);
        return Ok(());
    }
    
    let path = Path::new(&args[1]);
    
    if !path.exists() {
        eprintln!("Error: File not found: {:?}", path);
        std::process::exit(1);
    }
    
    // Parse the DSL
    println!("\n╔══════════════════════════════════════════╗");
    println!("║  Loading pipeline from {:?}  ║", path.file_name().unwrap_or_default().to_str().unwrap_or(""));
    println!("╚══════════════════════════════════════════╝\n");
    
    let pipeline = load_pipeline_file(path)
        .map_err(|e| {
            eprintln!("❌ DSL Parse Error: {}", e);
            e
        })?;
    
    let name = pipeline.name().unwrap_or("Unnamed");
    println!("📋 Pipeline: {}", name);
    println!("📊 Stages: {}\n", pipeline.stages.len());
    
    // Print stages
    for (i, stage) in pipeline.stages.iter().enumerate() {
        if let Some(stage_name) = stage.name() {
            println!("  {}. {}", i + 1, stage_name);
        }
    }
    println!();
    
    // Execute
    println!("▶ Executing pipeline...\n");
    
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let mut runner = PipelineRunner::new();
        match runner.run_async(&pipeline).await {
            Ok(result) => {
                println!("\n📊 Results:");
                println!("   Success: {}", if result.success { "✅" } else { "❌" });
                println!("   Duration: {}ms", result.duration_ms);
                println!("   Stages: {}", result.stages_executed);
                println!("   Steps: {}", result.steps_executed);
                std::process::exit(if result.success { 0 } else { 1 });
            }
            Err(e) => {
                eprintln!("\n❌ Pipeline failed: {}", e);
                std::process::exit(1);
            }
        }
    })
}
