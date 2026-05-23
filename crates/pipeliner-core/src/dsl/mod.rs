//! DSL module - Runtime pipeline parsing without compilation
//!
//! Write pipelines in .dsl files, run without compilation:
//!
//! ```rust,ignore
//! use pipeliner_core::dsl::parse_pipeline;
//! 
//! let pipeline = parse_pipeline(r#"
//!     pipeline {
//!         name = \"CI\"
//!         stages {
//!             stage(\"Build\") {
//!                 steps {
//!                     sh \"cargo build\"
//!                 }
//!             }
//!         }
//!     }
//! "#)?;
//! ```

pub mod parser;

pub use parser::parse_pipeline;

use crate::{Pipeline, PipelineRunner};
use std::path::Path;

/// Run a parsed pipeline
pub async fn run_pipeline(pipeline: &Pipeline) -> Result<crate::runtime::PipelineRunResult, crate::runtime::RuntimeError> {
    let mut runner = PipelineRunner::new();
    runner.run_async(pipeline).await
}

/// Load and parse a pipeline from a file
pub fn load_pipeline_file(path: impl AsRef<Path>) -> Result<Pipeline, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    parse_pipeline(&content)
}
