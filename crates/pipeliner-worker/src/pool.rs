//! Worker pool management.
//!
//! This module provides the worker pool for parallel job execution.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task;
use tracing::{debug, error, info, warn};

use crate::{Job, JobQueue, WorkerResult};

/// Worker identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkerId(pub usize);

impl std::fmt::Display for WorkerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "worker-{}", self.0)
    }
}

/// Worker configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub max_concurrent: usize,
    pub job_timeout: Option<Duration>,
    pub heartbeat_interval: Duration,
    pub shutdown_timeout: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            job_timeout: Some(Duration::from_secs(3600)),
            heartbeat_interval: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(60),
        }
    }
}

/// A single worker
#[derive(Debug)]
pub struct Worker {
    id: WorkerId,
    config: WorkerConfig,
    queue: JobQueue,
    rx: mpsc::Receiver<WorkerMessage>,
    active_jobs: Arc<AtomicUsize>,
}

enum WorkerMessage {
    Job(Job),
    Stop,
}

impl Worker {
    #[must_use]
    pub fn new(
        id: WorkerId,
        config: WorkerConfig,
        queue: JobQueue,
        rx: mpsc::Receiver<WorkerMessage>,
    ) -> Self {
        Self {
            id,
            config,
            queue,
            rx,
            active_jobs: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn run(&mut self) {
        info!("Worker {} starting", self.id);

        loop {
            tokio::select! {
                Some(msg) = self.rx.recv() => {
                    match msg {
                        WorkerMessage::Job(job) => {
                            self.execute_job(job).await;
                        }
                        WorkerMessage::Stop => {
                            info!("Worker {} stopping", self.id);
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(self.config.heartbeat_interval) => {
                    // Heartbeat
                }
            }
        }
    }

    async fn execute_job(&mut self, mut job: Job) {
        info!("Worker {} executing job {}", self.id, job.id);

        let active = self.active_jobs.fetch_add(1, Ordering::SeqCst);
        if active >= self.config.max_concurrent {
            self.active_jobs.fetch_sub(1, Ordering::SeqCst);
            warn!("Worker {} at capacity, re-queueing job", self.id);
            self.queue.enqueue(job);
            return;
        }

        job.start();

        let result = tokio::time::timeout(
            self.config.job_timeout.unwrap_or(Duration::MAX),
            self.run_pipeline(&job),
        )
        .await;

        match result {
            Ok(Ok(())) => {
                job.complete();
                self.queue.complete(&job.id);
                info!("Job {} completed successfully", job.id);
            }
            Ok(Err(e)) => {
                job.fail(e.to_string());

                if job.retry() {
                    warn!(
                        "Job {} failed, retrying ({}/{})",
                        job.id, job.retries, job.max_retries
                    );
                    self.queue.enqueue(job);
                } else {
                    self.queue.complete(&job.id);
                    error!("Job {} failed after retries", job.id);
                }
            }
            Err(_) => {
                job.fail("timeout");
                self.queue.complete(&job.id);
                error!("Job {} timed out", job.id);
            }
        }

        self.active_jobs.fetch_sub(1, Ordering::SeqCst);
    }

    async fn run_pipeline(&self, _job: &Job) -> WorkerResult<()> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }
}

/// Worker pool
#[derive(Debug)]
pub struct WorkerPool {
    config: WorkerConfig,
    queue: JobQueue,
    workers: Vec<task::JoinHandle<()>>,
}

impl WorkerPool {
    #[must_use]
    pub fn new(config: WorkerConfig, queue: JobQueue) -> Self {
        Self {
            config,
            queue,
            workers: Vec::new(),
        }
    }

    pub async fn start(&mut self) {
        for i in 0..self.config.max_concurrent {
            let (tx, rx) = mpsc::channel(100);
            let worker_id = WorkerId(i);

            let mut worker = Worker::new(worker_id, self.config.clone(), self.queue.clone(), rx);
            let handle = task::spawn(async move {
                worker.run().await;
            });

            self.workers.push(handle);
        }

        info!(
            "Worker pool started with {} workers",
            self.config.max_concurrent
        );
    }

    pub fn submit(&self, job: Job) {
        self.queue.enqueue(job);
    }

    pub async fn stop(&mut self) {
        info!("Stopping worker pool...");

        for worker in self.workers.drain(..) {
            worker.abort();
        }

        info!("Worker pool stopped");
    }

    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_config_default() {
        let config = WorkerConfig::default();
        assert_eq!(config.max_concurrent, 4);
    }

    #[test]
    fn test_worker_id_display() {
        let id = WorkerId(1);
        assert_eq!(id.to_string(), "worker-1");
    }
}
