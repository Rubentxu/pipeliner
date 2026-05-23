//! # Events Module
//!
//! Event system for pipeline execution tracking.
//!
//! This module provides:
//! - [`PipelineEvent`] - All possible pipeline events
//! - [`EventEmitter`] - Trait for publishing events
//! - [`EventSubscription`] - Subscribe/unsubscribe mechanism
//! - [`JsonlEventWriter`] - Write events to JSON Lines file
//! - Buffered event stream
//!
//! ## Event Flow
//!
//! ```ignore
//! Pipeline Started
//!   └── Stage Started: "build"
//!         └── Step Started: "compile"
//!         └── Step Completed: "compile" (success)
//!   └── Stage Completed: "build" (success)
//!   └── Stage Started: "test"
//!         └── Step Started: "test-unit"
//!         └── Step Completed: "test-unit" (success)
//!   └── Stage Completed: "test" (success)
//! Pipeline Completed (success)
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pipeliner_runtime::{LocalExecutor, EventEmitter, PipelineEvent};
//!
//! // Create executor with event handler
//! let executor = LocalExecutor::new();
//! executor.on(|event| {
//!     println!("Pipeline event: {:?}", event);
//! });
//! ```
//!
//! ## JSON Lines Event Writer
//!
//! ```rust,ignore
//! use pipeliner_runtime::events::JsonlEventWriter;
//! use uuid::Uuid;
//!
//! let run_id = Uuid::new_v4();
//! let writer = JsonlEventWriter::new(run_id, ".pipeliner/runs").unwrap();
//! writer.emit(PipelineEvent::Started { ... });
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use parking_lot::RwLock;
use uuid::Uuid;

/// Pipeline execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum PipelineEvent {
    /// Pipeline execution started
    Started {
        run_id: Uuid,
        pipeline_id: Uuid,
        pipeline_name: Option<String>,
        started_at: DateTime<Utc>,
    },
    /// Stage execution started
    StageStarted {
        run_id: Uuid,
        pipeline_id: Uuid,
        stage_id: String,
        stage_name: String,
        started_at: DateTime<Utc>,
    },
    /// Stage execution completed
    StageCompleted {
        run_id: Uuid,
        pipeline_id: Uuid,
        stage_id: String,
        stage_name: String,
        completed_at: DateTime<Utc>,
        success: bool,
        exit_code: Option<i32>,
    },
    /// Step execution started
    StepStarted {
        run_id: Uuid,
        pipeline_id: Uuid,
        stage_id: String,
        step_index: usize,
        step_type: String,
        started_at: DateTime<Utc>,
    },
    /// Step execution completed
    StepCompleted {
        run_id: Uuid,
        pipeline_id: Uuid,
        stage_id: String,
        step_index: usize,
        step_type: String,
        completed_at: DateTime<Utc>,
        success: bool,
        exit_code: Option<i32>,
        duration_secs: f64,
    },
    /// Pipeline execution completed
    Completed {
        run_id: Uuid,
        pipeline_id: Uuid,
        completed_at: DateTime<Utc>,
        success: bool,
        total_duration_secs: f64,
    },
    /// Pipeline execution failed
    Failed {
        run_id: Uuid,
        pipeline_id: Uuid,
        reason: String,
        failed_at: DateTime<Utc>,
        total_duration_secs: f64,
    },
    /// Retry attempt for a stage
    StageRetry {
        run_id: Uuid,
        pipeline_id: Uuid,
        stage_id: String,
        attempt: u32,
        max_attempts: u32,
    },
    /// Pipeline execution was cancelled
    Cancelled {
        run_id: Uuid,
        pipeline_id: Uuid,
        reason: Option<String>,
        cancelled_at: DateTime<Utc>,
    },
}

