//! Integration tests for the spec module.
//!
//! These tests verify JSON serialization and deserialization
//! for pipeline specifications.

use pipeliner_core::spec::{
    PipelineSpec, PostSpec, StageExecution, StageSpec,
    step_spec::{EchoStepSpec, InterpolationMode, ShellKind, ShellStepSpec, StepSpec},
};

#[test]
fn test_pipeline_spec_json_roundtrip() {
    // SCN-SPEC-001: PipelineSpec serializes and deserializes correctly
    let spec = PipelineSpec {
        schema_version: "pipeliner.pipeline.v1".into(),
        pipeliner_version: "0.1.0".into(),
        env: None,
        stages: vec![StageSpec {
            id: "build".into(),
            display_name: "Build".into(),
            env: None,
            options: None,
            execution: StageExecution::Steps {
                steps: vec![StepSpec::Echo(EchoStepSpec {
                    message: "hello".into(),
                })],
            },
            post: None,
        }],
        post: None,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&spec).unwrap();

    // Deserialize back
    let parsed: PipelineSpec = serde_json::from_str(&json).unwrap();

    // Verify equality
    assert_eq!(parsed.schema_version, spec.schema_version);
    assert_eq!(parsed.pipeliner_version, spec.pipeliner_version);
    assert_eq!(parsed.stages.len(), spec.stages.len());
}

#[test]
fn test_stage_spec_json_roundtrip() {
    // SCN-SPEC-002: StageSpec serializes and deserializes correctly
    let stage = StageSpec {
        id: "test".into(),
        display_name: "Test Stage".into(),
        env: None,
        options: None,
        execution: StageExecution::Steps {
            steps: vec![
                StepSpec::Echo(EchoStepSpec {
                    message: "Running tests...".into(),
                }),
                StepSpec::Shell(ShellStepSpec::new("cargo test")),
            ],
        },
        post: None,
    };

    let json = serde_json::to_string(&stage).unwrap();
    let parsed: StageSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, stage.id);
    assert_eq!(parsed.display_name, stage.display_name);
}

#[test]
fn test_step_spec_shell_json_roundtrip() {
    // SCN-SPEC-003: StepSpec::Shell serializes and deserializes correctly
    let step = StepSpec::Shell(
        ShellStepSpec::new("echo hello")
            .with_label("my step")
            .with_kind(ShellKind::PowerShell)
            .with_capture_stdout(),
    );

    let json = serde_json::to_string(&step).unwrap();
    let parsed: StepSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed, step);
}

#[test]
fn test_step_spec_echo_json_roundtrip() {
    // SCN-SPEC-004: StepSpec::Echo serializes and deserializes correctly
    let step = StepSpec::Echo(EchoStepSpec {
        message: "Hello, World!".into(),
    });

    let json = serde_json::to_string(&step).unwrap();
    let parsed: StepSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed, step);
}

#[test]
fn test_parallel_stage_execution_json_roundtrip() {
    // SCN-SPEC-005: Parallel stage execution serializes correctly
    let stage = StageSpec {
        id: "parallel".into(),
        display_name: "Parallel Stage".into(),
        env: None,
        options: None,
        execution: StageExecution::Parallel {
            stages: vec![
                StageSpec {
                    id: "job1".into(),
                    display_name: "Job 1".into(),
                    env: None,
                    options: None,
                    execution: StageExecution::Steps {
                        steps: vec![StepSpec::Echo(EchoStepSpec {
                            message: "Job 1".into(),
                        })],
                    },
                    post: None,
                },
                StageSpec {
                    id: "job2".into(),
                    display_name: "Job 2".into(),
                    env: None,
                    options: None,
                    execution: StageExecution::Steps {
                        steps: vec![StepSpec::Echo(EchoStepSpec {
                            message: "Job 2".into(),
                        })],
                    },
                    post: None,
                },
            ],
        },
        post: None,
    };

    let json = serde_json::to_string(&stage).unwrap();
    let parsed: StageSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, "parallel");
    match parsed.execution {
        StageExecution::Parallel { stages } => {
            assert_eq!(stages.len(), 2);
        }
        StageExecution::Steps { .. } => {
            panic!("Expected Parallel execution");
        }
    }
}

#[test]
fn test_post_spec_json_roundtrip() {
    // SCN-SPEC-006: PostSpec serializes and deserializes correctly
    let post = PostSpec {
        always: vec![StepSpec::Echo(EchoStepSpec {
            message: "cleanup".into(),
        })],
        success: vec![StepSpec::Echo(EchoStepSpec {
            message: "notify success".into(),
        })],
        failure: vec![StepSpec::Echo(EchoStepSpec {
            message: "notify failure".into(),
        })],
    };

    let json = serde_json::to_string(&post).unwrap();
    let parsed: PostSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.always.len(), 1);
    assert_eq!(parsed.success.len(), 1);
    assert_eq!(parsed.failure.len(), 1);
}

