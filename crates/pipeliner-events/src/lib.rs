//! # Pipeliner Events
//!
//! Event sourcing infrastructure for Pipeliner pipeline execution.
//!
//! ## Architecture
//!
//! The events crate provides:
//!
//! - `event_store`: Persistent storage for events
//! - `event_bus`: Pub/sub communication for event distribution
//! - `types`: Base event types and domain-specific events
//!
//! ## Example
//!
//! ```rust,ignore
//! use pipeliner_events::{LocalEventBus, InMemoryEventStore, PipelineEvent};
//!
//! let bus = LocalEventBus::new();
//! let store = InMemoryEventStore::new();
//! ```

#![warn(missing_docs)]
#![warn(unused)]

pub mod event_bus;
pub mod event_store;
pub mod types;

pub use event_bus::{EventBus, EventHandler, LocalEventBus};
pub use event_store::{EventStore, InMemoryEventStore, InMemorySnapshotStore, SnapshotStore};
pub use types::{
    AnyEvent, EventEnvelope, EventMetadata, InfrastructureEvent, PipelineEvent, WorkerEvent,
};

/// Event errors
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct EventsError(#[from] EventsErrorKind);

#[derive(Debug, thiserror::Error)]
pub enum EventsErrorKind {
    #[error("event store error: {0}")]
    EventStore(#[from] crate::event_store::EventStoreError),

    #[error("event bus error: {0}")]
    EventBus(#[from] crate::event_bus::EventBusError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for events operations
pub type EventsResult<T = ()> = Result<T, EventsError>;

#[cfg(test)]
mod tests {
    use crate::types::{
        EventEnvelope, EventMetadata, InfrastructureEvent, PipelineEvent, WorkerEvent,
    };
    use uuid::Uuid;

    #[test]
    fn test_pipeline_event_created() {
        let event = PipelineEvent::Created {
            pipeline_id: Uuid::new_v4(),
            name: "test-pipeline".to_string(),
        };
        assert_eq!(event.event_type(), "PipelineCreated");
    }

    #[test]
    fn test_worker_event_job_started() {
        let event = WorkerEvent::JobStarted {
            worker_id: "worker-1".to_string(),
            job_id: Uuid::new_v4(),
        };
        assert_eq!(event.event_type(), "JobStarted");
    }

    #[test]
    fn test_infrastructure_event_container() {
        let event = InfrastructureEvent::ContainerStarted {
            container_id: "container-123".to_string(),
        };
        assert_eq!(event.event_type(), "ContainerStarted");
    }
}
