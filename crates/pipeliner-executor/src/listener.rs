//! Event listeners for pipeline execution.
//!
//! This module provides the listener trait and implementations for
//! receiving events during pipeline execution.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::io::Write;
use tracing::{Level, event};

use pipeliner_core::Step;

use crate::{ExecutionContext, ExecutionResult, ExecutionStatus};

/// Execution event types
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// Pipeline started
    PipelineStarted {
        pipeline_name: String,
        execution_id: String,
        start_time: DateTime<Utc>,
    },
    /// Pipeline completed
    PipelineCompleted {
        pipeline_name: String,
        execution_id: String,
        result: ExecutionResult,
        end_time: DateTime<Utc>,
    },
    /// Pipeline failed
    PipelineFailed {
        pipeline_name: String,
        execution_id: String,
        error: String,
        end_time: DateTime<Utc>,
    },
    /// Stage started
    StageStarted {
        stage_name: String,
        execution_id: String,
    },
    /// Stage completed
    StageCompleted {
        stage_name: String,
        status: ExecutionStatus,
        duration: chrono::Duration,
    },
    /// Stage failed
    StageFailed { stage_name: String, error: String },
    /// Step started
    StepStarted {
        stage_name: String,
        step_name: String,
    },
    /// Step completed
    StepCompleted {
        stage_name: String,
        step_name: String,
        status: ExecutionStatus,
        duration: chrono::Duration,
    },
    /// Step failed
    StepFailed {
        stage_name: String,
        step_name: String,
        error: String,
    },
    /// Log output
    LogOutput {
        stage_name: String,
        step_name: String,
        output: String,
    },
    /// Artifact archived
    ArtifactArchived {
        stage_name: String,
        artifact: String,
    },
    /// Stash created
    StashCreated { name: String, path: String },
    /// Stash restored
    StashRestored { name: String, path: String },
}

/// Listener trait for execution events
#[async_trait]
pub trait ExecutionListener: Send + Sync {
    /// Called when an event occurs
    async fn on_event(&self, event: &ExecutionEvent, context: &ExecutionContext);
}

/// Composite listener that combines multiple listeners
#[derive(Default)]
pub struct CompositeListener {
    listeners: Vec<Box<dyn ExecutionListener>>,
}

impl std::fmt::Debug for CompositeListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeListener")
            .field("listeners", &self.listeners.len())
            .finish()
    }
}

impl CompositeListener {
    /// Creates a new composite listener
    #[must_use]
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    /// Adds a listener
    pub fn add<L: ExecutionListener + 'static>(&mut self, listener: L) {
        self.listeners.push(Box::new(listener));
    }
}

#[async_trait]
impl ExecutionListener for CompositeListener {
    async fn on_event(&self, event: &ExecutionEvent, context: &ExecutionContext) {
        for listener in &self.listeners {
            listener.on_event(event, context).await;
        }
    }
}

/// Tracing listener that logs events
#[derive(Debug, Default)]
pub struct TracingListener;

#[async_trait]
impl ExecutionListener for TracingListener {
    async fn on_event(&self, event: &ExecutionEvent, _context: &ExecutionContext) {
        match event {
            ExecutionEvent::PipelineStarted {
                pipeline_name,
                execution_id,
                ..
            } => {
                event!(Level::INFO, pipeline = %pipeline_name, id = %execution_id, "Pipeline started");
            }
            ExecutionEvent::PipelineCompleted {
                pipeline_name,
                result,
                ..
            } => {
                event!(Level::INFO, pipeline = %pipeline_name, status = ?result.status, "Pipeline completed");
            }
            ExecutionEvent::PipelineFailed {
                pipeline_name,
                error,
                ..
            } => {
                event!(Level::ERROR, pipeline = %pipeline_name, error = %error, "Pipeline failed");
            }
            ExecutionEvent::StageStarted { stage_name, .. } => {
                event!(Level::INFO, stage = %stage_name, "Stage started");
            }
            ExecutionEvent::StageCompleted {
                stage_name,
                status,
                duration,
            } => {
                event!(Level::INFO, stage = %stage_name, status = ?status, duration = ?duration, "Stage completed");
            }
            ExecutionEvent::StageFailed { stage_name, error } => {
                event!(Level::ERROR, stage = %stage_name, error = %error, "Stage failed");
            }
            ExecutionEvent::StepStarted {
                stage_name,
                step_name,
            } => {
                event!(Level::DEBUG, stage = %stage_name, step = %step_name, "Step started");
            }
            ExecutionEvent::StepCompleted {
                stage_name,
                step_name,
                status,
                duration,
            } => {
                event!(Level::DEBUG, stage = %stage_name, step = %step_name, status = ?status, duration = ?duration, "Step completed");
            }
            ExecutionEvent::StepFailed {
                stage_name,
                step_name,
                error,
            } => {
                event!(Level::WARN, stage = %stage_name, step = %step_name, error = %error, "Step failed");
            }
            ExecutionEvent::LogOutput {
                stage_name,
                step_name,
                output,
            } => {
                event!(Level::INFO, stage = %stage_name, step = %step_name, output = %output);
            }
            ExecutionEvent::ArtifactArchived {
                stage_name,
                artifact,
            } => {
                event!(Level::INFO, stage = %stage_name, artifact = %artifact, "Artifact archived");
            }
            ExecutionEvent::StashCreated { name, path } => {
                event!(Level::INFO, stash = %name, path = %path, "Stash created");
            }
            ExecutionEvent::StashRestored { name, path } => {
                event!(Level::INFO, stash = %name, path = %path, "Stash restored");
            }
        }
    }
}

