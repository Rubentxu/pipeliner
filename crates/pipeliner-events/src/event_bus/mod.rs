//! Event bus implementation.
//!
//! Provides an in-memory event bus for pub/sub communication.

use async_trait::async_trait;
use dashmap::DashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::types::{AnyEvent, EventEnvelope};

/// Event handler trait
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    fn handle<'a>(
        &'a self,
        event: &'a EventEnvelope,
    ) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>;
}

/// Event bus trait
#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    type Error: std::fmt::Debug + std::fmt::Display;

    async fn publish(&self, event: EventEnvelope) -> Result<(), Self::Error>;

    async fn subscribe(&self, handler: Arc<dyn EventHandler>) -> Result<(), Self::Error>;

    async fn unsubscribe(&self, handler_id: &Uuid) -> Result<(), Self::Error>;
}

/// Local in-memory event bus
pub struct LocalEventBus {
    sender: broadcast::Sender<Arc<EventEnvelope>>,
    handlers: DashMap<Uuid, Arc<dyn EventHandler>>,
}

impl Default for LocalEventBus {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self {
            sender,
            handlers: DashMap::new(),
        }
    }
}

impl LocalEventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe_with_id(&self, handler: Arc<dyn EventHandler>) -> Uuid {
        let id = Uuid::new_v4();
        self.handlers.insert(id, handler);
        id
    }
}

#[async_trait::async_trait]
impl EventBus for LocalEventBus {
    type Error = EventBusError;

    async fn publish(&self, event: EventEnvelope) -> Result<(), Self::Error> {
        let event = Arc::new(event);
        let _ = self.sender.send(event);
        Ok(())
    }

    async fn subscribe(&self, handler: Arc<dyn EventHandler>) -> Result<(), Self::Error> {
        let _id = self.subscribe_with_id(handler);
        Ok(())
    }

    async fn unsubscribe(&self, handler_id: &Uuid) -> Result<(), Self::Error> {
        self.handlers.remove(handler_id);
        Ok(())
    }
}

/// Event bus errors
#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    #[error("Handler not found: {0}")]
    HandlerNotFound(Uuid),

    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),
}

/// Subscription for receiving events
#[derive(Debug)]
pub struct Subscription {
    receiver: broadcast::Receiver<Arc<EventEnvelope>>,
}

impl Subscription {
    pub fn new(receiver: broadcast::Receiver<Arc<EventEnvelope>>) -> Self {
        Self { receiver }
    }

    pub async fn recv(
        &mut self,
    ) -> Result<Arc<EventEnvelope>, tokio::sync::broadcast::error::RecvError> {
        self.receiver.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AnyEvent, EventEnvelope, EventMetadata};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_local_event_bus_publish() {
        let bus = LocalEventBus::new();
        let aggregate_id = Uuid::new_v4();

        let event = EventEnvelope::new(
            AnyEvent::Pipeline(crate::types::PipelineEvent::Created {
                pipeline_id: aggregate_id,
                name: "test".to_string(),
            }),
            EventMetadata::new("test"),
        );

        let result = bus.publish(event).await;
        assert!(result.is_ok());
    }
}
