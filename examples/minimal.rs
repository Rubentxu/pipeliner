#!/usr/bin/env cargo
//! ```cargo
//! [dependencies]
//! pipeliner-core = { path = "../crates/pipeliner-core" }
//! serde_json = "1"
//! ```

use pipeliner_core::spec::{PipelineSpec, StageSpec, StageExecution, StepSpec, ShellStepSpec, EchoStepSpec};

fn main() {
    let spec = PipelineSpec {
        schema_version: "pipeliner.pipeline.v1".to_string(),
        pipeliner_version: "0.1.0".to_string(),
        stages: vec![
            StageSpec {
                id: "build".to_string(),
                display_name: "Build".to_string(),
                execution: StageExecution::Steps {
                    steps: vec![
                        StepSpec::Shell(ShellStepSpec::new("cargo build")),
                        StepSpec::Echo(EchoStepSpec { message: "done".to_string() }),
                    ],
                },
                post: None,
            },
        ],
        post: None,
    };

    let json = serde_json::to_string_pretty(&spec).unwrap();
    println!("{}", json);
}