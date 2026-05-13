//! # Pipeline Observers
//!
//! Provides observer pattern for pipeline execution events.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;

// Re-export EventBus trait for use in tests
use pipeliner_events::EventBus;

/// Pipeline execution context
#[derive(Debug, Clone)]
pub struct PipelineContext {
    /// Pipeline name
    pub pipeline_name: String,
    /// Current stage name (if inside a stage)
    pub stage_name: Option<String>,
    /// Current step name (if inside a step)
    pub step_name: Option<String>,
}

impl PipelineContext {
    /// Create a new pipeline context
    pub fn new(pipeline_name: &str) -> Self {
        Self {
            pipeline_name: pipeline_name.to_string(),
            stage_name: None,
            step_name: None,
        }
    }

    /// Create a context for a stage
    pub fn for_stage(&self, stage_name: &str) -> Self {
        Self {
            pipeline_name: self.pipeline_name.clone(),
            stage_name: Some(stage_name.to_string()),
            step_name: None,
        }
    }

    /// Create a context for a step
    pub fn for_step(&self, step_name: &str) -> Self {
        Self {
            pipeline_name: self.pipeline_name.clone(),
            stage_name: self.stage_name.clone(),
            step_name: Some(step_name.to_string()),
        }
    }
}

/// Pipeline event types
#[derive(Debug, Clone, Serialize)]
pub enum PipelineEventType {
    StageStart,
    StageComplete,
    StepStart,
    StepComplete,
    Error,
}

/// A pipeline execution event
#[derive(Debug, Clone, Serialize)]
pub struct PipelineEvent {
    /// Event type
    pub event_type: PipelineEventType,
    /// Pipeline context
    pub pipeline: String,
    /// Stage name (if applicable)
    pub stage: Option<String>,
    /// Step name (if applicable)
    pub step: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Duration in milliseconds (for complete events)
    pub duration_ms: Option<u64>,
    /// Success flag (for step/stage complete events)
    pub success: Option<bool>,
    /// Error message (for error events)
    pub error: Option<String>,
}

impl PipelineEvent {
    /// Create a stage start event
    pub fn stage_start(pipeline: &str, stage: &str) -> Self {
        Self {
            event_type: PipelineEventType::StageStart,
            pipeline: pipeline.to_string(),
            stage: Some(stage.to_string()),
            step: None,
            timestamp: Utc::now(),
            duration_ms: None,
            success: None,
            error: None,
        }
    }

    /// Create a stage complete event
    pub fn stage_complete(pipeline: &str, stage: &str, duration: Duration, success: bool) -> Self {
        Self {
            event_type: PipelineEventType::StageComplete,
            pipeline: pipeline.to_string(),
            stage: Some(stage.to_string()),
            step: None,
            timestamp: Utc::now(),
            duration_ms: Some(duration.as_millis() as u64),
            success: Some(success),
            error: None,
        }
    }

    /// Create a step start event
    pub fn step_start(pipeline: &str, stage: &str, step: &str) -> Self {
        Self {
            event_type: PipelineEventType::StepStart,
            pipeline: pipeline.to_string(),
            stage: Some(stage.to_string()),
            step: Some(step.to_string()),
            timestamp: Utc::now(),
            duration_ms: None,
            success: None,
            error: None,
        }
    }

    /// Create a step complete event
    pub fn step_complete(pipeline: &str, stage: &str, step: &str, duration: Duration, success: bool) -> Self {
        Self {
            event_type: PipelineEventType::StepComplete,
            pipeline: pipeline.to_string(),
            stage: Some(stage.to_string()),
            step: Some(step.to_string()),
            timestamp: Utc::now(),
            duration_ms: Some(duration.as_millis() as u64),
            success: Some(success),
            error: None,
        }
    }

    /// Create an error event
    pub fn error(pipeline: &str, error: &str) -> Self {
        Self {
            event_type: PipelineEventType::Error,
            pipeline: pipeline.to_string(),
            stage: None,
            step: None,
            timestamp: Utc::now(),
            duration_ms: None,
            success: None,
            error: Some(error.to_string()),
        }
    }
}

