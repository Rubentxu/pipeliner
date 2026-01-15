//! Job queue implementation.
//!
//! This module provides a thread-safe job queue for pipeline executions.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::WorkerResult;
use pipeliner_core::pipeline;

/// Thread-safe job queue
#[derive(Debug, Clone)]
pub struct JobQueue {
    inner: Arc<JobQueueInner>,
}

#[derive(Debug)]
struct JobQueueInner {
    pending: Arc<Mutex<BinaryHeap<JobEntry>>>,
    processing: Arc<DashMap<Uuid, Job>>,
    completed: Arc<DashMap<Uuid, Job>>,
    cancelled: Arc<DashMap<Uuid, Job>>,
}

impl JobQueue {
    /// Creates a new job queue
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(JobQueueInner {
                pending: Arc::new(Mutex::new(BinaryHeap::new())),
                processing: Arc::new(DashMap::new()),
                completed: Arc::new(DashMap::new()),
                cancelled: Arc::new(DashMap::new()),
            }),
        }
    }

    /// Enqueues a job
    pub fn enqueue(&self, job: Job) {
        let entry = JobEntry {
            priority: job.priority,
            id: job.id,
            created_at: job.created_at,
        };
        let mut pending = self.inner.pending.lock().unwrap();
        pending.push(entry);
    }

    /// Dequeues the next job
    pub fn dequeue(&self) -> Option<Job> {
        let mut pending = self.inner.pending.lock().unwrap();
        pending.pop().map(|entry| {
            self.inner.processing.insert(entry.id, Job::default());
            self.inner
                .processing
                .get(&entry.id)
                .unwrap()
                .value()
                .clone()
        })
    }

    /// Gets a job by ID
    pub fn get(&self, id: &Uuid) -> Option<Job> {
        if let Some(job) = self.inner.processing.get(id) {
            return Some(job.value().clone());
        }
        if let Some(job) = self.inner.completed.get(id) {
            return Some(job.value().clone());
        }
        if let Some(job) = self.inner.cancelled.get(id) {
            return Some(job.value().clone());
        }
        None
    }

    /// Marks a job as completed
    pub fn complete(&self, id: &Uuid) {
        if let Some((_, job)) = self.inner.processing.remove(id) {
            self.inner.completed.insert(job.id, job);
        }
    }

    /// Marks a job as cancelled
    pub fn cancel(&self, id: &Uuid) {
        if let Some((_, job)) = self.inner.processing.remove(id) {
            self.inner.cancelled.insert(job.id, job);
        }
    }

    /// Returns the number of pending jobs
    #[must_use]
    pub fn len(&self) -> usize {
        let pending = self.inner.pending.lock().unwrap();
        pending.len()
    }

    /// Returns true if the queue is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of processing jobs
    #[must_use]
    pub fn processing_count(&self) -> usize {
        self.inner.processing.len()
    }

    /// Returns the number of completed jobs
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.inner.completed.len()
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Job entry for the priority queue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JobEntry {
    priority: JobPriority,
    id: Uuid,
    created_at: DateTime<Utc>,
}

impl Ord for JobEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse order for min-heap behavior (lower priority number = higher priority)
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| other.created_at.cmp(&self.created_at))
    }
}

impl PartialOrd for JobEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A job in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job ID
    pub id: Uuid,
    /// Pipeline to execute
    pub pipeline: Option<pipeline::Pipeline>,
    /// Job priority
    pub priority: JobPriority,
    /// Current status
    pub status: JobStatus,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Started at
    pub started_at: Option<DateTime<Utc>>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of retries
    pub retries: u32,
    /// Maximum retries
    pub max_retries: u32,
    /// Job metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            pipeline: None,
            priority: JobPriority::Normal,
            status: JobStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
            retries: 0,
            max_retries: 3,
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl Job {
    /// Creates a new job
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a job from a pipeline
    #[must_use]
    pub fn from_pipeline(pipeline: pipeliner_core::Pipeline) -> Self {
        Self {
            id: Uuid::new_v4(),
            pipeline: Some(pipeline),
            created_at: Utc::now(),
            ..Self::default()
        }
    }

    /// Sets the priority
    #[must_use]
    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Sets the maximum retries
    #[must_use]
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Marks the job as running
    pub fn start(&mut self) {
        self.status = JobStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Marks the job as completed
    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Marks the job as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = JobStatus::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(Utc::now());
    }

    /// Cancels the job
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Increments retry count
    #[must_use]
    pub fn retry(&mut self) -> bool {
        if self.retries >= self.max_retries {
            return false;
        }
        self.retries += 1;
        true
    }
}

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JobPriority {
    /// Highest priority
    Critical = 0,
    /// High priority
    High = 1,
    /// Normal priority
    Normal = 2,
    /// Low priority
    Low = 3,
    /// Lowest priority
    Background = 4,
}

impl Default for JobPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is pending
    Pending,
    /// Job is running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was cancelled
    Cancelled,
}

impl Default for JobStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_queue_new() {
        let queue = JobQueue::new();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_job_queue_enqueue_dequeue() {
        let queue = JobQueue::new();
        let job = Job::from_pipeline(pipeliner_core::Pipeline::new());
        queue.enqueue(job);

        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
    }

    #[test]
    fn test_job_from_pipeline() {
        let pipeline = pipeliner_core::Pipeline::new().with_name("Test");
        let job = Job::from_pipeline(pipeline);

        assert!(job.pipeline.is_some());
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[test]
    fn test_job_start() {
        let mut job = Job::new();
        assert_eq!(job.status, JobStatus::Pending);

        job.start();
        assert_eq!(job.status, JobStatus::Running);
        assert!(job.started_at.is_some());
    }

    #[test]
    fn test_job_complete() {
        let mut job = Job::new();
        job.start();
        job.complete();

        assert_eq!(job.status, JobStatus::Completed);
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_fail() {
        let mut job = Job::new();
        job.start();
        job.fail("test error");

        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.error, Some("test error".to_string()));
    }

    #[test]
    fn test_job_retry() {
        let mut job = Job::new().with_max_retries(3);
        assert_eq!(job.retries, 0);

        assert!(job.retry());
        assert_eq!(job.retries, 1);
        assert!(job.retry());
        assert_eq!(job.retries, 2);
        assert!(job.retry());
        assert_eq!(job.retries, 3);
        assert!(!job.retry()); // Should return false now
    }

    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::Critical < JobPriority::High);
        assert!(JobPriority::High < JobPriority::Normal);
        assert!(JobPriority::Normal < JobPriority::Low);
        assert!(JobPriority::Low < JobPriority::Background);
    }
}
