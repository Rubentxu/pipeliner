//! Prelude module for common imports

// Re-export macros
pub use crate::{
    agent_any, agent_docker, agent_kubernetes, agent_label, echo, parallel, pipeline, post, sh,
    stage, steps, timeout, when,
};

// Re-export all pipeline types with full paths
pub use crate::pipeline::agent::{AgentConfig, AgentType, DockerConfig, KubernetesConfig};
pub use crate::pipeline::errors::{PipelineError, ValidationError};
pub use crate::pipeline::options::{BuildDiscarder, PipelineOptions, Trigger};
pub use crate::pipeline::pipeline_def::{Pipeline, PipelineBuilder};
pub use crate::pipeline::post::PostCondition;
pub use crate::pipeline::stage::{Stage, StageBuilder, WhenCondition};
pub use crate::pipeline::steps::{Step, StepType};
pub use crate::pipeline::types::{PipelineResult, StageResult, Validate};
pub use crate::pipeline::{Environment, Parameters};

// Re-export executor types
pub use crate::executor::{
    ExecutorCapabilities, ExecutorConfig, HealthStatus, LocalExecutor, PipelineContext,
    PipelineExecutor,
};
