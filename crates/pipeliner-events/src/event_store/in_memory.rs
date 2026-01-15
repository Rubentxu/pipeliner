//! Event store implementation.
//!
//! Provides an in-memory event store for development and testing.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{AnyEvent, EventEnvelope, EventMetadata};

/// Event store trait
#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    type Error: std::fmt::Debug + std::fmt::Display;

    async fn append(
        &self,
        aggregate_id: &Uuid,
        events: &[EventEnvelope],
    ) -> Result<(), Self::Error>;

    async fn get_events(&self, aggregate_id: &Uuid) -> Result<Vec<EventEnvelope>, Self::Error>;

    async fn list_aggregates(&self) -> Result<Vec<Uuid>, Self::Error>;
}

/// In-memory event store for development and testing
#[derive(Debug, Default)]
pub struct InMemoryEventStore {
    events: DashMap<Uuid, Vec<EventEnvelope>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: DashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl EventStore for InMemoryEventStore {
    type Error = EventStoreError;

    async fn append(
        &self,
        aggregate_id: &Uuid,
        new_events: &[EventEnvelope],
    ) -> Result<(), Self::Error> {
        let mut events = self.events.entry(*aggregate_id).or_default();
        events.extend_from_slice(new_events);
        Ok(())
    }

    async fn get_events(&self, aggregate_id: &Uuid) -> Result<Vec<EventEnvelope>, Self::Error> {
        if let Some(events) = self.events.get(aggregate_id) {
            Ok(events.clone())
        } else {
            Ok(Vec::new())
        }
    }

    async fn list_aggregates(&self) -> Result<Vec<Uuid>, Self::Error> {
        Ok(self.events.iter().map(|e| *e.key()).collect())
    }
}

/// Event store errors
#[derive(Debug, thiserror::Error)]
pub enum EventStoreError {
    #[error("Concurrency conflict: {0}")]
    ConcurrencyConflict(String),

    #[error("Event not found: {0}")]
    EventNotFound(Uuid),

    #[error("Storage error: {0}")]
    StorageError(String),
}

/// A snapshot of aggregate state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub aggregate_id: Uuid,
    pub aggregate_type: String,
    pub version: u64,
    pub state: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
}

/// Snapshot store trait
#[async_trait::async_trait]
pub trait SnapshotStore: Send + Sync {
    type Error: std::fmt::Debug + std::fmt::Display;

    async fn save(&self, snapshot: &Snapshot) -> Result<(), Self::Error>;

    async fn load(&self, aggregate_id: &Uuid) -> Result<Option<Snapshot>, Self::Error>;
}

/// In-memory snapshot store
#[derive(Debug, Default)]
pub struct InMemorySnapshotStore {
    snapshots: DashMap<Uuid, Snapshot>,
}

impl InMemorySnapshotStore {
    pub fn new() -> Self {
        Self {
            snapshots: DashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl SnapshotStore for InMemorySnapshotStore {
    type Error = SnapshotStoreError;

    async fn save(&self, snapshot: &Snapshot) -> Result<(), Self::Error> {
        self.snapshots
            .insert(snapshot.aggregate_id, snapshot.clone());
        Ok(())
    }

    async fn load(&self, aggregate_id: &Uuid) -> Result<Option<Snapshot>, Self::Error> {
        Ok(self.snapshots.get(aggregate_id).map(|s| s.clone()))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Snapshot store error: {0}")]
pub struct SnapshotStoreError(String);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PipelineEvent;

    #[test]
    fn test_in_memory_event_store_append() {
        let store = InMemoryEventStore::new();
        let aggregate_id = Uuid::new_v4();

        let event = EventEnvelope::new(
            AnyEvent::Pipeline(PipelineEvent::Created {
                pipeline_id: aggregate_id,
                name: "test".to_string(),
            }),
            EventMetadata::new("test"),
        );

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { store.append(&aggregate_id, &[event.clone()]).await });

        assert!(result.is_ok());

        let events = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { store.get_events(&aggregate_id).await.unwrap() });

        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_in_memory_event_store_get_empty() {
        let store = InMemoryEventStore::new();
        let aggregate_id = Uuid::new_v4();

        let events = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { store.get_events(&aggregate_id).await.unwrap() });

        assert!(events.is_empty());
    }

    #[test]
    fn test_in_memory_event_store_list_aggregates() {
        let store = InMemoryEventStore::new();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let event1 = EventEnvelope::new(
            AnyEvent::Pipeline(PipelineEvent::Created {
                pipeline_id: id1,
                name: "test1".to_string(),
            }),
            EventMetadata::new("test"),
        );

        let event2 = EventEnvelope::new(
            AnyEvent::Pipeline(PipelineEvent::Created {
                pipeline_id: id2,
                name: "test2".to_string(),
            }),
            EventMetadata::new("test"),
        );

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            store.append(&id1, &[event1]).await.unwrap();
            store.append(&id2, &[event2]).await.unwrap();
        });

        let aggregates = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { store.list_aggregates().await.unwrap() });

        assert_eq!(aggregates.len(), 2);
    }

    #[test]
    fn test_in_memory_snapshot_store() {
        let store = InMemorySnapshotStore::new();
        let aggregate_id = Uuid::new_v4();

        let snapshot = Snapshot {
            aggregate_id,
            aggregate_type: "Pipeline".to_string(),
            version: 1,
            state: serde_json::json!({"name": "test"}),
            created_at: chrono::Utc::now(),
        };

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { store.save(&snapshot).await });

        assert!(result.is_ok());

        let loaded = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { store.load(&aggregate_id).await.unwrap() });

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().version, 1);
    }
}
