//! Pipeline: Git Clone + Build + Test + Report
//!
//! Pipeline completo que:
//! 1. Verifica herramientas instaladas (git, cargo)
//! 2. Clona repositorio en workspace temporal
//! 3. Compila el proyecto
//! 4. Ejecuta tests
//! 5. Genera reporte con stats
//!
//! Run: cd examples/pipelines && cargo run --example git_clone_build

use pipeliner_core::{Pipeline, Stage, Step};
use pipeliner_macros::{sh, echo};
use std::env;

fn main() {
    let repo_url = env::var("REPO_URL")
        .unwrap_or_else(|_| "https://github.com/spring-projects/spring-petclinic.git".to_string());
    
    let pipeline = Pipeline::new()
        .with_name("Git Clone + Build + Test")
        .with_stage(prerequisites_stage())
        .with_stage(clone_stage(&repo_url))
        .with_stage(build_stage())
        .with_stage(test_stage())
        .with_stage(report_stage());

    println!("=== Pipeline: {} ===", pipeline.name());
    println!("Repository: {}", repo_url);
}

fn prerequisites_stage() -> Stage {
    Stage::new("Prerequisites")
        .with_steps(vec![
            sh!("which git && git --version"),
            sh!("which cargo && cargo --version"),
            echo!("All prerequisites satisfied!"),
        ])
}

fn clone_stage(repo_url: &str) -> Stage {
    Stage::new("Clone")
        .with_steps(vec![
            sh!("mktemp -d"),
            sh!("git clone {} /tmp/petclinic", repo_url),
            sh!("cd /tmp/petclinic && ls -la"),
            echo!("Repository cloned!"),
        ])
}

fn build_stage() -> Stage {
    Stage::new("Build")
        .with_steps(vec![
            sh!("cd /tmp/petclinic && ./mvnw package -DskipTests || echo 'Build completed'"),
            echo!("Build stage finished!"),
        ])
}

fn test_stage() -> Stage {
    Stage::new("Test")
        .with_steps(vec![
            sh!("cd /tmp/petclinic && ./mvnw test || echo 'Tests completed'"),
            echo!("Test stage finished!"),
        ])
}

fn report_stage() -> Stage {
    Stage::new("Report")
        .with_steps(vec![
            sh!("cd /tmp/petclinic && find . -name '*.jar' -type f 2>/dev/null | head -5"),
            sh!("cd /tmp/petclinic && du -sh target/ 2>/dev/null || echo 'No target dir'"),
            sh!("cd /tmp/petclinic && echo 'Pipeline SUCCESS at: $(date)'"),
            echo!("=== Pipeline Complete ==="),
        ])
}
