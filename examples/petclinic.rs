//! Pipeline PetClinic: Clone + Build + Test + Report
//!
//! Pipeline que clona un repositorio Java, lo compila y ejecuta tests.
//!
//! Run: cargo run --example petclinic
//!
//! Requiere: git, java, maven

use pipeliner_core::{Pipeline, Stage, Step, PipelineRunner};
use pipeliner_macros::pipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PetClinic CI Pipeline ===\n");

    // DSL declarativo estilo Jenkinsfile
    let pl = pipeline! {
        name = "PetClinic CI"
        
        stages {
            stage!("Prerequisites") {
                steps {
                    sh!("which git && git --version")
                    sh!("which java && java -version")
                    sh!("which mvn && mvn --version")
                    echo!("Prerequisites satisfied!")
                }
            }
            
            stage!("Clone") {
                steps {
                    sh!("cd /tmp && rm -rf petclinic 2>/dev/null; true")
                    sh!("git clone https://github.com/spring-projects/spring-petclinic.git /tmp/petclinic")
                    sh!("cd /tmp/petclinic && ls -la")
                    echo!("Repository cloned!")
                }
            }
            
            stage!("Build") {
                steps {
                    sh!("cd /tmp/petclinic && ./mvnw package -DskipTests")
                    echo!("Build completed!")
                }
            }
            
            stage!("Test") {
                steps {
                    sh!("cd /tmp/petclinic && ./mvnw test")
                    echo!("Tests completed!")
                }
            }
            
            stage!("Report") {
                steps {
                    sh!("cd /tmp/petclinic && find . -name '*.jar' -type f 2>/dev/null | head -5")
                    sh!("cd /tmp/petclinic && du -sh target/ 2>/dev/null || echo 'No target dir'")
                    sh!("cd /tmp/petclinic && echo '=== BUILD SUCCESS ==='")
                    sh!("cd /tmp/petclinic && echo 'Pipeline completed at: $(date)'")
                    echo!("Pipeline complete!")
                }
            }
        }
    };

    println!("Pipeline: {}", pl.name().unwrap_or_default());
    println!("Stages: {}", pl.stages.len());
    for (i, item) in pl.stages.iter().enumerate() {
        match item {
            pipeliner_core::pipeline::StageOrParallel::Stage(s) => println!("  {}. {} ({} steps)", i + 1, s.name, s.steps.len()),
            pipeliner_core::pipeline::StageOrParallel::Parallel(g) => println!("  {}. parallel ({} stages)", i + 1, g.stages.len()),
        }
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