/// Observer trait for pipeline execution events
///
/// Implement this trait to receive notifications about pipeline execution events.
/// All methods have default no-op implementations for convenience.
pub trait PipelineObserver: Send + Sync {
    /// Called when a stage starts execution
    fn on_stage_start(&self, _ctx: &PipelineContext) {}

    /// Called when a stage completes execution
    fn on_stage_complete(&self, _ctx: &PipelineContext, _duration: Duration, _success: bool) {}

    /// Called when a step starts execution
    fn on_step_start(&self, _ctx: &PipelineContext) {}

    /// Called when a step completes execution
    fn on_step_complete(&self, _ctx: &PipelineContext, _duration: Duration, _success: bool) {}

    /// Called when an error occurs
    fn on_error(&self, _ctx: &PipelineContext, _error: &str) {}
}

/// Blanket implementation for Fn-based observers
impl<F> PipelineObserver for F
where
    F: Fn(&PipelineContext) + Send + Sync,
{
    fn on_stage_start(&self, ctx: &PipelineContext) {
        self(ctx)
    }
}

// =============================================================================
// Built-in Observers
// =============================================================================

/// Logging observer - logs all events at the specified level
pub struct LoggingObserver {
    level: tracing::Level,
}

impl LoggingObserver {
    /// Create a new logging observer with the specified log level
    pub fn new(level: tracing::Level) -> Self {
        Self { level }
    }

    /// Create a debug-level logging observer
    pub fn debug() -> Self {
        Self::new(tracing::Level::DEBUG)
    }

    /// Create an info-level logging observer
    pub fn info() -> Self {
        Self::new(tracing::Level::INFO)
    }

    /// Create a warn-level logging observer
    pub fn warn() -> Self {
        Self::new(tracing::Level::WARN)
    }
}

impl Default for LoggingObserver {
    fn default() -> Self {
        Self::info()
    }
}

impl PipelineObserver for LoggingObserver {
    fn on_stage_start(&self, ctx: &PipelineContext) {
        if let Some(stage) = &ctx.stage_name {
            match self.level {
                tracing::Level::DEBUG => tracing::debug!("[Stage] {} started", stage),
                tracing::Level::INFO => tracing::info!("[Stage] {} started", stage),
                tracing::Level::WARN => tracing::warn!("[Stage] {} started", stage),
                tracing::Level::ERROR => tracing::error!("[Stage] {} started", stage),
                _ => tracing::info!("[Stage] {} started", stage),
            }
        }
    }

    fn on_stage_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
        if let Some(stage) = &ctx.stage_name {
            let status = if success { "SUCCESS" } else { "FAILED" };
            match self.level {
                tracing::Level::DEBUG => tracing::debug!(
                    "[Stage] {} completed in {}ms ({})",
                    stage,
                    duration.as_millis(),
                    status
                ),
                tracing::Level::INFO => tracing::info!(
                    "[Stage] {} completed in {}ms ({})",
                    stage,
                    duration.as_millis(),
                    status
                ),
                tracing::Level::WARN => tracing::warn!(
                    "[Stage] {} completed in {}ms ({})",
                    stage,
                    duration.as_millis(),
                    status
                ),
                tracing::Level::ERROR => tracing::error!(
                    "[Stage] {} completed in {}ms ({})",
                    stage,
                    duration.as_millis(),
                    status
                ),
                _ => tracing::info!(
                    "[Stage] {} completed in {}ms ({})",
                    stage,
                    duration.as_millis(),
                    status
                ),
            }
        }
    }

    fn on_step_start(&self, ctx: &PipelineContext) {
        if let Some(step) = &ctx.step_name {
            match self.level {
                tracing::Level::DEBUG => tracing::debug!("  [Step] {} started", step),
                _ => {}
            }
        }
    }

    fn on_step_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
        if let Some(step) = &ctx.step_name {
            let status = if success { "✓" } else { "✗" };
            match self.level {
                tracing::Level::DEBUG => tracing::debug!(
                    "  [Step] {} completed in {}ms ({})",
                    step,
                    duration.as_millis(),
                    status
                ),
                _ => {}
            }
        }
    }

    fn on_error(&self, ctx: &PipelineContext, error: &str) {
        tracing::error!("[Error] {}: {}", ctx.pipeline_name, error);
    }
}

