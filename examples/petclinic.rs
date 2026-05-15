//! Pipeline PetClinic: Clone + Build + Test + Report
//!
//! Pipeline que clona un repositorio Java, lo compila y ejecuta tests.
//!
//! Run: cargo run --example petclinic
//!
//! Requiere: git, java, maven

use pipeliner_core::{Pipeline, PipelineRunner, Stage, Step};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PetClinic CI Pipeline ===\n");

    let pl = Pipeline::new()
        .with_name("PetClinic CI")
        .with_stage(
            Stage::new("Prerequisites")
                .with_step(Step::shell("which git && git --version"))
                .with_step(Step::shell("which java && java -version"))
                .with_step(Step::shell("which mvn && mvn --version"))
                .with_step(Step::echo("Prerequisites satisfied!")),
        )
        .with_stage(
            Stage::new("Clone")
                .with_step(Step::shell("cd /tmp && rm -rf petclinic 2>/dev/null; true"))
                .with_step(Step::shell("git clone https://github.com/spring-projects/spring-petclinic.git /tmp/petclinic"))
                .with_step(Step::shell("cd /tmp/petclinic && ls -la"))
                .with_step(Step::echo("Repository cloned!")),
        )
        .with_stage(
            Stage::new("Build")
                .with_step(Step::shell("cd /tmp/petclinic && ./mvnw package -DskipTests"))
                .with_step(Step::echo("Build completed!")),
        )
        .with_stage(
            Stage::new("Test")
                .with_step(Step::shell("cd /tmp/petclinic && ./mvnw test"))
                .with_step(Step::echo("Tests completed!")),
        )
        .with_stage(
            Stage::new("Report")
                .with_step(Step::shell("cd /tmp/petclinic && find . -name '*.jar' -type f 2>/dev/null | head -5"))
                .with_step(Step::shell("cd /tmp/petclinic && du -sh target/ 2>/dev/null || echo 'No target dir'"))
                .with_step(Step::shell("cd /tmp/petclinic && echo '=== BUILD SUCCESS ==='"))
                .with_step(Step::shell("cd /tmp/petclinic && echo 'Pipeline completed at: $(date)'"))
                .with_step(Step::echo("Pipeline complete!")),
        );

    println!("Pipeline: {}", pl.name().unwrap_or_default());
    println!("Stages: {}", pl.stages.len());
    for (i, stage) in pl.stages.iter().enumerate() {
        println!("  {}. {} ({} steps)", i + 1, stage.name, stage.steps.len());
    }
    
    // Run the pipeline
    let mut runner = PipelineRunner::new();
    println!("\n--- Running Pipeline ---");
    println!("(This may take several minutes for Maven build/test)\n");
    let results = runner.run_async(&pl).await?;
    
    println!("\n--- Results ---");
    println!("Success: {}", results.success);
    println!("Duration: {}ms", results.duration_ms);
    println!("Stages executed: {}", results.stages_executed);
    println!("Steps executed: {}", results.steps_executed);
    
    if results.success {
        println!("\n✅ PetClinic Pipeline completed successfully!");
    } else {
        println!("\n❌ PetClinic Pipeline failed!");
        if let Some(err) = results.error {
            println!("Error: {}", err);
        }
    }
    
    Ok(())
}
