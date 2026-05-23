#!/usr/bin/env cargo
//! ```cargo
//! [dependencies]
//! pipeliner-core = { path = "../crates/pipeliner-core" }
//! pipeliner-macros = { path = "../crates/pipeliner-macros" }
//! serde_json = "1"
//! ```

//！这个暂时不使用 macro，因为 macro 需要正确的 cargo 依赖
// use pipeliner_macros::pipeline;

use pipeliner_core::spec::{PipelineSpec, StageSpec, StageExecution, StepSpec, ShellStepSpec, EchoStepSpec};

fn main() {
    // 使用旧 API 手动构建 - 验证 PipelineSpec 结构
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
            StageSpec {
                id: "test".to_string(),
                display_name: "Test".to_string(),
                execution: StageExecution::Steps {
                    steps: vec![
                        StepSpec::Shell(ShellStepSpec::new("cargo test")),
                    ],
                },
                post: None,
            },
        ],
        post: None,
    };

    serde_json::to_writer(std::io::stdout(), &spec).unwrap();
}