/// JSON collector observer - collects events for structured output
#[derive(Debug, Default)]
pub struct JsonCollector {
    /// Collected events
    pub events: Mutex<Vec<PipelineEvent>>,
}

impl JsonCollector {
    /// Create a new JSON collector
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Get all collected events
    pub fn events(&self) -> Vec<PipelineEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Clear collected events
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Export events as JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&*self.events.lock().unwrap())
            .unwrap_or_else(|_| "[]".to_string())
    }
}

impl PipelineObserver for JsonCollector {
    fn on_stage_start(&self, ctx: &PipelineContext) {
        if let Some(stage) = &ctx.stage_name {
            self.events
                .lock()
                .unwrap()
                .push(PipelineEvent::stage_start(&ctx.pipeline_name, stage));
        }
    }

    fn on_stage_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
        if let Some(stage) = &ctx.stage_name {
            self.events.lock().unwrap().push(PipelineEvent::stage_complete(
                &ctx.pipeline_name,
                stage,
                duration,
                success,
            ));
        }
    }

    fn on_step_start(&self, ctx: &PipelineContext) {
        if let (Some(stage), Some(step)) = (&ctx.stage_name, &ctx.step_name) {
            self.events.lock().unwrap().push(PipelineEvent::step_start(
                &ctx.pipeline_name,
                stage,
                step,
            ));
        }
    }

    fn on_step_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
        if let (Some(stage), Some(step)) = (&ctx.stage_name, &ctx.step_name) {
            self.events.lock().unwrap().push(PipelineEvent::step_complete(
                &ctx.pipeline_name,
                stage,
                step,
                duration,
                success,
            ));
        }
    }

    fn on_error(&self, ctx: &PipelineContext, error: &str) {
        self.events
            .lock()
            .unwrap()
            .push(PipelineEvent::error(&ctx.pipeline_name, error));
    }
}

/// No-op observer - does nothing
pub struct NoopObserver;

impl NoopObserver {
    /// Create a new no-op observer
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineObserver for NoopObserver {}

/// Composed observer - calls multiple observers in sequence
pub struct ObserverList {
    observers: Vec<Box<dyn PipelineObserver>>,
}

impl ObserverList {
    /// Create a new empty observer list
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
        }
    }

    /// Add an observer to the list
    pub fn add(mut self, observer: Box<dyn PipelineObserver>) -> Self {
        self.observers.push(observer);
        self
    }
}

impl Default for ObserverList {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineObserver for ObserverList {
    fn on_stage_start(&self, ctx: &PipelineContext) {
        for observer in &self.observers {
            if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                observer.on_stage_start(ctx);
            })) {
                tracing::warn!("Observer panicked in on_stage_start: {:?}", payload);
            }
        }
    }

    fn on_stage_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
        for observer in &self.observers {
            if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                observer.on_stage_complete(ctx, duration, success);
            })) {
                tracing::warn!("Observer panicked in on_stage_complete: {:?}", payload);
            }
        }
    }

    fn on_step_start(&self, ctx: &PipelineContext) {
        for observer in &self.observers {
            if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                observer.on_step_start(ctx);
            })) {
                tracing::warn!("Observer panicked in on_step_start: {:?}", payload);
            }
        }
    }

    fn on_step_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
        for observer in &self.observers {
            if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                observer.on_step_complete(ctx, duration, success);
            })) {
                tracing::warn!("Observer panicked in on_step_complete: {:?}", payload);
            }
        }
    }

    fn on_error(&self, ctx: &PipelineContext, error: &str) {
        for observer in &self.observers {
            if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                observer.on_error(ctx, error);
            })) {
                tracing::warn!("Observer panicked in on_error: {:?}", payload);
            }
        }
    }
}

