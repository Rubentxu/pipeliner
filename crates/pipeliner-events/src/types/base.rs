//! Event types for the Pipeliner event sourcing system.
//!
//! This module defines the base event types and specific events for
//! pipelines, workers, and infrastructure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export structure types from pipeliner-core for backward compatibility.
// These were originally defined here but moved to pipeliner-core so that
// Pipeline::structure() can be implemented without circular dependencies.
pub use pipeliner_core::structure::{PipelineStructure, StageStructure, StepStructure};

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub source: String,
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            correlation_id: None,
            causation_id: None,
            source: "pipeliner".to_string(),
        }
    }
}

impl EventMetadata {
    pub fn new(source: &str) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            correlation_id: None,
            causation_id: None,
            source: source.to_string(),
        }
    }

    pub fn with_correlation(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn with_causation(mut self, causation_id: Uuid) -> Self {
        self.causation_id = Some(causation_id);
        self
    }
}

/// All events enum for generic handling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum AnyEvent {
    Pipeline(PipelineEvent),
    Worker(WorkerEvent),
    Infrastructure(InfrastructureEvent),
}

impl AnyEvent {
    pub fn event_type(&self) -> &str {
        match self {
            Self::Pipeline(e) => e.event_type(),
            Self::Worker(e) => e.event_type(),
            Self::Infrastructure(e) => e.event_type(),
        }
    }

    pub fn aggregate_id(&self) -> Option<&Uuid> {
        match self {
            Self::Pipeline(e) => Some(e.aggregate_id()),
            Self::Worker(_) => None,
            Self::Infrastructure(_) => None,
        }
    }
}

/// Base event envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event: AnyEvent,
    pub metadata: EventMetadata,
}

impl EventEnvelope {
    pub fn new(event: AnyEvent, metadata: EventMetadata) -> Self {
        Self { event, metadata }
    }
}

/// Pipeline events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineEvent {
    /// Pipeline declaration with full structure for external visualization.
    /// Emitted BEFORE execution starts so consumers can project the graph.
    PipelineDecl {
        pipeline_id: Uuid,
        execution_id: Uuid,
        /// Pipeline name
        name: String,
        /// Serialized pipeline structure (stages, steps, conditions, DAG)
        structure: PipelineStructure,
    },
    Created {
        pipeline_id: Uuid,
        name: String,
    },
    Started {
        pipeline_id: Uuid,
        execution_id: Uuid,
        stage: String,
    },
    StageStarted {
        pipeline_id: Uuid,
        execution_id: Uuid,
        stage_name: String,
    },
    StageCompleted {
        pipeline_id: Uuid,
        execution_id: Uuid,
        stage_name: String,
        result: String,
        /// Duration in milliseconds. [ADDITIVE]
        duration_ms: u64,
    },
    StepStarted {
        pipeline_id: Uuid,
        execution_id: Uuid,
        stage_name: String,
        step_name: String,
    },
    StepCompleted {
        pipeline_id: Uuid,
        execution_id: Uuid,
        stage_name: String,
        step_name: String,
        output: Option<String>,
        /// Duration in milliseconds. [ADDITIVE]
        duration_ms: u64,
        /// Exit code of the command. [ADDITIVE]
        exit_code: Option<i32>,
    },
    Completed {
        pipeline_id: Uuid,
        execution_id: Uuid,
        result: String,
    },
    Failed {
        pipeline_id: Uuid,
        execution_id: Uuid,
        error: String,
    },
    Cancelled {
        pipeline_id: Uuid,
        execution_id: Uuid,
        reason: String,
    },
}

impl PipelineEvent {
    pub fn event_type(&self) -> &str {
        match self {
            Self::PipelineDecl { .. } => "PipelineDecl",
            Self::Created { .. } => "PipelineCreated",
            Self::Started { .. } => "PipelineStarted",
            Self::StageStarted { .. } => "StageStarted",
            Self::StageCompleted { .. } => "StageCompleted",
            Self::StepStarted { .. } => "StepStarted",
            Self::StepCompleted { .. } => "StepCompleted",
            Self::Completed { .. } => "PipelineCompleted",
            Self::Failed { .. } => "PipelineFailed",
            Self::Cancelled { .. } => "PipelineCancelled",
        }
    }

    pub fn aggregate_id(&self) -> &Uuid {
        match self {
            Self::PipelineDecl { pipeline_id, .. } => pipeline_id,
            Self::Created { pipeline_id, .. } => pipeline_id,
            Self::Started { pipeline_id, .. } => pipeline_id,
            Self::StageStarted { pipeline_id, .. } => pipeline_id,
            Self::StageCompleted { pipeline_id, .. } => pipeline_id,
            Self::StepStarted { pipeline_id, .. } => pipeline_id,
            Self::StepCompleted { pipeline_id, .. } => pipeline_id,
            Self::Completed { pipeline_id, .. } => pipeline_id,
            Self::Failed { pipeline_id, .. } => pipeline_id,
            Self::Cancelled { pipeline_id, .. } => pipeline_id,
        }
    }
}

