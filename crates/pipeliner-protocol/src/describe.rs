use std::io::{self, Write};

#[derive(Debug, thiserror::Error)]
pub enum DescribeError {
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// Write PipelineSpec as JSON to stdout
pub fn describe_to_stdout(spec: &pipeliner_core::spec::PipelineSpec) -> Result<(), DescribeError> {
    let json = serde_json::to_string_pretty(spec)?;
    println!("{}", json);
    Ok(())
}

/// Write PipelineSpec as JSON to any Writer
pub fn describe_to_writer<W: Write>(
    spec: &pipeliner_core::spec::PipelineSpec,
    writer: W,
) -> Result<(), DescribeError> {
    serde_json::to_writer(writer, spec)?;
    Ok(())
}