#[test]
fn test_pipeline_spec_with_post_json_roundtrip() {
    // SCN-SPEC-007: PipelineSpec with post-actions roundtrips correctly
    let spec = PipelineSpec {
        schema_version: "pipeliner.pipeline.v1".into(),
        pipeliner_version: "0.1.0".into(),
        env: None,
        stages: vec![StageSpec {
            id: "build".into(),
            display_name: "Build".into(),
            env: None,
            options: None,
            execution: StageExecution::Steps {
                steps: vec![StepSpec::Shell(ShellStepSpec::new("cargo build"))],
            },
            post: Some(PostSpec {
                always: vec![],
                success: vec![StepSpec::Echo(EchoStepSpec {
                    message: "Build succeeded".into(),
                })],
                failure: vec![StepSpec::Echo(EchoStepSpec {
                    message: "Build failed".into(),
                })],
            }),
        }],
        post: None,
    };

    let json = serde_json::to_string(&spec).unwrap();
    let parsed: PipelineSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.stages.len(), 1);
    let parsed_stage = &parsed.stages[0];
    assert!(parsed_stage.post.is_some());
}

#[test]
fn test_shell_step_spec_all_fields() {
    // SCN-SPEC-008: ShellStepSpec with all fields serializes correctly
    let step = StepSpec::Shell(
        ShellStepSpec::new("echo test")
            .with_label("full test")
            .with_kind(ShellKind::Cmd)
            .with_interpolation(InterpolationMode::Raw)
            .with_capture_stdout()
            .with_return_status()
            .with_allow_failure(),
    );

    let json = serde_json::to_string(&step).unwrap();

    // Verify JSON contains expected fields
    assert!(json.contains("\"type\":\"shell\""));
    assert!(json.contains("\"kind\":\"cmd\""));
    assert!(json.contains("\"script\":\"echo test\""));
    assert!(json.contains("\"label\":\"full test\""));
    assert!(json.contains("\"interpolation\":\"raw\""));
    assert!(json.contains("\"capture_stdout\":true"));
    assert!(json.contains("\"return_status\":true"));
    assert!(json.contains("\"fail_on_nonzero\":false"));

    let parsed: StepSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, step);
}

#[test]
fn test_pipeline_spec_to_json_method() {
    // SCN-SPEC-009: PipelineSpec::to_json() method works
    let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0");

    let json = spec.to_json().unwrap();
    assert!(json.contains("pipeliner.pipeline.v1"));
    assert!(json.contains("0.1.0"));
}

#[test]
fn test_pipeline_spec_from_json_method() {
    // SCN-SPEC-010: PipelineSpec::from_json() method works
    let json = r#"{
        "schema_version": "pipeliner.pipeline.v1",
        "pipeliner_version": "0.1.0",
        "stages": []
    }"#;

    let spec = PipelineSpec::from_json(json).unwrap();
    assert_eq!(spec.schema_version, "pipeliner.pipeline.v1");
    assert_eq!(spec.pipeliner_version, "0.1.0");
}

#[test]
fn test_pipeline_spec_full_roundtrip() {
    // SCN-SPEC-011: Complete PipelineSpec with all features roundtrips
    let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
        .with_stage(
            StageSpec::new("build", "Build")
                .with_steps(vec![
                    StepSpec::Shell(ShellStepSpec::new("cargo build --release")),
                    StepSpec::Echo(EchoStepSpec {
                        message: "Build complete".into(),
                    }),
                ])
                .with_post(PostSpec {
                    always: vec![StepSpec::Echo(EchoStepSpec {
                        message: "Always runs".into(),
                    })],
                    success: vec![],
                    failure: vec![StepSpec::Echo(EchoStepSpec {
                        message: "Build failed notification".into(),
                    })],
                }),
        )
        .with_stage(
            StageSpec::new("test", "Test")
                .with_steps(vec![StepSpec::Shell(ShellStepSpec::new("cargo test"))]),
        )
        .with_post(PostSpec {
            always: vec![StepSpec::Echo(EchoStepSpec {
                message: "Pipeline finished".into(),
            })],
            success: vec![StepSpec::Echo(EchoStepSpec {
                message: "Pipeline succeeded".into(),
            })],
            failure: vec![StepSpec::Echo(EchoStepSpec {
                message: "Pipeline failed".into(),
            })],
        });

    let json = serde_json::to_string_pretty(&spec).unwrap();
    let parsed: PipelineSpec = serde_json::from_str(&json).unwrap();

    // Verify all fields
    assert_eq!(parsed.schema_version, spec.schema_version);
    assert_eq!(parsed.pipeliner_version, spec.pipeliner_version);
    assert_eq!(parsed.stages.len(), 2);

    // Verify first stage has post
    assert!(parsed.stages[0].post.is_some());
}