/// Worker events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerEvent {
    WorkerStarted {
        worker_id: String,
    },
    WorkerStopped {
        worker_id: String,
        reason: String,
    },
    JobAssigned {
        worker_id: String,
        job_id: Uuid,
    },
    JobStarted {
        worker_id: String,
        job_id: Uuid,
    },
    JobCompleted {
        worker_id: String,
        job_id: Uuid,
        result: String,
    },
    JobFailed {
        worker_id: String,
        job_id: Uuid,
        error: String,
    },
    Heartbeat {
        worker_id: String,
    },
}

impl WorkerEvent {
    pub fn event_type(&self) -> &str {
        match self {
            Self::WorkerStarted { .. } => "WorkerStarted",
            Self::WorkerStopped { .. } => "WorkerStopped",
            Self::JobAssigned { .. } => "JobAssigned",
            Self::JobStarted { .. } => "JobStarted",
            Self::JobCompleted { .. } => "JobCompleted",
            Self::JobFailed { .. } => "JobFailed",
            Self::Heartbeat { .. } => "WorkerHeartbeat",
        }
    }
}

/// Infrastructure events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InfrastructureEvent {
    ContainerCreated {
        container_id: String,
        image: String,
    },
    ContainerStarted {
        container_id: String,
    },
    ContainerStopped {
        container_id: String,
        exit_code: i32,
    },
    ContainerFailed {
        container_id: String,
        error: String,
    },
    NetworkCreated {
        network_id: String,
    },
    NetworkRemoved {
        network_id: String,
    },
    ImagePulled {
        image: String,
    },
}

impl InfrastructureEvent {
    pub fn event_type(&self) -> &str {
        match self {
            Self::ContainerCreated { .. } => "ContainerCreated",
            Self::ContainerStarted { .. } => "ContainerStarted",
            Self::ContainerStopped { .. } => "ContainerStopped",
            Self::ContainerFailed { .. } => "ContainerFailed",
            Self::NetworkCreated { .. } => "NetworkCreated",
            Self::NetworkRemoved { .. } => "NetworkRemoved",
            Self::ImagePulled { .. } => "ImagePulled",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_metadata_new() {
        let metadata = EventMetadata::new("test");
        assert_eq!(metadata.source, "test");
        assert!(metadata.event_id.is_nil() == false);
        assert!(metadata.timestamp <= Utc::now());
    }

    #[test]
    fn test_event_metadata_with_correlation() {
        let correlation_id = Uuid::new_v4();
        let metadata = EventMetadata::new("test").with_correlation(correlation_id);
        assert_eq!(metadata.correlation_id, Some(correlation_id));
    }

    #[test]
    fn test_pipeline_event_type() {
        let event = PipelineEvent::PipelineDecl {
            pipeline_id: Uuid::new_v4(),
            execution_id: Uuid::new_v4(),
            name: "test".to_string(),
            structure: PipelineStructure {
                stages: vec![StageStructure {
                    name: "Build".to_string(),
                    steps: vec![StepStructure {
                        name: Some("step1".to_string()),
                        step_type: "shell".to_string(),
                        command: Some("echo hello".to_string()),
                    }],
                    has_parallel: false,
                    has_matrix: false,
                    when_condition: None,
                }],
            },
        };
        assert_eq!(event.event_type(), "PipelineDecl");

        let event = PipelineEvent::Created {
            pipeline_id: Uuid::new_v4(),
            name: "test".to_string(),
        };
        assert_eq!(event.event_type(), "PipelineCreated");

        let event = PipelineEvent::Completed {
            pipeline_id: Uuid::new_v4(),
            execution_id: Uuid::new_v4(),
            result: "SUCCESS".to_string(),
        };
        assert_eq!(event.event_type(), "PipelineCompleted");
    }

    #[test]
    fn test_any_event_pipeline() {
        let event = AnyEvent::Pipeline(PipelineEvent::PipelineDecl {
            pipeline_id: Uuid::new_v4(),
            execution_id: Uuid::new_v4(),
            name: "test".to_string(),
            structure: PipelineStructure { stages: vec![] },
        });
        assert_eq!(event.event_type(), "PipelineDecl");
        assert!(event.aggregate_id().is_some());
    }

    #[test]
    fn test_any_event_worker() {
        let event = AnyEvent::Worker(WorkerEvent::JobStarted {
            worker_id: "worker-1".to_string(),
            job_id: Uuid::new_v4(),
        });
        assert_eq!(event.event_type(), "JobStarted");
        assert!(event.aggregate_id().is_none());
    }

    #[test]
    fn test_worker_event_type() {
        let event = WorkerEvent::JobStarted {
            worker_id: "worker-1".to_string(),
            job_id: Uuid::new_v4(),
        };
        assert_eq!(event.event_type(), "JobStarted");
    }

    #[test]
    fn test_infrastructure_event_type() {
        let event = InfrastructureEvent::ContainerStarted {
            container_id: "abc123".to_string(),
        };
        assert_eq!(event.event_type(), "ContainerStarted");
    }
}
