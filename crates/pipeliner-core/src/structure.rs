//! Pipeline structure types for external visualization.
//!
//! These types represent the declarative structure of a pipeline (stages, steps, DAG)
//! for consumption by external systems like the Bastion dashboard.
//!
//! Used by `Pipeline::structure()` to emit `PipelineDecl` events before execution starts.

use serde::{Deserialize, Serialize};

/// Serializable pipeline structure for external consumers (dashboard, Bastion).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStructure {
    /// List of stages in the pipeline
    pub stages: Vec<StageStructure>,
}

/// Serializable stage structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageStructure {
    /// Stage name
    pub name: String,
    /// Steps in this stage
    pub steps: Vec<StepStructure>,
    /// Whether this stage has parallel branches
    pub has_parallel: bool,
    /// Whether this stage uses matrix expansion
    pub has_matrix: bool,
    /// Human-readable description of when condition
    pub when_condition: Option<String>,
}

/// Serializable step structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStructure {
    /// Optional step name
    pub name: Option<String>,
    /// Step type (e.g., "shell", "echo", "retry", "timeout")
    pub step_type: String,
    /// Command for shell steps
    pub command: Option<String>,
}