/// No-op listener
#[derive(Debug, Default)]
pub struct NoopListener;

#[async_trait]
impl ExecutionListener for NoopListener {
    async fn on_event(&self, _event: &ExecutionEvent, _context: &ExecutionContext) {}
}

/// Buffer listener that stores events in memory
#[derive(Debug, Default)]
pub struct BufferListener {
    events: std::sync::Arc<std::sync::Mutex<Vec<ExecutionEvent>>>,
}

impl BufferListener {
    /// Creates a new buffer listener
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Gets all recorded events
    pub fn get_events(&self) -> Vec<ExecutionEvent> {
        let events = self.events.lock().unwrap();
        events.clone()
    }

    /// Clears all recorded events
    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        events.clear();
    }
}

#[async_trait]
impl ExecutionListener for BufferListener {
    async fn on_event(&self, event: &ExecutionEvent, _context: &ExecutionContext) {
        let mut events = self.events.lock().unwrap();
        events.push(event.clone());
    }
}

/// Text file listener that writes events to a text file
#[derive(Debug)]
pub struct TextFileListener {
    path: std::path::PathBuf,
    file: std::sync::Arc<std::sync::Mutex<Option<std::fs::File>>>,
}

impl TextFileListener {
    /// Creates a new text file listener
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            file: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

#[async_trait]
impl ExecutionListener for TextFileListener {
    async fn on_event(&self, event: &ExecutionEvent, _context: &ExecutionContext) {
        let mut file = self.file.lock().unwrap();
        if file.is_none() {
            let f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .ok();
            *file = f;
        }

        if let Some(f) = file.as_mut() {
            let _ = writeln!(f, "{:?}", event);
        }
    }
}

/// Event emitter for execution events
pub struct EventEmitter {
    listener: std::sync::Arc<std::sync::Mutex<Option<Box<dyn ExecutionListener>>>>,
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EventEmitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventEmitter").finish()
    }
}

impl EventEmitter {
    /// Creates a new event emitter
    #[must_use]
    pub fn new() -> Self {
        Self {
            listener: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Sets the listener
    pub fn set_listener<L: ExecutionListener + 'static>(&self, listener: L) {
        let mut guard = self.listener.lock().unwrap();
        *guard = Some(Box::new(listener));
    }

    /// Emits an event
    pub async fn emit(&self, event: ExecutionEvent, context: &ExecutionContext) {
        let guard = self.listener.lock().unwrap();
        if let Some(listener) = guard.as_ref() {
            listener.on_event(&event, context).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_composite_listener_creation() {
        let listener = CompositeListener::new();
        assert_eq!(listener.listeners.len(), 0);
    }

    #[test]
    fn test_composite_listener_add() {
        let mut listener = CompositeListener::new();
        listener.add(TracingListener);
        assert_eq!(listener.listeners.len(), 1);
    }

    #[test]
    fn test_buffer_listener() {
        let listener = BufferListener::new();
        assert_eq!(listener.get_events().len(), 0);
        listener.clear();
        assert_eq!(listener.get_events().len(), 0);
    }

    #[tokio::test]
    async fn test_buffer_listener_records_events() {
        let listener = Arc::new(BufferListener::new());
        let event = ExecutionEvent::StepCompleted {
            stage_name: "test".to_string(),
            step_name: "step1".to_string(),
            status: ExecutionStatus::Success,
            duration: chrono::Duration::seconds(1),
        };
        let context = ExecutionContext::new();

        listener.on_event(&event, &context).await;
        assert_eq!(listener.get_events().len(), 1);
    }

    #[test]
    fn test_noop_listener() {
        let listener = NoopListener;
        assert!(true); // Just verify it can be created
    }

    #[test]
    fn test_event_emitter() {
        let emitter = EventEmitter::new();
        assert!(emitter.listener.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_event_emitter_with_no_listener() {
        let emitter = EventEmitter::new();
        let event = ExecutionEvent::PipelineStarted {
            pipeline_name: "test".to_string(),
            execution_id: "123".to_string(),
            start_time: Utc::now(),
        };
        let context = ExecutionContext::new();

        // Should not panic even without a listener
        emitter.emit(event, &context).await;
    }
}