impl PipelineEvent {
    /// Returns the run ID associated with this event.
    pub fn run_id(&self) -> Uuid {
        match self {
            PipelineEvent::Started { run_id, .. } => *run_id,
            PipelineEvent::StageStarted { run_id, .. } => *run_id,
            PipelineEvent::StageCompleted { run_id, .. } => *run_id,
            PipelineEvent::StepStarted { run_id, .. } => *run_id,
            PipelineEvent::StepCompleted { run_id, .. } => *run_id,
            PipelineEvent::Completed { run_id, .. } => *run_id,
            PipelineEvent::Failed { run_id, .. } => *run_id,
            PipelineEvent::StageRetry { run_id, .. } => *run_id,
            PipelineEvent::Cancelled { run_id, .. } => *run_id,
        }
    }

    /// Returns the pipeline ID associated with this event.
    pub fn pipeline_id(&self) -> Uuid {
        match self {
            PipelineEvent::Started { pipeline_id, .. } => *pipeline_id,
            PipelineEvent::StageStarted { pipeline_id, .. } => *pipeline_id,
            PipelineEvent::StageCompleted { pipeline_id, .. } => *pipeline_id,
            PipelineEvent::StepStarted { pipeline_id, .. } => *pipeline_id,
            PipelineEvent::StepCompleted { pipeline_id, .. } => *pipeline_id,
            PipelineEvent::Completed { pipeline_id, .. } => *pipeline_id,
            PipelineEvent::Failed { pipeline_id, .. } => *pipeline_id,
            PipelineEvent::StageRetry { pipeline_id, .. } => *pipeline_id,
            PipelineEvent::Cancelled { pipeline_id, .. } => *pipeline_id,
        }
    }

    /// Returns true if this event indicates a terminal state (Completed, Failed, or Cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(self, PipelineEvent::Completed { .. } | PipelineEvent::Failed { .. } | PipelineEvent::Cancelled { .. })
    }
}

/// Event emitter trait for publishing pipeline events.
///
/// Implement this trait to receive pipeline events in your own code.
/// Most users will use [`LocalExecutor::subscribe`] instead.
pub trait EventEmitter: Send + Sync {
    /// Emit a pipeline event.
    fn emit(&self, event: PipelineEvent);

    /// Clone as boxed trait object.
    fn box_clone(&self) -> Box<dyn EventEmitter>;
}

impl Clone for Box<dyn EventEmitter> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// A simple event emitter that calls a callback function.
pub struct CallbackEmitter {
    callback: Arc<dyn Fn(PipelineEvent) + Send + Sync>,
}

impl CallbackEmitter {
    /// Creates a new callback emitter with the given callback.
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(PipelineEvent) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }
}

impl EventEmitter for CallbackEmitter {
    fn emit(&self, event: PipelineEvent) {
        (self.callback)(event);
    }

    fn box_clone(&self) -> Box<dyn EventEmitter> {
        Box::new(CallbackEmitter {
            callback: self.callback.clone(),
        })
    }
}

/// A buffered event emitter that stores events in memory.
#[derive(Clone)]
pub struct BufferedEmitter {
    events: Arc<RwLock<Vec<PipelineEvent>>>,
}

impl BufferedEmitter {
    /// Creates a new buffered emitter.
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Returns a copy of all events collected so far.
    pub fn events(&self) -> Vec<PipelineEvent> {
        self.events.read().clone()
    }

    /// Clears all buffered events.
    pub fn clear(&self) {
        self.events.write().clear();
    }

    /// Returns the number of buffered events.
    pub fn len(&self) -> usize {
        self.events.read().len()
    }

    /// Returns true if there are no buffered events.
    pub fn is_empty(&self) -> bool {
        self.events.read().is_empty()
    }
}

impl Default for BufferedEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventEmitter for BufferedEmitter {
    fn emit(&self, event: PipelineEvent) {
        self.events.write().push(event);
    }

    fn box_clone(&self) -> Box<dyn EventEmitter> {
        Box::new(BufferedEmitter {
            events: self.events.clone(),
        })
    }
}

/// A subscription to pipeline events.
///
/// Created by [`LocalExecutor::subscribe`] and automatically unsubscribes when dropped.
pub struct EventSubscription {
    emitter: Box<dyn EventEmitter>,
    events: Arc<RwLock<Vec<PipelineEvent>>>,
}

impl EventSubscription {
    /// Returns a copy of all events received so far.
    pub fn events(&self) -> Vec<PipelineEvent> {
        self.events.read().clone()
    }

