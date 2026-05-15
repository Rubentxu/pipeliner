//! Example: Agent Pipeline with DSL macros
//!
//! Demonstrates how to use AgentStep with the DSL macros.
//!
//! Run with: cd crates && cargo run -p pipeliner-agent --example agent-pipeline

use pipeliner_core::{LlmAgentConfig, Pipeline, Stage, Step, StepType};
use pipeliner_macros::{echo, sh};

fn main() {
    println!("=== Agent Pipeline with DSL Macros ===\n");

    // Create a simple agent step
    let agent_config = LlmAgentConfig::new("claude-3-5-sonnet")
        .with_prompt("Review this code for potential bugs")
        .with_tools(vec!["read_file".to_string(), "grep".to_string()])
        .with_skill("skills/code-review.md");
    
    let agent_step = Step::agent(agent_config).with_name("code-reviewer");

    // Create stages using the macros for simple steps
    let setup_stage = Stage::new("Setup")
        .with_step(sh!("echo 'Starting agent pipeline...'"))
        .with_step(echo!("Environment ready"));

    let review_stage = Stage::new("Code Review")
        .with_step(agent_step)
        .with_step(echo!("Review complete!"));

    let report_stage = Stage::new("Report")
        .with_step(sh!("echo 'Generating report...'"));

    // Create a pipeline
    let pipeline = Pipeline::new()
        .with_name("Agent Example Pipeline")
        .with_stage(setup_stage)
        .with_stage(review_stage)
        .with_stage(report_stage);

    println!("Pipeline: {:?}", pipeline.name());
    println!("Stages: {}", pipeline.stages.len());
    for stage in &pipeline.stages {
        println!("  - Stage '{}': {} steps", stage.name, stage.steps.len());
        for step in &stage.steps {
            let step_type = match &step.step_type {
                StepType::Shell { command } => format!("shell: {}", &command[..command.len().min(30)]),
                StepType::Echo { message } => format!("echo: {}", message),
                StepType::Agent { config } => {
                    println!("    Agent model: {}", config.model);
                    "agent".to_string()
                }
                _ => "other".to_string(),
            };
            println!("    * {} (name: {:?})", step_type, step.name);
        }
    }

    println!("\nNote: Agent execution requires Rig integration.");
    println!("Current implementation is a mock that echoes the prompt.");
}
