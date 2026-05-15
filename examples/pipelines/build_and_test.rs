//! Pipeline: Build + Test + Verify Binary Stats
//!
//! Este pipeline demuestra un flujo CI/CD típico:
//! 1. Compilar en release
//! 2. Ejecutar tests
//! 3. Verificar binary creado
//! 4. Reportar stats
//!
//! Run: cd crates && cargo run -p pipeliner-cli --example build_and_test

use pipeliner_core::{Pipeline, Stage, Step, PipelineRunner};
use pipeliner_macros::{sh, echo};

fn main() {
    let pipeline = Pipeline::new()
        .with_name("Build + Test CI")
        .with_stage(build_stage())
        .with_stage(test_stage())
        .with_stage(verify_stage())
        .with_stage(report_stage());

    println!("=== Pipeline: {} ===", pipeline.name());
    println!("Stages: {}", pipeline.stages.len());
}

fn build_stage() -> Stage {
    Stage::new("Build")
        .with_steps(vec![
            sh!("cargo build --release"),
            echo!("Build completed successfully!"),
        ])
}

fn test_stage() -> Stage {
    Stage::new("Test")
        .with_steps(vec![
            sh!("cargo test --all"),
            echo!("All tests passed!"),
        ])
}

fn verify_stage() -> Stage {
    Stage::new("Verify")
        .with_steps(vec![
            sh!("ls -lh target/release/pipeliner || ls -lh target/debug/pipeliner"),
            sh!("stat target/release/pipeliner 2>/dev/null || stat target/debug/pipeliner"),
            echo!("Binary verified!"),
        ])
}

fn report_stage() -> Stage {
    Stage::new("Report")
        .with_steps(vec![
            sh!("echo '=== BUILD SUCCESS ==='"),
            sh!("echo 'Pipeline completed at: $(date)'"),
            echo!("CI Pipeline finished!"),
        ])
}
