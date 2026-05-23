//! # Graph Command Module
//!
//! Generates pipeline graphs in various formats (Mermaid, DOT).

use crate::commands::GraphArgs;
use anyhow::Result;
use pipeliner_core::Pipeline;

/// Graph output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphFormat {
    /// Mermaid flowchart syntax
    Mermaid,
    /// Graphviz DOT format
    Dot,
}

impl GraphFormat {
    /// Parse a string into a GraphFormat
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mermaid" => Some(Self::Mermaid),
            "dot" => Some(Self::Dot),
            _ => None,
        }
    }
}

/// Generates a Mermaid flowchart from a pipeline.
///
/// # Arguments
///
/// * `pipeline` - The pipeline to generate the graph for
///
/// # Example
///
/// ```ignore
/// use pipeliner_core::Pipeline;
/// use pipeliner_cli::commands::graph::generate_mermaid;
///
/// let pipeline = Pipeline::new().with_name("My Pipeline");
/// let mermaid = generate_mermaid(&pipeline)?;
/// println!("{}", mermaid);
/// ```
pub fn generate_mermaid(pipeline: &Pipeline) -> Result<String> {
    let mut output = String::new();

    // Start the flowchart
    output.push_str("flowchart TD\n");

    // Add the pipeline name as a comment
    if let Some(ref name) = pipeline.name {
        output.push_str(&format!("    %% Pipeline: {}\n", name));
    }

    // Generate nodes for each stage
    let mut stage_ids = Vec::new();
    for (idx, stage_or_parallel) in pipeline.stages.iter().enumerate() {
        let stage_name = stage_or_parallel.name().unwrap_or("unnamed");
        let stage_id = format!("S{}", idx);
        stage_ids.push((stage_id.clone(), stage_name.to_string()));

        // Create node for stage
        output.push_str(&format!(
            "    {}[\"{}\"]\n",
            stage_id,
            escape_label(stage_name)
        ));
    }

    // Create connections between stages (sequential)
    for i in 0..stage_ids.len().saturating_sub(1) {
        output.push_str(&format!(
            "    {} --> {}\n",
            stage_ids[i].0, stage_ids[i + 1].0
        ));
    }

    Ok(output)
}

/// Generates a DOT (Graphviz) digraph from a pipeline.
///
/// # Arguments
///
/// * `pipeline` - The pipeline to generate the graph for
///
/// # Example
///
/// ```ignore
/// use pipeliner_core::Pipeline;
/// use pipeliner_cli::commands::graph::generate_dot;
///
/// let pipeline = Pipeline::new().with_name("My Pipeline");
/// let dot = generate_dot(&pipeline)?;
/// println!("{}", dot);
/// ```
pub fn generate_dot(pipeline: &Pipeline) -> Result<String> {
    let mut output = String::new();

    // Start the digraph
    let graph_name = pipeline
        .name
        .as_deref()
        .unwrap_or("pipeline")
        .replace('"', "\\\"");
    output.push_str(&format!("digraph \"{}\" {{\n", graph_name));
    output.push_str("    rankdir=TB\n");
    output.push_str("    node [shape=box]\n");

    // Generate nodes for each stage
    let mut stage_ids = Vec::new();
    for (idx, stage_or_parallel) in pipeline.stages.iter().enumerate() {
        let stage_name = stage_or_parallel.name().unwrap_or("unnamed");
        let stage_id = format!("s{}", idx);
        stage_ids.push((stage_id.clone(), stage_name.to_string()));

        // Create node for stage
        output.push_str(&format!(
            "    {} [label=\"{}\"]\n",
            stage_id,
            escape_label(stage_name)
        ));
    }

    // Create connections between stages (sequential)
    for i in 0..stage_ids.len().saturating_sub(1) {
        output.push_str(&format!(
            "    {} -> {}\n",
            stage_ids[i].0, stage_ids[i + 1].0
        ));
    }

    // End the digraph
    output.push_str("}\n");

    Ok(output)
}

