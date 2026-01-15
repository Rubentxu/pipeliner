//! Metrics collection
//!
//! Provides metrics for pipeline execution.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Metrics for a pipeline execution
#[derive(Debug, Clone)]
pub struct PipelineMetrics {
    /// Pipeline name
    pub pipeline_name: String,

    /// Execution duration
    pub duration: Duration,

    /// Number of stages
    pub stage_count: usize,

    /// Number of successful stages
    pub successful_stages: usize,

    /// Number of failed stages
    pub failed_stages: usize,
}

/// Metrics collector for pipeline executions
pub struct MetricsCollector {
    /// Collected metrics
    metrics: Arc<RwLock<HashMap<String, PipelineMetrics>>>,
}

impl MetricsCollector {
    /// Creates a new metrics collector
    #[must_use]
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Records metrics for a pipeline execution
    #[allow(clippy::missing_panics_doc)]
    pub fn record(&self, metrics: PipelineMetrics) {
        let mut metrics_map = self.metrics.write().unwrap();
        metrics_map.insert(metrics.pipeline_name.clone(), metrics);
    }

    /// Gets metrics for a specific pipeline
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn get(&self, pipeline_name: &str) -> Option<PipelineMetrics> {
        let metrics_map = self.metrics.read().unwrap();
        metrics_map.get(pipeline_name).cloned()
    }

    /// Gets all recorded metrics
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn get_all(&self) -> Vec<PipelineMetrics> {
        let metrics_map = self.metrics.read().unwrap();
        metrics_map.values().cloned().collect()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();

        assert!(collector.get("test").is_none());
        assert!(collector.get_all().is_empty());
    }

    #[test]
    fn test_metrics_collector_record() {
        let collector = MetricsCollector::new();

        let metrics = PipelineMetrics {
            pipeline_name: "test".to_string(),
            duration: Duration::from_secs(10),
            stage_count: 2,
            successful_stages: 2,
            failed_stages: 0,
        };

        collector.record(metrics);

        let retrieved = collector.get("test");
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.pipeline_name, "test");
        assert_eq!(retrieved.stage_count, 2);
    }
}
