//! Job scheduling logic.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{JobQueue, WorkerPool};

/// Scheduler for job execution
#[derive(Debug)]
pub struct Scheduler {
    queue: JobQueue,
    pool: Arc<RwLock<WorkerPool>>,
    strategy: SchedulingStrategy,
    interval: Duration,
}

impl Scheduler {
    #[must_use]
    pub fn new(queue: JobQueue, pool: Arc<RwLock<WorkerPool>>) -> Self {
        Self {
            queue,
            pool,
            strategy: SchedulingStrategy::default(),
            interval: Duration::from_millis(100),
        }
    }

    #[must_use]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub async fn run(&self) {
        info!("Scheduler starting with {:?}", self.strategy);

        loop {
            tokio::time::sleep(self.interval).await;

            let pool = self.pool.read().await;
            if pool.queue_len() > 0 {
                self.dispatch_jobs(&pool).await;
            }
        }
    }

    async fn dispatch_jobs(&self, pool: &WorkerPool) {
        let capacity = pool.worker_count() * 4;
        let available = capacity.saturating_sub(pool.queue_len());

        for _ in 0..available {
            if let Some(job) = self.queue.dequeue() {
                debug!("Dispatching job {}", job.id);
                pool.submit(job);
            } else {
                break;
            }
        }
    }
}

/// Scheduling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingStrategy {
    Fifo,
    Priority,
    Fair,
    Deadline,
}

impl Default for SchedulingStrategy {
    fn default() -> Self {
        Self::Fifo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduling_strategy_variants() {
        assert_ne!(SchedulingStrategy::Fifo, SchedulingStrategy::Priority);
    }

    #[test]
    fn test_scheduling_strategy_default() {
        assert_eq!(SchedulingStrategy::default(), SchedulingStrategy::Fifo);
    }
}
