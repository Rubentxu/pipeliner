#!/usr/bin/env rustline-run
fn main() {
    let pipeline = std::env::var("PIPELINE_NAME").unwrap_or_else(|_| "unknown".to_string());
    let stage = std::env::var("PIPELINE_STAGE").unwrap_or_else(|_| "unknown".to_string());
    let step = std::env::var("PIPELINE_STEP").unwrap_or_else(|_| "unknown".to_string());
    println!("Pipeline: {}", pipeline);
    println!("Stage: {}", stage);
    println!("Step: {}", step);

    // Show current directory and hostname
    let cwd = std::env::current_dir().unwrap();
    println!("Working directory: {}", cwd.display());

    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "localhost".to_string());
    println!("Hostname: {}", hostname);
}