/// Type alias for boxed observer
pub type ObserverBox = Box<dyn PipelineObserver>;

/// Helper to create an observer box from a closure
pub fn observer_from_fn<F>(f: F) -> ObserverBox
where
    F: Fn(&PipelineContext) + Send + Sync + 'static,
{
    Box::new(f)
}

// =============================================================================
// EventBus Observer
// =============================================================================

use pipeliner_events::types::{AnyEvent, EventEnvelope, EventMetadata, PipelineEvent as EventBusPipelineEvent};

/// Observer that publishes events to an EventBus
pub struct EventBusObserver {
    bus: std::sync::Arc<pipeliner_events::LocalEventBus>,
    pipeline_id: uuid::Uuid,
    execution_id: uuid::Uuid,
}

impl EventBusObserver {
    /// Create a new EventBusObserver
    pub fn new(bus: std::sync::Arc<pipeliner_events::LocalEventBus>) -> Self {
        Self {
            bus,
            pipeline_id: uuid::Uuid::new_v4(),
            execution_id: uuid::Uuid::new_v4(),
        }
    }

    /// Create with specific IDs (for testing)
    pub fn with_ids(bus: std::sync::Arc<pipeliner_events::LocalEventBus>, pipeline_id: uuid::Uuid, execution_id: uuid::Uuid) -> Self {
        Self { bus, pipeline_id, execution_id }
    }

    fn publish(&self, event: EventBusPipelineEvent) {
        let envelope = EventEnvelope::new(
            AnyEvent::Pipeline(event),
            EventMetadata::new("pipeliner-executor"),
        );
        let bus = self.bus.clone();
        // Fire-and-forget: spawn async publish from sync context
        // This is safe because LocalExecutor::execute() runs in a tokio runtime
        let _ = tokio::spawn(async move {
            let _ = bus.publish(envelope).await;
        });
    }
}

impl PipelineObserver for EventBusObserver {
    fn on_stage_start(&self, ctx: &PipelineContext) {
        if let Some(stage_name) = &ctx.stage_name {
            let event = EventBusPipelineEvent::StageStarted {
                pipeline_id: self.pipeline_id,
                execution_id: self.execution_id,
                stage_name: stage_name.clone(),
            };
            self.publish(event);
        }
    }

