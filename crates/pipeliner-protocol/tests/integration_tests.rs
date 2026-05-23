//! Integration tests for pipeliner-protocol
//!
//! These tests verify the public API of the pipeliner-protocol crate.

use pipeliner_core::spec::{
    PipelineSpec, StageSpec, PostSpec,
    step_spec::{EchoStepSpec, StepSpec},
};

/// Test that describe_to_stdout produces valid JSON that can be deserialized back
#[test]
fn test_describe_to_stdout_produces_valid_json() {
    // Create a PipelineSpec with meaningful data (not empty)
    let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
        .with_stage(
            StageSpec::new("build", "Build")
                .with_steps(vec![StepSpec::Echo(EchoStepSpec {
                    message: "hello world".to_string(),
                })]),
        );

    // Use describe_to_writer to capture output since stdout capture is complex
    let mut buffer = Vec::new();
    pipeliner_protocol::describe_to_writer(&spec, &mut buffer)
        .expect("describe_to_writer should succeed");

    // Verify it can be deserialized to valid PipelineSpec
    let json_str = String::from_utf8(buffer).expect("should be valid UTF-8");
    let parsed: PipelineSpec = serde_json::from_str(&json_str)
        .expect("JSON should be valid and deserialize to PipelineSpec");

    // Verify the deserialized spec matches original
    assert_eq!(parsed.schema_version, spec.schema_version);
    assert_eq!(parsed.pipeliner_version, spec.pipeliner_version);
    assert_eq!(parsed.stages.len(), spec.stages.len());
    assert_eq!(parsed.stages[0].id, "build");
    assert_eq!(parsed.stages[0].display_name, "Build");
}

/// Test that describe_to_writer produces output that can be roundtripped
#[test]
fn test_describe_to_writer_roundtrip() {
    // Create a PipelineSpec with data that exercises serialization
    let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0")
        .with_stage(StageSpec::new("build", "Build").with_steps(vec![
            StepSpec::Echo(EchoStepSpec {
                message: "building...".to_string(),
            }),
        ]))
        .with_post(
            PostSpec::new().with_always_steps(vec![StepSpec::Echo(EchoStepSpec {
                message: "cleanup".to_string(),
            })]),
        );

    // Write to a Vec (in-memory writer)
    let mut buffer = Vec::new();
    pipeliner_protocol::describe_to_writer(&spec, &mut buffer)
        .expect("describe_to_writer should succeed");

    // Deserialize from the buffer
    let json_str = String::from_utf8(buffer).expect("buffer should be valid UTF-8");
    let roundtrip_spec =
        PipelineSpec::from_json(&json_str).expect("should deserialize from JSON");

    // Verify roundtrip preserved all data
    assert_eq!(roundtrip_spec.schema_version, spec.schema_version);
    assert_eq!(roundtrip_spec.pipeliner_version, spec.pipeliner_version);
    assert_eq!(roundtrip_spec.stages.len(), spec.stages.len());

    // Verify post was preserved
    assert!(roundtrip_spec.post.is_some());
    let post = roundtrip_spec.post.unwrap();
    assert_eq!(post.always.len(), 1);
    assert_eq!(post.always[0], spec.post.as_ref().unwrap().always[0]);
}

/// Test roundtrip with empty pipeline spec
#[test]
fn test_describe_to_writer_empty_spec() {
    let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0");

    let mut buffer = Vec::new();
    pipeliner_protocol::describe_to_writer(&spec, &mut buffer)
        .expect("describe_to_writer should succeed");

    let json_str = String::from_utf8(buffer).expect("buffer should be valid UTF-8");
    let roundtrip = PipelineSpec::from_json(&json_str).expect("should deserialize");

    assert_eq!(roundtrip.schema_version, "pipeliner.pipeline.v1");
    assert!(roundtrip.stages.is_empty());
    assert!(roundtrip.post.is_none());
}

/// Test SCHEMA_VERSION constant
#[test]
fn test_schema_version_constant() {
    assert_eq!(pipeliner_protocol::SCHEMA_VERSION, "pipeliner.pipeline.v1");
}

/// Test PIPELINER_VERSION constant
#[test]
fn test_pipeliner_version_constant() {
    assert_eq!(pipeliner_protocol::PIPELINER_VERSION, "0.1.0");
}

/// Test that all public exports are accessible
#[test]
fn test_public_exports() {
    // These should all compile - verifying the public API surface
    let _: &str = pipeliner_protocol::SCHEMA_VERSION;
    let _: &str = pipeliner_protocol::PIPELINER_VERSION;

    let spec = PipelineSpec::new("pipeliner.pipeline.v1", "0.1.0");
    let mut buffer = Vec::new();

    // Verify describe_to_writer works
    pipeliner_protocol::describe_to_writer(&spec, &mut buffer)
        .expect("should write successfully");
}
