//! Event store module.

pub mod in_memory;

pub use in_memory::{
    EventStore, EventStoreError, InMemoryEventStore, InMemorySnapshotStore, SnapshotStore,
    SnapshotStoreError,
};
