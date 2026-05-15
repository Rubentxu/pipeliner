//! Tests for pipeliner-macros DSL macros

use pipeliner_core::{
    LlmAgentConfig, Pipeline, Stage, Step, StepType, Validate,
};

/// Helper macro to create a shell step (equivalent to sh!())
macro_rules! shell_step {
    ($cmd:expr) => {
        Step::shell($cmd)
    };
}

/// Helper macro to create an echo step (equivalent to echo!())
macro_rules! echo_step {
    ($msg:expr) => {
        Step::echo($msg)
    };
}

#[test]
fn test_shell_step_macro() {
    let step = shell_step!("cargo build");
    
    assert!(matches!(step.step_type, StepType::Shell { .. }));
    
    if let StepType::Shell { command } = step.step_type {
        assert_eq!(command, "cargo build");
    }
}

#[test]
fn test_echo_step_macro() {
    let step = echo_step!("Hello, world!");
    
    assert!(matches!(step.step_type, StepType::Echo { .. }));
    
    if let StepType::Echo { message } = step.step_type {
        assert_eq!(message, "Hello, world!");
    }
}

#[test]
fn test_shell_step_with_single_quotes() {
    // Test that shell command with single quotes works
    let step = shell_step!("echo 'hello world'");
    
    if let StepType::Shell { command } = step.step_type {
        assert_eq!(command, "echo 'hello world'");
    }
}

#[test]
fn test_pipeline_with_multiple_stages() {
    let setup_stage = Stage::new("Setup")
        .with_step(shell_step!("echo Setup"));
    
    let build_stage = Stage::new("Build")
        .with_step(shell_step!("cargo build"));
    
    let test_stage = Stage::new("Test")
        .with_step(shell_step!("cargo test"));
    
    let pipeline = Pipeline::new()
        .with_name("Test Pipeline")
        .with_stage(setup_stage)
        .with_stage(build_stage)
        .with_stage(test_stage);
    
    assert_eq!(pipeline.name(), Some("Test Pipeline"));
    assert_eq!(pipeline.stages.len(), 3);
    assert_eq!(pipeline.stages[0].name, "Setup");
    assert_eq!(pipeline.stages[1].name, "Build");
    assert_eq!(pipeline.stages[2].name, "Test");
}

#[test]
fn test_stage_with_multiple_steps() {
    let stage = Stage::new("Build")
        .with_step(shell_step!("cargo build"))
        .with_step(echo_step!("Build started"))
        .with_step(shell_step!("cargo test"))
        .with_step(echo_step!("Tests complete"));
    
    assert_eq!(stage.name, "Build");
    assert_eq!(stage.steps.len(), 4);
}

#[test]
fn test_step_with_name() {
    let step = shell_step!("cargo build").with_name("build-step");
    
    assert_eq!(step.name, Some("build-step".to_string()));
}

#[test]
fn test_step_with_timeout() {
    use std::time::Duration;
    
    let step = shell_step!("sleep 10").with_timeout(Duration::from_secs(5));
    
    assert_eq!(step.timeout, Some(Duration::from_secs(5)));
}

#[test]
fn test_step_with_retry() {
    let step = shell_step!("flaky-command").with_retry(3);
    
    assert_eq!(step.retry, Some(3));
}

#[test]
fn test_llm_agent_config_builder() {
    let config = LlmAgentConfig::new("claude-3-5-sonnet")
        .with_prompt("Review code for bugs")
        .with_tools(vec!["read_file".to_string(), "grep".to_string()])
        .with_skill("skills/code-review.md")
        .with_max_tokens(4096)
        .with_temperature(0.7);
    
    assert_eq!(config.model, "claude-3-5-sonnet");
    assert_eq!(config.prompt, "Review code for bugs");
    assert_eq!(config.tools.len(), 2);
    assert_eq!(config.skill, Some("skills/code-review.md".to_string()));
    assert_eq!(config.max_tokens, Some(4096));
    assert_eq!(config.temperature, Some(0.7));
}

#[test]
fn test_llm_agent_config_defaults() {
    let config = LlmAgentConfig::new("gpt-4");
    
    assert_eq!(config.model, "gpt-4");
    assert!(config.prompt.is_empty());
    assert!(config.tools.is_empty());
    assert!(config.skill.is_none());
    assert_eq!(config.max_tokens, Some(4096)); // default
    assert_eq!(config.temperature, Some(0.7)); // default
}

#[test]
fn test_agent_step_creation() {
    let config = LlmAgentConfig::new("claude")
        .with_prompt("Analyze this");
    
    let step = Step::agent(config.clone()).with_name("analyzer");
    
    assert!(matches!(step.step_type, StepType::Agent { .. }));
    assert_eq!(step.name, Some("analyzer".to_string()));
}

#[test]
fn test_agent_step_in_pipeline() {
    let agent_config = LlmAgentConfig::new("claude")
        .with_prompt("Review PR")
        .with_tools(vec!["read_file".to_string()]);
    
    let agent_step = Step::agent(agent_config);
    
    let stage = Stage::new("Review")
        .with_step(agent_step)
        .with_step(echo_step!("Review complete"));
    
    assert_eq!(stage.steps.len(), 2);
    
    // Check first step is agent
    assert!(matches!(stage.steps[0].step_type, StepType::Agent { .. }));
    
    // Check second step is echo
    assert!(matches!(stage.steps[1].step_type, StepType::Echo { .. }));
}