    /// Clears all received events.
    pub fn clear(&self) {
        self.events.write().clear();
    }

    /// Returns the number of received events.
    pub fn len(&self) -> usize {
        self.events.read().len()
    }

    /// Returns true if no events have been received.
    pub fn is_empty(&self) -> bool {
        self.events.read().is_empty()
    }
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        // The subscription is automatically unsubscribed when dropped
        // because the callback is removed from the executor's subscriber list
    }
}

/// Creates a combined emitter that broadcasts to multiple emitters.
pub struct MultiEmitter {
    emitters: Arc<RwLock<Vec<Box<dyn EventEmitter>>>>,
}

impl MultiEmitter {
    /// Creates a new multi-emitter.
    pub fn new() -> Self {
        Self {
            emitters: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Adds an emitter to the broadcast list.
    pub fn add(&self, emitter: Box<dyn EventEmitter>) {
        self.emitters.write().push(emitter);
    }

    /// Removes all emitters.
    pub fn clear(&self) {
        self.emitters.write().clear();
    }
}

impl Default for MultiEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventEmitter for MultiEmitter {
    fn emit(&self, event: PipelineEvent) {
        let emitters = self.emitters.read().clone();
        for emitter in emitters {
            emitter.emit(event.clone());
        }
    }

    fn box_clone(&self) -> Box<dyn EventEmitter> {
        Box::new(MultiEmitter {
            emitters: self.emitters.clone(),
        })
    }
}

/// Event writer that writes events to a JSON Lines file.
///
/// Each event is serialized to JSON and written as a separate line.
/// The file is located at `<base_dir>/runs/<run_id>/events.jsonl`.
///
/// ## Example
///
/// ```rust,ignore
/// use pipeliner_runtime::events::{JsonlEventWriter, PipelineEvent};
/// use uuid::Uuid;
///
/// let run_id = Uuid::new_v4();
/// let writer = JsonlEventWriter::new(run_id, ".pipeliner/runs").unwrap();
/// writer.emit(PipelineEvent::Started { run_id, pipeline_id, ... });
/// ```
pub struct JsonlEventWriter {
    run_id: Uuid,
    writer: Arc<Mutex<BufWriter<File>>>,
    file_path: PathBuf,
}

impl JsonlEventWriter {
    /// Creates a new JSON Lines event writer for the given run ID.
    ///
    /// # Arguments
    ///
    /// * `run_id` - Unique identifier for this run
    /// * `base_dir` - Base directory for run artifacts (e.g., ".pipeliner/runs")
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or the file cannot be opened.
    pub fn new(run_id: Uuid, base_dir: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let base_dir = base_dir.into();
        let run_dir = base_dir.join("runs").join(run_id.to_string());

        // Create the run directory
        fs::create_dir_all(&run_dir)?;

        let file_path = run_dir.join("events.jsonl");
        let file = File::create(&file_path)?;
        let writer = BufWriter::new(file);

        Ok(Self {
            run_id,
            writer: Arc::new(Mutex::new(writer)),
            file_path,
        })
    }

    /// Returns the run ID for this writer.
    pub fn run_id(&self) -> Uuid {
        self.run_id
    }

    /// Returns the path to the events file.
    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    /// Flushes any buffered data to disk.
    pub fn flush(&self) -> Result<(), std::io::Error> {
        match self.writer.lock().as_mut() {
            Ok(writer) => writer.flush(),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "lock poisoned"
            )),
        }
    }
}

impl EventEmitter for JsonlEventWriter {
    fn emit(&self, event: PipelineEvent) {
        let json = serde_json::to_string(&event).expect("event serialization failed");
        // Use match to handle the Result properly
        match self.writer.lock().as_mut() {
            Ok(writer) => {
                // Ignore write errors in emit - they'll surface on flush or drop
                let _ = writeln!(writer, "{}", json);
            }
            Err(_) => {
                // Poisoned lock - ignore
            }
        }
    }