/// Escapes special characters in graph labels.
fn escape_label(label: &str) -> String {
    label
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Generate a pipeline graph and print it to stdout.
///
/// # Arguments
///
/// * `args` - Graph command arguments containing pipeline source and format
///
/// # Example
///
/// ```ignore
/// use pipeliner_cli::commands::graph::graph_pipeline;
///
/// let args = GraphArgs {
///     script: Some("pipeline.json".into()),
///     definition: None,
///     format: "mermaid".to_string(),
/// };
///
/// graph_pipeline(args)?;
/// ```
pub fn graph_pipeline(args: GraphArgs) -> Result<()> {
    use std::io::Write;

    // Get pipeline definition from script or inline definition
    let definition = if let Some(ref path) = args.script {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read pipeline file: {:?}", path))?
    } else if let Some(ref def) = args.definition {
        def.clone()
    } else {
        anyhow::bail!("Either a pipeline file or --definition must be provided");
    };

    // Parse the pipeline
    let pipeline: Pipeline = serde_json::from_str(&definition)
        .context("Failed to parse pipeline JSON")?;

    // Generate the graph based on format
    let output = match args.format.to_lowercase().as_str() {
        "mermaid" => generate_mermaid(&pipeline)?,
        "dot" => generate_dot(&pipeline)?,
        _ => anyhow::bail!(
            "Unsupported format: {}. Valid options: mermaid, dot",
            args.format
        ),
    };

    // Print to stdout
    println!("{}", output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::{Stage, Step, StepType};

    fn create_test_pipeline() -> Pipeline {
        Pipeline::new()
            .with_name("Test Pipeline")
            .with_stage(Stage {
                name: "build".to_string(),
                agent: None,
                environment: Default::default(),
                options: None,
                when: None,
                post: None,
                steps: vec![Step {
                    step_type: StepType::Echo {
                        message: "Building".to_string(),
                    },
                    name: Some("build-step".to_string()),
                    timeout: None,
                    retry: None,
                }],
            })
            .with_stage(Stage {
                name: "test".to_string(),
                agent: None,
                environment: Default::default(),
                options: None,
                when: None,
                post: None,
                steps: vec![Step {
                    step_type: StepType::Echo {
                        message: "Testing".to_string(),
                    },
                    name: Some("test-step".to_string()),
                    timeout: None,
                    retry: None,
                }],
            })
    }

    #[test]
    fn test_graph_format_from_str_mermaid() {
        assert_eq!(
            GraphFormat::from_str("mermaid"),
            Some(GraphFormat::Mermaid)
        );
        assert_eq!(
            GraphFormat::from_str("Mermaid"),
            Some(GraphFormat::Mermaid)
        );
    }

    #[test]
    fn test_graph_format_from_str_dot() {
        assert_eq!(GraphFormat::from_str("dot"), Some(GraphFormat::Dot));
        assert_eq!(GraphFormat::from_str("DOT"), Some(GraphFormat::Dot));
    }

    #[test]
    fn test_graph_format_from_str_invalid() {
        assert_eq!(GraphFormat::from_str("invalid"), None);
        assert_eq!(GraphFormat::from_str(""), None);
    }

    #[test]
    fn test_generate_mermaid_basic() {
        let pipeline = create_test_pipeline();
        let result = generate_mermaid(&pipeline).unwrap();

        assert!(result.contains("flowchart TD"));
        assert!(result.contains("Test Pipeline"));
        assert!(result.contains("S0"));
        assert!(result.contains("S1"));
        assert!(result.contains("build"));
        assert!(result.contains("test"));
        assert!(result.contains("-->"));
    }

    #[test]
    fn test_generate_dot_basic() {
        let pipeline = create_test_pipeline();
        let result = generate_dot(&pipeline).unwrap();

        assert!(result.contains("digraph"));
        assert!(result.contains("Test Pipeline"));
        assert!(result.contains("s0"));
        assert!(result.contains("s1"));
        assert!(result.contains("build"));
        assert!(result.contains("test"));
        assert!(result.contains("->"));
    }

    #[test]
    fn test_escape_label() {
        assert_eq!(escape_label("hello"), "hello");
        assert_eq!(escape_label("a & b"), "a &amp; b");
        assert_eq!(escape_label("a < b"), "a &lt; b");
        assert_eq!(escape_label("a > b"), "a &gt; b");
        assert_eq!(escape_label("a \"b\""), "a &quot;b&quot;");
    }

    #[test]
    fn test_mermaid_with_empty_pipeline() {
        let pipeline = Pipeline::new().with_name("Empty");
        let result = generate_mermaid(&pipeline).unwrap();

        assert!(result.contains("flowchart TD"));
        assert!(result.contains("Empty"));
    }

    #[test]
    fn test_dot_with_empty_pipeline() {
        let pipeline = Pipeline::new().with_name("Empty");
        let result = generate_dot(&pipeline).unwrap();

        assert!(result.contains("digraph"));
        assert!(result.contains("Empty"));
    }
}