#[test]
fn test_pipeline_validation_empty_stages() {
    let pipeline = Pipeline::new();
    assert!(pipeline.validate().is_err());
}

#[test]
fn test_pipeline_validation_with_stages() {
    let stage = Stage::new("Build")
        .with_step(shell_step!("echo 'building'"));
    
    let pipeline = Pipeline::new()
        .with_stage(stage);
    
    assert!(pipeline.validate().is_ok());
}

#[test]
fn test_stage_validation_empty_name() {
    let stage = Stage::new("");
    assert!(stage.validate().is_err());
}

#[test]
fn test_stage_validation_valid_name() {
    let stage = Stage::new("Build")
        .with_step(shell_step!("build"));
    assert!(stage.validate().is_ok());
}

#[test]
fn test_pipeline_serialization() {
    use serde_json;
    
    let stage = Stage::new("Build")
        .with_step(shell_step!("cargo build"))
        .with_step(echo_step!("Build done"));
    
    let pipeline = Pipeline::new()
        .with_name("Test Pipeline")
        .with_stage(stage);
    
    // Serialize
    let json = serde_json::to_string(&pipeline).unwrap();
    assert!(json.contains("\"name\":\"Test Pipeline\""));
    assert!(json.contains("\"type\":\"shell\""));
    assert!(json.contains("\"type\":\"echo\""));
    
    // Deserialize
    let deserialized: Pipeline = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name(), Some("Test Pipeline"));
    assert_eq!(deserialized.stages.len(), 1);
}

#[test]
fn test_agent_config_serialization() {
    use serde_json;
    
    let config = LlmAgentConfig::new("claude")
        .with_prompt("Analyze code")
        .with_tools(vec!["grep".to_string()])
        .with_skill("review.md");
    
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"model\":\"claude\""));
    assert!(json.contains("\"prompt\":\"Analyze code\""));
    assert!(json.contains("\"skill\":\"review.md\""));
    
    let deserialized: LlmAgentConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.model, "claude");
    assert_eq!(deserialized.prompt, "Analyze code");
}

// ============================================================================
// Pipeline! Macro Tests (Jenkinsfile-style DSL)
// ============================================================================

use pipeliner_macros::{pipeline, sh, echo};

#[test]
fn test_pipeline_macro_simple() {
    let pipeline = pipeline! {
        name = "Test Pipeline"
        agent(any)
        
        stages {
            stage!("Build") {
                steps {
                    sh!("cargo build")
                    echo!("Build done")
                }
            }
        }
    };
    
    assert_eq!(pipeline.name(), Some("Test Pipeline"));
    assert_eq!(pipeline.stages.len(), 1);
    assert_eq!(pipeline.stages[0].name, "Build");
    assert_eq!(pipeline.stages[0].steps.len(), 2);
}

#[test]
fn test_pipeline_macro_multiple_stages() {
    let pipeline = pipeline! {
        name = "CI Pipeline"
        agent(any)
        
        stages {
            stage!("Build") {
                steps {
                    sh!("cargo build --release")
                }
            }
            
            stage!("Test") {
                steps {
                    sh!("cargo test")
                }
            }
            
            stage!("Verify") {
                steps {
                    sh!("ls -lh target/release")
                    echo!("Done!")
                }
            }
        }
    };
    
    assert_eq!(pipeline.name(), Some("CI Pipeline"));
    assert_eq!(pipeline.stages.len(), 3);
    assert_eq!(pipeline.stages[0].name, "Build");
    assert_eq!(pipeline.stages[1].name, "Test");
    assert_eq!(pipeline.stages[2].name, "Verify");
}

#[test]
fn test_pipeline_macro_petclinic_style() {
    let pipeline = pipeline! {
        name = "PetClinic CI"
        agent(any)
        
        stages {
            stage!("Prerequisites") {
                steps {
                    sh!("which git")
                    sh!("which java")
                    sh!("which mvn")
                }
            }
            
            stage!("Clone") {
                steps {
                    sh!("git clone https://github.com/spring-projects/spring-petclinic.git /tmp/petclinic")
                }
            }
            
            stage!("Build") {
                steps {
                    sh!("cd /tmp/petclinic && ./mvnw package -DskipTests")
                }
            }
            
            stage!("Test") {
                steps {
                    sh!("cd /tmp/petclinic && ./mvnw test")
                }
            }
            
            stage!("Report") {
                steps {
                    sh!("cd /tmp/petclinic && find . -name '*.jar' -type f")
                    sh!("echo 'Pipeline SUCCESS'")
                }
            }
        }
    };
    
    assert_eq!(pipeline.name(), Some("PetClinic CI"));
    assert_eq!(pipeline.stages.len(), 5);
    
    // Verify stage names
    assert_eq!(pipeline.stages[0].name, "Prerequisites");
    assert_eq!(pipeline.stages[1].name, "Clone");
    assert_eq!(pipeline.stages[2].name, "Build");
    assert_eq!(pipeline.stages[3].name, "Test");
    assert_eq!(pipeline.stages[4].name, "Report");
}
