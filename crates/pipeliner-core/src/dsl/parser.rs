//! Simple DSL Parser for Jenkins-style pipelines
//!
//! Write pipelines in .dsl files, run without compilation:
//!
//! ```bash
//! pipeliner run pipeline.dsl
//! ```

use crate::{Pipeline, Stage, Step, StageOrParallel, Environment, AgentType, ParallelGroup};
use std::time::Duration;

pub type DslResult<T> = Result<T, String>;

/// Parse pipeline from DSL string
pub fn parse_pipeline(input: &str) -> DslResult<Pipeline> {
    let mut pipeline = Pipeline::new();
    let mut in_stages = false;
    let mut in_stage = false;
    let mut current_stage: Option<Stage> = None;
    let mut current_steps: Vec<Step> = Vec::new();
    let mut in_environment = false;
    
    for line in input.lines() {
        let line = line.trim();
        
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Pipeline start
        if line == "pipeline {" {
            continue;
        }
        
        // Pipeline end
        if line == "}" && !in_stages && !in_environment && !in_stage {
            break;
        }
        
        // Name
        if line.starts_with("name = ") {
            let name = line.trim_start_matches("name = ").trim_matches('"');
            pipeline = pipeline.with_name(name);
            continue;
        }
        
        // Agent
        if line.starts_with("agent = ") {
            let agent_str = line.trim_start_matches("agent = ").trim();
            if agent_str == "any" {
                pipeline = pipeline.with_agent(AgentType::any());
            } else if agent_str.starts_with("docker(") {
                let image = agent_str.trim_start_matches("docker(\"").trim_end_matches("\")");
                pipeline = pipeline.with_agent(AgentType::docker(image));
            }
            continue;
        }
        
        // Environment
        if line == "environment {" {
            in_environment = true;
            continue;
        }
        if line == "}" && in_environment {
            in_environment = false;
            continue;
        }
        if in_environment && line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim();
                let value = parts[1].trim().trim_matches('"');
                let mut env = Environment::new();
                env.insert(key, value);
                pipeline = pipeline.with_environment(env);
            }
            continue;
        }
        
        // Stages
        if line == "stages {" {
            in_stages = true;
            continue;
        }
        if line == "}" && in_stages && !in_stage {
            in_stages = false;
            continue;
        }
        
        // Stage start
        if line.starts_with("stage(") || line.starts_with("stage (\"") {
            // Save previous stage if exists
            if let Some(stage) = current_stage.take() {
                pipeline = pipeline.with_stage(StageOrParallel::Stage(stage));
            }
            
            // Parse stage name
            let name = if line.contains('(') {
                line.split('(').nth(1)
                    .and_then(|s| s.split(')').next())
                    .map(|s| s.trim_matches('"'))
                    .unwrap_or("Unnamed")
            } else {
                "Unnamed"
            };
            
            current_stage = Some(Stage::new(name));
            current_steps = Vec::new();
            in_stage = true;
            continue;
        }
        
        // Steps block
        if line == "steps {" {
            continue;
        }
        
        // Parallel block
        if line.starts_with("parallel {") || line == "parallel{" {
            // For now, skip parallel - would need more complex handling
            continue;
        }
        
        // sh command
        if line.starts_with("sh ") || line.starts_with("sh\"") {
            let cmd = if line.starts_with("sh \"") {
                line.trim_start_matches("sh \"").trim_end_matches('"')
            } else {
                line.trim_start_matches("sh ")
            };
            current_steps.push(Step::shell(cmd));
            continue;
        }
        
        // echo command
        if line.starts_with("echo ") || line.starts_with("echo\"") {
            let msg = if line.starts_with("echo \"") {
                line.trim_start_matches("echo \"").trim_end_matches('"')
            } else {
                line.trim_start_matches("echo ")
            };
            current_steps.push(Step::echo(msg));
            continue;
        }
        
        // timeout
        if line.starts_with("timeout(") {
            // Extract timeout value
            let mins = line.split('(').nth(1)
                .and_then(|s| s.split(')').next())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);
            
            // Find closing brace for steps
            let steps_start = current_steps.len();
            // For simplicity, wrap all current steps in timeout
            continue;
        }
        
        // Stage end
        if line == "}" && in_stage {
            if let Some(mut stage) = current_stage.take() {
                stage = stage.with_steps(current_steps.clone());
                pipeline = pipeline.with_stage(StageOrParallel::Stage(stage));
            }
            current_steps = Vec::new();
            in_stage = false;
            continue;
        }
        
        // Post block
        if line == "post {" {
            // Skip post for now
            continue;
        }
    }
    
    // Save last stage
    if let Some(mut stage) = current_stage {
        stage = stage.with_steps(current_steps);
        pipeline = pipeline.with_stage(StageOrParallel::Stage(stage));
    }
    
    if pipeline.stages.is_empty() {
        return Err("Pipeline has no stages".to_string());
    }
    
    Ok(pipeline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_pipeline() {
        let dsl = r#"
pipeline {
    name = "Test"
    stages {
        stage("Build") {
            steps {
                sh "cargo build"
            }
        }
    }
}
"#;
        let pipeline = parse_pipeline(dsl).unwrap();
        assert_eq!(pipeline.name(), Some("Test"));
        assert_eq!(pipeline.stages.len(), 1);
    }

    #[test]
    fn test_pipeline_with_env() {
        let dsl = r#"
pipeline {
    name = "Test"
    environment {
        FOO = "bar"
    }
    stages {
        stage("Build") {
            steps {
                sh "echo $FOO"
            }
        }
    }
}
"#;
        let pipeline = parse_pipeline(dsl).unwrap();
        assert_eq!(pipeline.name(), Some("Test"));
    }
}
