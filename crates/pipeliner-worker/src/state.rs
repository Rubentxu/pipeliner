//! Execution state tracking.
//!
//! This module provides state management for pipeline execution.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use crate::Job;

/// Current execution state
#[derive(Debug, Clone, Default)]
pub struct ExecutionState {
    /// All jobs by ID
    jobs: Arc<DashMap<Uuid, Job>>,
    /// Active job IDs
    active: Arc<DashMap<Uuid, Instant>>,
    /// Completed job IDs
    completed: Arc<DashMap<Uuid, Instant>>,
    /// Failed job IDs
    failed: Arc<DashMap<Uuid, Instant>>,
    /// Total jobs processed
    total_processed: Arc<std::sync::atomic::AtomicUsize>,
    /// Total jobs failed
    total_failed: Arc<std::sync::atomic::AtomicUsize>,
    /// Last job ID
    last_job_id: Arc<std::sync::RwLock<Option<Uuid>>>,
}

impl ExecutionState {
    /// Creates a new execution state
    #[must_use]
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(DashMap::new()),
            active: Arc::new(DashMap::new()),
            completed: Arc::new(DashMap::new()),
            failed: Arc::new(DashMap::new()),
            total_processed: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            total_failed: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            last_job_id: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    /// Adds a job
    pub fn add_job(&self, job: Job) {
        self.jobs.insert(job.id, job.clone());
        *self.last_job_id.write().unwrap() = Some(job.id);
    }

    /// Gets a job by ID
    #[must_use]
    pub fn get_job(&self, id: &Uuid) -> Option<Job> {
        self.jobs.get(id).map(|j| j.value().clone())
    }

    /// Marks a job as active
    pub fn mark_active(&self, id: &Uuid) {
        self.active.insert(*id, Instant::now());
    }

    /// Marks a job as completed
    pub fn mark_completed(&self, id: &Uuid) {
        self.active.remove(id);
        self.completed.insert(*id, Instant::now());
        self.total_processed
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Marks a job as failed
    pub fn mark_failed(&self, id: &Uuid) {
        self.active.remove(id);
        self.failed.insert(*id, Instant::now());
        self.total_failed
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Returns the number of active jobs
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Returns the number of completed jobs
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Returns the number of failed jobs
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.failed.len()
    }

    /// Returns total processed count
    #[must_use]
    pub fn total_processed(&self) -> usize {
        self.total_processed
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns total failed count
    #[must_use]
    pub fn total_failed(&self) -> usize {
        self.total_failed.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns the last job ID
    #[must_use]
    pub fn last_job_id(&self) -> Option<Uuid> {
        *self.last_job_id.read().unwrap()
    }

    /// Returns all active job IDs
    #[must_use]
    pub fn active_jobs(&self) -> Vec<Uuid> {
        self.active.iter().map(|e| *e.key()).collect()
    }

    /// Returns statistics
    #[must_use]
    pub fn stats(&self) -> StateStats {
        StateStats {
            active: self.active_count(),
            completed: self.completed_count(),
            failed: self.failed_count(),
            total_processed: self.total_processed(),
            total_failed: self.total_failed(),
        }
    }
}

/// State statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStats {
    /// Number of active jobs
    pub active: usize,
    /// Number of completed jobs
    pub completed: usize,
    /// Number of failed jobs
    pub failed: usize,
    /// Total processed
    pub total_processed: usize,
    /// Total failed
    pub total_failed: usize,
}

impl Default for StateStats {
    fn default() -> Self {
        Self {
            active: 0,
            completed: 0,
            failed: 0,
            total_processed: 0,
            total_failed: 0,
        }
    }
}

/// Worker state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerSnapshot {
    /// Worker ID
    pub worker_id: String,
    /// Is running
    pub is_running: bool,
    /// Current job
    pub current_job: Option<JobSummary>,
    /// Jobs processed
    pub jobs_processed: usize,
    /// Jobs failed
    pub jobs_failed: usize,
    /// Last heartbeat
    pub last_heartbeat: Option<u64>,
}

/// Job summary for snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: String,
    pub status: String,
    pub started_at: Option<u64>,
}

impl From<Job> for JobSummary {
    fn from(job: Job) -> Self {
        Self {
            id: job.id.to_string(),
            status: format!("{:?}", job.status),
            started_at: job.started_at.map(|d| d.timestamp() as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_state_new() {
        let state = ExecutionState::new();
        assert_eq!(state.active_count(), 0);
        assert_eq!(state.completed_count(), 0);
    }

    #[test]
    fn test_execution_state_add_job() {
        let state = ExecutionState::new();
        let job = Job::new();
        state.add_job(job.clone());

        assert_eq!(state.jobs.len(), 1);
        assert!(state.get_job(&job.id).is_some());
    }

    #[test]
    fn test_execution_state_mark_active_completed() {
        let state = ExecutionState::new();
        let job = Job::new();

        state.add_job(job.clone());
        state.mark_active(&job.id);

        assert_eq!(state.active_count(), 1);

        state.mark_completed(&job.id);
        assert_eq!(state.active_count(), 0);
        assert_eq!(state.completed_count(), 1);
        assert_eq!(state.total_processed(), 1);
    }

    #[test]
    fn test_execution_state_mark_failed() {
        let state = ExecutionState::new();
        let job = Job::new();

        state.add_job(job.clone());
        state.mark_active(&job.id);
        state.mark_failed(&job.id);

        assert_eq!(state.active_count(), 0);
        assert_eq!(state.failed_count(), 1);
        assert_eq!(state.total_failed(), 1);
    }

    #[test]
    fn test_state_stats_default() {
        let stats = StateStats::default();
        assert_eq!(stats.active, 0);
        assert_eq!(stats.completed, 0);
    }
}