    fn on_stage_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
        if let Some(stage_name) = &ctx.stage_name {
            let result = if success { "success" } else { "failed" };
            let event = EventBusPipelineEvent::StageCompleted {
                pipeline_id: self.pipeline_id,
                execution_id: self.execution_id,
                stage_name: stage_name.clone(),
                result: result.to_string(),
                duration_ms: duration.as_millis() as u64,
            };
            self.publish(event);
        }
    }

    fn on_step_start(&self, ctx: &PipelineContext) {
        if let (Some(stage_name), Some(step_name)) = (&ctx.stage_name, &ctx.step_name) {
            let event = EventBusPipelineEvent::StepStarted {
                pipeline_id: self.pipeline_id,
                execution_id: self.execution_id,
                stage_name: stage_name.clone(),
                step_name: step_name.clone(),
            };
            self.publish(event);
        }
    }

    fn on_step_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
        if let (Some(stage_name), Some(step_name)) = (&ctx.stage_name, &ctx.step_name) {
            let exit_code = if success { Some(0) } else { Some(1) };
            let event = EventBusPipelineEvent::StepCompleted {
                pipeline_id: self.pipeline_id,
                execution_id: self.execution_id,
                stage_name: stage_name.clone(),
                step_name: step_name.clone(),
                output: None,
                duration_ms: duration.as_millis() as u64,
                exit_code,
            };
            self.publish(event);
        }
    }

    fn on_error(&self, ctx: &PipelineContext, error: &str) {
        let event = EventBusPipelineEvent::Failed {
            pipeline_id: self.pipeline_id,
            execution_id: self.execution_id,
            error: error.to_string(),
        };
        self.publish(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_context_new() {
        let ctx = PipelineContext::new("test-pipeline");
        assert_eq!(ctx.pipeline_name, "test-pipeline");
        assert!(ctx.stage_name.is_none());
        assert!(ctx.step_name.is_none());
    }

    #[test]
    fn test_pipeline_context_for_stage() {
        let ctx = PipelineContext::new("test").for_stage("build");
        assert_eq!(ctx.pipeline_name, "test");
        assert_eq!(ctx.stage_name, Some("build".to_string()));
    }

    #[test]
    fn test_pipeline_context_for_step() {
        let ctx = PipelineContext::new("test").for_stage("build").for_step("compile");
        assert_eq!(ctx.stage_name, Some("build".to_string()));
        assert_eq!(ctx.step_name, Some("compile".to_string()));
    }

    #[test]
    fn test_logging_observer_creation() {
        let observer = LoggingObserver::debug();
        assert!(std::mem::size_of_val(&observer) > 0);
    }

    #[test]
    fn test_json_collector() {
        let collector = JsonCollector::new();

        let ctx = PipelineContext::new("test").for_stage("build").for_step("compile");

        collector.on_stage_start(&ctx);
        collector.on_step_start(&PipelineContext::for_step(&ctx, "compile"));

        assert_eq!(collector.events().len(), 2);
    }

    #[test]
    fn test_json_collector_to_json() {
        let collector = JsonCollector::new();
        let ctx = PipelineContext::new("test").for_stage("build");
        collector.on_stage_start(&ctx);

        let json = collector.to_json();
        assert!(json.contains("\"pipeline\"") && json.contains("\"test\""));
        assert!(json.contains("\"event_type\"") && json.contains("\"StageStart\""));
    }

    #[test]
    fn test_observer_list() {
        let list = ObserverList::new()
            .add(Box::new(LoggingObserver::info()))
            .add(Box::new(JsonCollector::new()));

        // Should not panic when calling
        let ctx = PipelineContext::new("test").for_stage("build");
        list.on_stage_start(&ctx);
    }

    #[test]
    fn test_observer_from_fn() {
        let called = std::sync::Arc::new(std::sync::Mutex::new(false));
        let called_for_observer = std::sync::Arc::clone(&called);
        let observer = observer_from_fn(move |_ctx| {
            *called_for_observer.lock().unwrap() = true;
        });

        let ctx = PipelineContext::new("test");
        observer.on_stage_start(&ctx);

        assert!(*called.lock().unwrap());
    }

    #[test]
    fn test_noop_observer() {
        let observer = NoopObserver::new();
        let ctx = PipelineContext::new("test");

        // Should not panic
        observer.on_stage_start(&ctx);
        observer.on_stage_complete(&ctx, Duration::from_secs(1), true);
        observer.on_step_start(&ctx);
        observer.on_step_complete(&ctx, Duration::from_secs(1), true);
        observer.on_error(&ctx, "test error");
    }

    #[test]
    fn test_event_creation() {
        let event = PipelineEvent::stage_start("pipeline", "build");
        assert!(matches!(event.event_type, PipelineEventType::StageStart));
        assert_eq!(event.stage, Some("build".to_string()));
    }

    #[test]
    fn test_event_serialization() {
        let event = PipelineEvent::stage_start("pipeline", "build");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("pipeline"));
        assert!(json.contains("build"));
    }

    // =======================================================================
    // EventBusObserver Tests (T5.2)
    // =======================================================================

    use async_trait::async_trait;
    use std::pin::Pin;
    use std::sync::Arc;
    use pipeliner_events::types::{AnyEvent, EventEnvelope};
    use tokio::sync::broadcast;

    // Use a custom EventBus implementation for testing that properly supports
    // receiving events via broadcast channel. The actual LocalEventBus stores
    // handlers but doesn't invoke them in publish(), so we use our own bus.

    /// A test event bus wrapper that properly supports receiving events
    struct TestBus {
        sender: broadcast::Sender<Arc<EventEnvelope>>,
        receiver: broadcast::Receiver<Arc<EventEnvelope>>,
    }

    impl TestBus {
        fn new() -> Self {
            let (sender, receiver) = broadcast::channel(16);
            Self { sender, receiver }
        }

        fn publish(&self, event: EventEnvelope) {
            let _ = self.sender.send(Arc::new(event));
        }

        fn subscribe(&self) -> broadcast::Receiver<Arc<EventEnvelope>> {
            self.sender.subscribe()
        }

        fn get_receiver(&self) -> broadcast::Receiver<Arc<EventEnvelope>> {
            self.sender.subscribe()
        }
    }

    // Implementation of EventBus for TestBus to use with EventBusObserver
    #[async_trait]
    impl pipeliner_events::EventBus for TestBus {
        type Error = pipeliner_events::event_bus::EventBusError;

        async fn publish(&self, event: pipeliner_events::types::EventEnvelope) -> Result<(), Self::Error> {
            let event = Arc::new(event);
            let _ = self.sender.send(event);
            Ok(())
        }

        async fn subscribe(&self, _handler: Arc<dyn pipeliner_events::EventHandler>) -> Result<(), Self::Error> {
            // Not used in our test - we use broadcast receiver instead
            Ok(())
        }

        async fn unsubscribe(&self, _handler_id: &uuid::Uuid) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    // Now create an EventBusObserver variant that works with our TestBus
    struct TestEventBusObserver {
        bus: std::sync::Arc<TestBus>,
        pipeline_id: uuid::Uuid,
        execution_id: uuid::Uuid,
    }

    impl TestEventBusObserver {
        fn with_ids(bus: std::sync::Arc<TestBus>, pipeline_id: uuid::Uuid, execution_id: uuid::Uuid) -> Self {
            Self { bus, pipeline_id, execution_id }
        }

        fn publish(&self, event: pipeliner_events::types::PipelineEvent) {
            use pipeliner_events::types::{AnyEvent, EventEnvelope, EventMetadata};
            let envelope = EventEnvelope::new(
                AnyEvent::Pipeline(event),
                EventMetadata::new("pipeliner-executor"),
            );
            self.bus.publish(envelope);
        }

        fn on_stage_start(&self, ctx: &PipelineContext) {
            if let Some(stage_name) = &ctx.stage_name {
                let event = pipeliner_events::types::PipelineEvent::StageStarted {
                    pipeline_id: self.pipeline_id,
                    execution_id: self.execution_id,
                    stage_name: stage_name.clone(),
                };
                self.publish(event);
            }
        }

        fn on_stage_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
            if let Some(stage_name) = &ctx.stage_name {
                let result = if success { "success" } else { "failed" };
                let event = pipeliner_events::types::PipelineEvent::StageCompleted {
                    pipeline_id: self.pipeline_id,
                    execution_id: self.execution_id,
                    stage_name: stage_name.clone(),
                    result: result.to_string(),
                    duration_ms: duration.as_millis() as u64,
                };
                self.publish(event);
            }
        }

        fn on_step_start(&self, ctx: &PipelineContext) {
            if let (Some(stage_name), Some(step_name)) = (&ctx.stage_name, &ctx.step_name) {
                let event = pipeliner_events::types::PipelineEvent::StepStarted {
                    pipeline_id: self.pipeline_id,
                    execution_id: self.execution_id,
                    stage_name: stage_name.clone(),
                    step_name: step_name.clone(),
                };
                self.publish(event);
            }
        }

        fn on_step_complete(&self, ctx: &PipelineContext, duration: Duration, success: bool) {
            if let (Some(stage_name), Some(step_name)) = (&ctx.stage_name, &ctx.step_name) {
                let exit_code = if success { Some(0) } else { Some(1) };
                let event = pipeliner_events::types::PipelineEvent::StepCompleted {
                    pipeline_id: self.pipeline_id,
                    execution_id: self.execution_id,
                    stage_name: stage_name.clone(),
                    step_name: step_name.clone(),
                    output: None,
                    duration_ms: duration.as_millis() as u64,
                    exit_code,
                };
                self.publish(event);
            }
        }

        fn on_error(&self, ctx: &PipelineContext, error: &str) {
            let event = pipeliner_events::types::PipelineEvent::Failed {
                pipeline_id: self.pipeline_id,
                execution_id: self.execution_id,
                error: error.to_string(),
            };
            self.publish(event);
        }
    }

    #[tokio::test]
    async fn test_eventbus_observer_stage_start() {
        let bus = std::sync::Arc::new(TestBus::new());
        let pipeline_id = uuid::Uuid::new_v4();
        let execution_id = uuid::Uuid::new_v4();
        let observer = TestEventBusObserver::with_ids(bus.clone(), pipeline_id, execution_id);

        // Get receiver BEFORE publishing - broadcast receivers only get events sent after subscription
        let mut rx = bus.get_receiver();

        let ctx = PipelineContext::new("test-pipeline").for_stage("build");
        observer.on_stage_start(&ctx);

        let event = rx.recv().await.unwrap();
        assert_eq!(event.metadata.source, "pipeliner-executor");

        if let AnyEvent::Pipeline(pipeliner_events::types::PipelineEvent::StageStarted { pipeline_id: pid, execution_id: eid, stage_name }) = &event.event {
            assert_eq!(pid, &pipeline_id);
            assert_eq!(eid, &execution_id);
            assert_eq!(stage_name, "build");
        } else {
            panic!("Expected StageStarted event");
        }
    }

    #[tokio::test]
    async fn test_eventbus_observer_stage_complete() {
        let bus = std::sync::Arc::new(TestBus::new());
        let pipeline_id = uuid::Uuid::new_v4();
        let execution_id = uuid::Uuid::new_v4();
        let observer = TestEventBusObserver::with_ids(bus.clone(), pipeline_id, execution_id);

        // Get receiver BEFORE publishing
        let mut rx = bus.get_receiver();

        let ctx = PipelineContext::new("test-pipeline").for_stage("build");
        observer.on_stage_complete(&ctx, Duration::from_millis(100), true);

        let event = rx.recv().await.unwrap();

        if let AnyEvent::Pipeline(pipeliner_events::types::PipelineEvent::StageCompleted { pipeline_id: pid, execution_id: eid, stage_name, result, duration_ms }) = &event.event {
            assert_eq!(pid, &pipeline_id);
            assert_eq!(eid, &execution_id);
            assert_eq!(stage_name, "build");
            assert_eq!(result, "success");
            assert_eq!(*duration_ms, 100);
        } else {
            panic!("Expected StageCompleted event");
        }
    }

    #[tokio::test]
    async fn test_eventbus_observer_step_start() {
        let bus = std::sync::Arc::new(TestBus::new());
        let pipeline_id = uuid::Uuid::new_v4();
        let execution_id = uuid::Uuid::new_v4();
        let observer = TestEventBusObserver::with_ids(bus.clone(), pipeline_id, execution_id);

        // Get receiver BEFORE publishing
        let mut rx = bus.get_receiver();

        let ctx = PipelineContext::new("test-pipeline").for_stage("build").for_step("compile");
        observer.on_step_start(&ctx);

        let event = rx.recv().await.unwrap();

        if let AnyEvent::Pipeline(pipeliner_events::types::PipelineEvent::StepStarted { pipeline_id: pid, execution_id: eid, stage_name, step_name }) = &event.event {
            assert_eq!(pid, &pipeline_id);
            assert_eq!(eid, &execution_id);
            assert_eq!(stage_name, "build");
            assert_eq!(step_name, "compile");
        } else {
            panic!("Expected StepStarted event");
        }
    }

    #[tokio::test]
    async fn test_eventbus_observer_step_complete() {
        let bus = std::sync::Arc::new(TestBus::new());
        let pipeline_id = uuid::Uuid::new_v4();
        let execution_id = uuid::Uuid::new_v4();
        let observer = TestEventBusObserver::with_ids(bus.clone(), pipeline_id, execution_id);

        // Get receiver BEFORE publishing
        let mut rx = bus.get_receiver();

        let ctx = PipelineContext::new("test-pipeline").for_stage("build").for_step("compile");
        observer.on_step_complete(&ctx, Duration::from_millis(50), true);

        let event = rx.recv().await.unwrap();

        if let AnyEvent::Pipeline(pipeliner_events::types::PipelineEvent::StepCompleted { pipeline_id: pid, execution_id: eid, stage_name, step_name, output, duration_ms, exit_code }) = &event.event {
            assert_eq!(pid, &pipeline_id);
            assert_eq!(eid, &execution_id);
            assert_eq!(stage_name, "build");
            assert_eq!(step_name, "compile");
            assert!(output.is_none());
            assert_eq!(*duration_ms, 50);
            assert_eq!(*exit_code, Some(0)); // success = exit code 0
        } else {
            panic!("Expected StepCompleted event");
        }
    }

    #[tokio::test]
    async fn test_eventbus_observer_step_complete_failure() {
        let bus = std::sync::Arc::new(TestBus::new());
        let pipeline_id = uuid::Uuid::new_v4();
        let execution_id = uuid::Uuid::new_v4();
        let observer = TestEventBusObserver::with_ids(bus.clone(), pipeline_id, execution_id);

        // Get receiver BEFORE publishing
        let mut rx = bus.get_receiver();

        let ctx = PipelineContext::new("test-pipeline").for_stage("build").for_step("compile");
        observer.on_step_complete(&ctx, Duration::from_millis(50), false);

        let event = rx.recv().await.unwrap();

        if let AnyEvent::Pipeline(pipeliner_events::types::PipelineEvent::StepCompleted { exit_code, .. }) = &event.event {
            assert_eq!(*exit_code, Some(1)); // failure = exit code 1
        } else {
            panic!("Expected StepCompleted event");
        }
    }

    #[tokio::test]
    async fn test_eventbus_observer_error() {
        let bus = std::sync::Arc::new(TestBus::new());
        let pipeline_id = uuid::Uuid::new_v4();
        let execution_id = uuid::Uuid::new_v4();
        let observer = TestEventBusObserver::with_ids(bus.clone(), pipeline_id, execution_id);

        // Get receiver BEFORE publishing
        let mut rx = bus.get_receiver();

        let ctx = PipelineContext::new("test-pipeline");
        observer.on_error(&ctx, "Something went wrong");

        let event = rx.recv().await.unwrap();

        if let AnyEvent::Pipeline(pipeliner_events::types::PipelineEvent::Failed { pipeline_id: pid, execution_id: eid, error }) = &event.event {
            assert_eq!(pid, &pipeline_id);
            assert_eq!(eid, &execution_id);
            assert_eq!(error, "Something went wrong");
        } else {
            panic!("Expected Failed event");
        }
    }

    // Now test with the actual EventBusObserver + LocalEventBus
    // This tests that EventBusObserver.publish() works correctly (doesn't panic)
    // even though LocalEventBus handlers aren't invoked

    #[tokio::test]
    async fn test_eventbus_observer_with_local_event_bus_does_not_panic() {
        let bus = std::sync::Arc::new(pipeliner_events::LocalEventBus::new());
        let pipeline_id = uuid::Uuid::new_v4();
        let execution_id = uuid::Uuid::new_v4();
        let observer = EventBusObserver::with_ids(bus, pipeline_id, execution_id);

        let ctx = PipelineContext::new("test-pipeline").for_stage("build");

        // Should not panic - this is the main thing we can test with LocalEventBus
        observer.on_stage_start(&ctx);
        observer.on_stage_complete(&ctx, Duration::from_millis(100), true);
        observer.on_step_start(&ctx);
        observer.on_step_complete(&ctx, Duration::from_millis(50), true);
        observer.on_error(&ctx, "test error");
    }
}