    fn box_clone(&self) -> Box<dyn EventEmitter> {
        // JsonlEventWriter cannot be cloned because it wraps a File
        // Return a no-op emitter for boxed clones
        Box::new(JsonlEventWriterNoop { run_id: self.run_id })
    }
}

/// A no-op event emitter for boxed clones of JsonlEventWriter.
/// This is used because JsonlEventWriter cannot be truly cloned
/// (it wraps a File handle).
struct JsonlEventWriterNoop {
    run_id: Uuid,
}

impl EventEmitter for JsonlEventWriterNoop {
    fn emit(&self, _event: PipelineEvent) {
        // No-op: events are not written anywhere
    }

    fn box_clone(&self) -> Box<dyn EventEmitter> {
        Box::new(Self {
            run_id: self.run_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_event_started() {
        let run_id = Uuid::new_v4();
        let pipeline_id = Uuid::new_v4();
        let event = PipelineEvent::Started {
            run_id,
            pipeline_id,
            pipeline_name: Some("test-pipeline".to_string()),
            started_at: Utc::now(),
        };
        assert!(!event.is_terminal());
        assert!(matches!(event, PipelineEvent::Started { .. }));
        assert_eq!(event.run_id(), run_id);
        assert_eq!(event.pipeline_id(), pipeline_id);
    }

    #[test]
    fn test_pipeline_event_completed() {
        let run_id = Uuid::new_v4();
        let pipeline_id = Uuid::new_v4();
        let event = PipelineEvent::Completed {
            run_id,
            pipeline_id,
            completed_at: Utc::now(),
            success: true,
            total_duration_secs: 10.0,
        };
        assert!(event.is_terminal());
        assert_eq!(event.run_id(), run_id);
    }

    #[test]
    fn test_pipeline_event_failed() {
        let run_id = Uuid::new_v4();
        let pipeline_id = Uuid::new_v4();
        let event = PipelineEvent::Failed {
            run_id,
            pipeline_id,
            reason: "Stage failed".to_string(),
            failed_at: Utc::now(),
            total_duration_secs: 5.0,
        };
        assert!(event.is_terminal());
        assert_eq!(event.run_id(), run_id);
    }

    #[test]
    fn test_pipeline_event_pipeline_id() {
        let run_id = Uuid::new_v4();
        let pipeline_id = Uuid::new_v4();
        let event = PipelineEvent::Started {
            run_id,
            pipeline_id,
            pipeline_name: None,
            started_at: Utc::now(),
        };
        assert_eq!(event.pipeline_id(), pipeline_id);
        assert_eq!(event.run_id(), run_id);
    }

    #[test]
    fn test_callback_emitter() {
        let events = Arc::new(RwLock::new(Vec::new()));
        let events_clone = events.clone();

        let emitter = CallbackEmitter::new(move |event| {
            events_clone.write().push(event);
        });

        let run_id = Uuid::new_v4();
        let test_event = PipelineEvent::Started {
            run_id,
            pipeline_id: Uuid::new_v4(),
            pipeline_name: Some("test".to_string()),
            started_at: Utc::now(),
        };

        emitter.emit(test_event.clone());

        let captured = events.read();
        assert_eq!(captured.len(), 1);
        assert!(matches!(captured[0], PipelineEvent::Started { .. }));
    }

    #[test]
    fn test_buffered_emitter() {
        let emitter = BufferedEmitter::new();
        let run_id = Uuid::new_v4();
        let pipeline_id = Uuid::new_v4();

        let event1 = PipelineEvent::Started {
            run_id,
            pipeline_id,
            pipeline_name: Some("test".to_string()),
            started_at: Utc::now(),
        };

        let event2 = PipelineEvent::Completed {
            run_id,
            pipeline_id,
            completed_at: Utc::now(),
            success: true,
            total_duration_secs: 1.0,
        };

        emitter.emit(event1.clone());
        emitter.emit(event2.clone());

        assert_eq!(emitter.len(), 2);
        assert!(!emitter.is_empty());

        let events = emitter.events();
        assert_eq!(events.len(), 2);

        emitter.clear();
        assert!(emitter.is_empty());
    }

    #[test]
    fn test_multi_emitter() {
        let emitter1 = BufferedEmitter::new();
        let emitter2 = BufferedEmitter::new();

        let multi = MultiEmitter::new();
        multi.add(Box::new(emitter1.clone()));
        multi.add(Box::new(emitter2.clone()));

        let run_id = Uuid::new_v4();
        let event = PipelineEvent::Started {
            run_id,
            pipeline_id: Uuid::new_v4(),
            pipeline_name: Some("test".to_string()),
            started_at: Utc::now(),
        };

        multi.emit(event);

        // Both emitters should have received the event
        assert_eq!(emitter1.len(), 1);
        assert_eq!(emitter2.len(), 1);
    }

    #[test]
    fn test_event_subscription() {
        let emitter = BufferedEmitter::new();
        let subscription = EventSubscription {
            emitter: Box::new(emitter.clone()),
            events: emitter.events.clone(),
        };

        let event = PipelineEvent::Started {
            run_id: Uuid::new_v4(),
            pipeline_id: Uuid::new_v4(),
            pipeline_name: Some("test".to_string()),
            started_at: Utc::now(),
        };

        subscription.emitter.emit(event);

        assert_eq!(subscription.len(), 1);
        assert!(!subscription.is_empty());
    }

    #[test]
    fn test_event_serialization() {
        let run_id = Uuid::new_v4();
        let event = PipelineEvent::Started {
            run_id,
            pipeline_id: Uuid::new_v4(),
            pipeline_name: Some("test-pipeline".to_string()),
            started_at: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("started"));
        assert!(json.contains("test-pipeline"));

        let parsed: PipelineEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, PipelineEvent::Started { .. }));
    }

    #[test]
    fn test_step_event_types() {
        let run_id = Uuid::new_v4();
        let event = PipelineEvent::StepStarted {
            run_id,
            pipeline_id: Uuid::new_v4(),
            stage_id: "build".to_string(),
            step_index: 0,
            step_type: "shell".to_string(),
            started_at: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("step_started"));

        let run_id = Uuid::new_v4();
        let event = PipelineEvent::StepCompleted {
            run_id,
            pipeline_id: Uuid::new_v4(),
            stage_id: "build".to_string(),
            step_index: 0,
            step_type: "shell".to_string(),
            completed_at: Utc::now(),
            success: true,
            exit_code: Some(0),
            duration_secs: 1.5,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("step_completed"));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_stage_retry_event() {
        let run_id = Uuid::new_v4();
        let event = PipelineEvent::StageRetry {
            run_id,
            pipeline_id: Uuid::new_v4(),
            stage_id: "build".to_string(),
            attempt: 2,
            max_attempts: 3,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("stage_retry"));
        assert!(json.contains("\"attempt\":2"));
        assert!(json.contains("\"max_attempts\":3"));
    }

    #[test]
    fn test_jsonl_event_writer() {
        use std::io::Read;

        let run_id = Uuid::new_v4();
        let temp_dir = std::env::temp_dir();
        let base_dir = temp_dir.join("pipeliner_test_jsonl").join(run_id.to_string());

        let writer = JsonlEventWriter::new(run_id, &base_dir).unwrap();
        assert_eq!(writer.run_id(), run_id);
        assert!(writer.file_path().to_string_lossy().contains(&run_id.to_string()));

        // Emit some events
        let pipeline_id = Uuid::new_v4();
        writer.emit(PipelineEvent::Started {
            run_id,
            pipeline_id,
            pipeline_name: Some("test".to_string()),
            started_at: Utc::now(),
        });

        writer.emit(PipelineEvent::Completed {
            run_id,
            pipeline_id,
            completed_at: Utc::now(),
            success: true,
            total_duration_secs: 1.0,
        });

        // Flush and drop to close the file
        writer.flush().unwrap();
        drop(writer);

        // Read the file and verify contents
        let file_path = base_dir.join("runs").join(run_id.to_string()).join("events.jsonl");
        let mut file = File::open(&file_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("started"));
        assert!(lines[1].contains("completed"));

        // Cleanup
        let _ = fs::remove_dir_all(base_dir.parent().unwrap());
    }
}
