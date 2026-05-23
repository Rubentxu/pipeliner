//! # Report Module
//!
//! Execution reporting for pipeline runs.
//!
//! This module provides:
//! - [`ExecutionReport`] - Complete execution report with timing information
//! - [`ReportGenerator`] - Generates reports from execution results
//! - [`ReportFormat`] - Output format selection (JSON, human-readable)
//! - [`StepTiming`] - Timing information for individual steps
//! - [`StageTiming`] - Timing information for stages
//!
//! ## Report Structure
//!
//! ```ignore
//! ExecutionReport {
//!     pipeline_id: UUID,
//!     pipeline_name: Option<String>,
//!     started_at: DateTime,
//!     completed_at: DateTime,
//!     success: bool,
//!     stage_timings: [
//!         StageTiming {
//!             stage_id: "build",
//!             stage_name: "Build",
//!             started_at: DateTime,
//!             completed_at: DateTime,
//!             success: true,
//!             step_timings: [
//!                 StepTiming {
//!                     step_index: 0,
//!                     step_type: "shell",
//!                     started_at: DateTime,
//!                     completed_at: DateTime,
//!                     success: true,
//!                     exit_code: Some(0),
//!                     duration_secs: 1.5,
//!                 }
//!             ],
//!         }
//!     ],
//!     total_duration_secs: 45.0,
//!     error_summary: None,
//! }
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pipeliner_runtime::{LocalExecutor, ReportGenerator, ReportFormat};
//!
//! let executor = LocalExecutor::new();
//! let result = executor.execute(&spec).await;
//!
//! let report = ReportGenerator::generate(&result);
//! println!("{}", report.to_json().unwrap());
//! println!("{}", report.to_human_readable().unwrap());
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pipeliner_core::spec::EnvSpec;

use crate::events::PipelineEvent;
use crate::local_executor::{ExecutionResult, StageResult, StepResult};

/// Timing information for a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepTiming {
    /// Step index within the stage
    pub step_index: usize,
    /// Step type (e.g., "shell", "echo")
    pub step_type: String,
    /// Step label if provided
    pub step_label: Option<String>,
    /// When the step started
    pub started_at: DateTime<Utc>,
    /// When the step completed
    pub completed_at: DateTime<Utc>,
    /// Whether the step succeeded
    pub success: bool,
    /// Exit code if applicable
    pub exit_code: Option<i32>,
    /// Step duration in seconds
    pub duration_secs: f64,
    /// Step duration in milliseconds
    pub duration_ms: i64,
    /// Standard output (if captured)
    pub stdout: Option<String>,
    /// Standard error (if captured)
    pub stderr: Option<String>,
    /// Length of captured output (for let_output steps)
    pub output_length: Option<usize>,
}

impl StepTiming {
    /// Creates a new step timing from a step result.
    pub fn from_step_result(
        step_index: usize,
        step_type: &str,
        step_label: Option<String>,
        result: &StepResult,
        started_at: DateTime<Utc>,
    ) -> Self {
        let duration_ms = (result.duration_secs * 1000.0) as i64;
        let output_length = result.stdout.as_ref().map(|s| s.len());
        Self {
            step_index,
            step_type: step_type.to_string(),
            step_label,
            started_at,
            completed_at: Utc::now(),
            success: result.success,
            exit_code: result.exit_code,
            duration_secs: result.duration_secs,
            duration_ms,
            stdout: result.stdout.clone(),
            stderr: result.stderr.clone(),
            output_length,
        }
    }

    /// Returns true if this step timed out.
    pub fn is_timeout(&self) -> bool {
        self.exit_code.is_none() && !self.success
    }
}

/// Timing information for a stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageTiming {
    /// Stage ID
    pub stage_id: String,
    /// Stage display name
    pub stage_name: String,
    /// When the stage started
    pub started_at: DateTime<Utc>,
    /// When the stage completed
    pub completed_at: DateTime<Utc>,
    /// Whether the stage succeeded
    pub success: bool,
    /// Exit code if applicable
    pub exit_code: Option<i32>,
    /// Stage duration in seconds
    pub duration_secs: f64,
    /// Stage duration in milliseconds
    pub duration_ms: i64,
    /// Timing information for each step
    pub step_timings: Vec<StepTiming>,
    /// Number of retry attempts
    pub retry_count: u32,
}

impl StageTiming {
    /// Creates a new stage timing from a stage result.
    pub fn from_stage_result(
        stage_id: &str,
        stage_name: &str,
        result: &StageResult,
        started_at: DateTime<Utc>,
    ) -> Self {
        let duration_ms = (result.duration_secs * 1000.0) as i64;
        Self {
            stage_id: stage_id.to_string(),
            stage_name: stage_name.to_string(),
            started_at,
            completed_at: Utc::now(),
            success: result.success,
            exit_code: result.exit_code,
            duration_secs: result.duration_secs,
            duration_ms,
            step_timings: result
                .step_results
                .iter()
                .enumerate()
                .map(|(i, sr)| {
                    StepTiming::from_step_result(
                        i,
                        &sr.step_type,
                        sr.step_label.clone(),
                        &sr.result,
                        sr.started_at,
                    )
                })
                .collect(),
            retry_count: result.retry_count,
        }
    }

    /// Returns the total number of steps in this stage.
    pub fn total_steps(&self) -> usize {
        self.step_timings.len()
    }

    /// Returns the number of successful steps.
    pub fn successful_steps(&self) -> usize {
        self.step_timings.iter().filter(|s| s.success).count()
    }

    /// Returns the number of failed steps.
    pub fn failed_steps(&self) -> usize {
        self.step_timings.iter().filter(|s| !s.success).count()
    }
}

/// Complete execution report for a pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    /// Pipeline ID
    pub pipeline_id: Uuid,
    /// Pipeline name if available
    pub pipeline_name: Option<String>,
    /// When the pipeline started
    pub started_at: DateTime<Utc>,
    /// When the pipeline completed
    pub completed_at: DateTime<Utc>,
    /// Whether the pipeline succeeded
    pub success: bool,
    /// Exit code if applicable
    pub exit_code: Option<i32>,
    /// Total duration in seconds
    pub total_duration_secs: f64,
    /// Total duration in milliseconds
    pub total_duration_ms: i64,
    /// Timing information for each stage
    pub stage_timings: Vec<StageTiming>,
    /// Summary of errors (if any)
    pub error_summary: Option<String>,
    /// Number of stages that were retried
    pub total_retries: u32,
    /// Environment snapshot at pipeline start
    pub env_snapshot: Option<EnvSpec>,
}

impl ExecutionReport {
    /// Creates a new execution report from an execution result.
    ///
    /// # Arguments
    ///
    /// * `result` - The execution result to generate the report from
    /// * `env_snapshot` - Optional environment snapshot to include in the report
    pub fn from_execution_result(result: &ExecutionResult, env_snapshot: Option<EnvSpec>) -> Self {
        let total_retries: u32 = result
            .stage_results
            .iter()
            .map(|s| s.retry_count)
            .sum();

        let error_summary = if !result.success {
            let failed_stages: Vec<_> = result
                .stage_results
                .iter()
                .filter(|s| !s.success)
                .map(|s| format!("{} (exit code: {:?})", s.stage_name, s.exit_code))
                .collect();
            Some(format!(
                "Failed stages: {}",
                failed_stages.join(", ")
            ))
        } else {
            None
        };

        let total_duration_ms = (result.total_duration_secs * 1000.0) as i64;

        Self {
            pipeline_id: result.pipeline_id,
            pipeline_name: result.pipeline_name.clone(),
            started_at: result.started_at,
            completed_at: result.completed_at,
            success: result.success,
            exit_code: result.exit_code,
            total_duration_secs: result.total_duration_secs,
            total_duration_ms,
            stage_timings: result
                .stage_results
                .iter()
                .enumerate()
                .map(|(i, sr)| {
                    StageTiming::from_stage_result(
                        &sr.stage_id,
                        &sr.stage_name,
                        sr,
                        result.stage_started_at(i),
                    )
                })
                .collect(),
            error_summary,
            total_retries,
            env_snapshot,
        }
    }

    /// Returns the total number of stages.
    pub fn total_stages(&self) -> usize {
        self.stage_timings.len()
    }

    /// Returns the number of successful stages.
    pub fn successful_stages(&self) -> usize {
        self.stage_timings.iter().filter(|s| s.success).count()
    }

    /// Returns the number of failed stages.
    pub fn failed_stages(&self) -> usize {
        self.stage_timings.iter().filter(|s| !s.success).count()
    }

    /// Returns the total number of steps across all stages.
    pub fn total_steps(&self) -> usize {
        self.stage_timings.iter().map(|s| s.total_steps()).sum()
    }

    /// Returns a summary string for the report.
    pub fn summary(&self) -> String {
        format!(
            "Pipeline '{}' {} in {:.2}s ({} stages, {} steps)",
            self.pipeline_name.as_deref().unwrap_or("unnamed"),
            if self.success { "succeeded" } else { "failed" },
            self.total_duration_secs,
            self.total_stages(),
            self.total_steps(),
        )
    }
}

/// Report output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// JSON format
    Json,
    /// Human-readable format
    HumanReadable,
    /// Compact summary format
    Summary,
}

impl Default for ReportFormat {
    fn default() -> Self {
        Self::Json
    }
}

/// Generates execution reports from execution results.
pub struct ReportGenerator;

impl ReportGenerator {
    /// Generates a report from an execution result.
    ///
    /// # Arguments
    ///
    /// * `result` - The execution result to generate the report from
    /// * `env_snapshot` - Optional environment snapshot to include in the report
    pub fn generate(result: &ExecutionResult, env_snapshot: Option<EnvSpec>) -> ExecutionReport {
        ExecutionReport::from_execution_result(result, env_snapshot)
    }

    /// Formats a report as JSON.
    pub fn to_json(report: &ExecutionReport) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(report)
    }

    /// Formats a report in a compact JSON format.
    pub fn to_compact_json(report: &ExecutionReport) -> Result<String, serde_json::Error> {
        serde_json::to_string(report)
    }

    /// Formats a report in human-readable format.
    pub fn to_human_readable(report: &ExecutionReport) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "═══════════════════════════════════════════════════════════════\n"
        ));
        output.push_str(&format!(
            "  Pipeline: {}\n",
            report.pipeline_name.as_deref().unwrap_or("unnamed")
        ));
        output.push_str(&format!(
            "  Pipeline ID: {}\n",
            report.pipeline_id
        ));
        output.push_str(&format!(
            "  Status: {}\n",
            if report.success { "✅ SUCCESS" } else { "❌ FAILED" }
        ));
        output.push_str(&format!(
            "  Duration: {:.2}s\n",
            report.total_duration_secs
        ));
        if report.total_retries > 0 {
            output.push_str(&format!(
                "  Retries: {}\n",
                report.total_retries
            ));
        }
        output.push_str(&format!(
            "═══════════════════════════════════════════════════════════════\n"
        ));

        for stage in &report.stage_timings {
            output.push_str(&format!(
                "\n  Stage: {} [{}]\n",
                stage.stage_name,
                if stage.success { "✅" } else { "❌" }
            ));
            output.push_str(&format!(
                "    Duration: {:.2}s\n",
                stage.duration_secs
            ));
            if stage.retry_count > 0 {
                output.push_str(&format!(
                    "    Retries: {}\n",
                    stage.retry_count
                ));
            }

            for step in &stage.step_timings {
                let status = if step.success { "✅" } else { "❌" };
                let exit_str = step
                    .exit_code
                    .map(|c| format!(" (exit code: {})", c))
                    .unwrap_or_default();
                output.push_str(&format!(
                    "    {} Step {}: {} [{:.2}s{}{}]\n",
                    status,
                    step.step_index,
                    step.step_type,
                    step.duration_secs,
                    exit_str,
                    if step.step_label.is_some() {
                        format!(" - {}", step.step_label.as_ref().unwrap())
                    } else {
                        String::new()
                    }
                ));
            }
        }

        if let Some(ref error) = report.error_summary {
            output.push_str(&format!(
                "\n═══════════════════════════════════════════════════════════════\n"
            ));
            output.push_str(&format!("  Errors: {}\n", error));
        }

        output.push_str(&format!(
            "\n═══════════════════════════════════════════════════════════════\n"
        ));

        output
    }

    /// Formats a report as a compact summary.
    pub fn to_summary(report: &ExecutionReport) -> String {
        let stage_summary: String = report
            .stage_timings
            .iter()
            .map(|s| {
                format!(
                    "{}[{:.1}s]",
                    if s.success { "✓" } else { "✗" },
                    s.duration_secs
                )
            })
            .collect::<Vec<_>>()
            .join(" → ");

        format!(
            "{} {} ({:.2}s) | {} | {}",
            if report.success { "✅" } else { "❌" },
            report.pipeline_name.as_deref().unwrap_or("unnamed"),
            report.total_duration_secs,
            stage_summary,
            report.error_summary.as_deref().unwrap_or("no errors"),
        )
    }

    /// Formats a report in the specified format.
    pub fn format(report: &ExecutionReport, format: ReportFormat) -> Result<String, String> {
        match format {
            ReportFormat::Json => Self::to_json(report).map_err(|e| e.to_string()),
            ReportFormat::HumanReadable => Ok(Self::to_human_readable(report)),
            ReportFormat::Summary => Ok(Self::to_summary(report)),
        }
    }

    /// Generates a report from a list of pipeline events.
    ///
    /// This is useful when you only have access to the event stream.
    pub fn from_events(events: &[PipelineEvent]) -> Option<ExecutionReport> {
        let run_id = events.first()?.run_id();
        let pipeline_id = events.first()?.pipeline_id();

        let started_at = events.iter().find_map(|e| {
            if let PipelineEvent::Started { started_at, .. } = e {
                Some(*started_at)
            } else {
                None
            }
        })?;

        let completed_event = events.iter().find_map(|e| {
            match e {
                PipelineEvent::Completed {
                    run_id: _,
                    pipeline_id: _,
                    completed_at,
                    success,
                    total_duration_secs,
                } => Some((*completed_at, *success, *total_duration_secs)),
                PipelineEvent::Failed {
                    run_id: _,
                    pipeline_id: _,
                    failed_at,
                    reason: _,
                    total_duration_secs,
                } => Some((*failed_at, false, *total_duration_secs)),
                _ => None,
            }
        })?;

        // Group events by stage
        let mut stage_timings: Vec<StageTiming> = Vec::new();
        let mut current_stage: Option<(String, String, DateTime<Utc>, Vec<PipelineEvent>)> = None;
        let mut current_step_events: Vec<PipelineEvent> = Vec::new();

        for event in events {
            match event {
                PipelineEvent::StageStarted { stage_id, stage_name, started_at, .. } => {
                    // Save previous stage
                    if let Some((stage_id, stage_name, started_at, step_events)) =
                        current_stage.take()
                    {
                        let stage_result = Self::events_to_stage_result(
                            &stage_id,
                            &stage_name,
                            started_at,
                            &step_events,
                        );
                        stage_timings.push(StageTiming::from_stage_result(
                            &stage_id,
                            &stage_name,
                            &stage_result,
                            started_at,
                        ));
                    }
                    current_stage = Some((stage_id.clone(), stage_name.clone(), *started_at, Vec::new()));
                }
                PipelineEvent::StepStarted { .. } | PipelineEvent::StepCompleted { .. } => {
                    if let Some((_, _, _, ref mut step_events)) = current_stage {
                        step_events.push(event.clone());
                    }
                }
                _ => {}
            }
        }

        // Save last stage
        if let Some((stage_id, stage_name, started_at, step_events)) = current_stage.take() {
            let stage_result =
                Self::events_to_stage_result(&stage_id, &stage_name, started_at, &step_events);
            stage_timings.push(StageTiming::from_stage_result(
                &stage_id,
                &stage_name,
                &stage_result,
                started_at,
            ));
        }

        let total_duration_ms = (completed_event.2 * 1000.0) as i64;

        Some(ExecutionReport {
            pipeline_id,
            pipeline_name: None,
            started_at,
            completed_at: completed_event.0,
            success: completed_event.1,
            exit_code: None,
            total_duration_secs: completed_event.2,
            total_duration_ms,
            stage_timings,
            error_summary: None,
            total_retries: 0,
            env_snapshot: None,
        })
    }

    fn events_to_stage_result(
        stage_id: &str,
        stage_name: &str,
        started_at: DateTime<Utc>,
        events: &[PipelineEvent],
    ) -> StageResult {
        let mut step_results = Vec::new();
        let mut current_step: Option<(usize, String, DateTime<Utc>)> = None;

        for event in events {
            match event {
                PipelineEvent::StepStarted {
                    step_index,
                    step_type,
                    started_at: step_started,
                    ..
                } => {
                    current_step = Some((*step_index, step_type.clone(), *step_started));
                }
                PipelineEvent::StepCompleted {
                    step_index,
                    step_type: _,
                    completed_at: _,
                    success,
                    exit_code,
                    duration_secs,
                    ..
                } => {
                    if let Some((idx, step_type, step_started)) = current_step.take() {
                        step_results.push(crate::local_executor::StepResultEntry {
                            step_index: idx,
                            step_type,
                            step_label: None,
                            started_at: step_started,
                            result: StepResult {
                                success: *success,
                                exit_code: *exit_code,
                                duration_secs: *duration_secs,
                                stdout: None,
                                stderr: None,
                            },
                        });
                    }
                }
                _ => {}
            }
        }

        let success = step_results.iter().all(|s| s.result.success);
        let duration_secs = if let Some(last) = step_results.last() {
            last.result.duration_secs
        } else {
            0.0
        };

        StageResult {
            stage_id: stage_id.to_string(),
            stage_name: stage_name.to_string(),
            success,
            exit_code: None,
            duration_secs,
            step_results,
            retry_count: 0,
            started_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_executor::{ExecutionResult, StepResult, StepResultEntry};

    fn create_test_execution_result(success: bool) -> ExecutionResult {
        let pipeline_id = Uuid::new_v4();
        let started_at = Utc::now();

        ExecutionResult {
            pipeline_id,
            pipeline_name: Some("test-pipeline".to_string()),
            success,
            exit_code: if success { Some(0) } else { Some(1) },
            started_at,
            completed_at: Utc::now(),
            total_duration_secs: 10.0,
            stage_results: vec![StageResult {
                stage_id: "build".to_string(),
                stage_name: "Build".to_string(),
                success,
                exit_code: if success { Some(0) } else { Some(1) },
                duration_secs: 5.0,
                step_results: vec![StepResultEntry {
                    step_index: 0,
                    step_type: "shell".to_string(),
                    step_label: Some("compile".to_string()),
                    started_at,
                    result: StepResult {
                        success,
                        exit_code: if success { Some(0) } else { Some(1) },
                        duration_secs: 2.0,
                        stdout: Some("output".to_string()),
                        stderr: None,
                    },
                }],
                retry_count: 0,
                started_at,
            }],
            total_retries: 0,
        }
    }

    #[test]
    fn test_execution_report_from_result() {
        let result = create_test_execution_result(true);
        let report = ExecutionReport::from_execution_result(&result, None);

        assert_eq!(report.pipeline_id, result.pipeline_id);
        assert_eq!(report.pipeline_name, Some("test-pipeline".to_string()));
        assert!(report.success);
        assert_eq!(report.total_stages(), 1);
        assert_eq!(report.total_steps(), 1);
        assert_eq!(report.successful_stages(), 1);
        assert_eq!(report.failed_stages(), 0);
    }

    #[test]
    fn test_execution_report_failure() {
        let result = create_test_execution_result(false);
        let report = ExecutionReport::from_execution_result(&result, None);

        assert!(!report.success);
        assert!(report.error_summary.is_some());
        // Stage name is "Build" (with capital B)
        assert!(report.error_summary.unwrap().contains("Build"));
    }

    #[test]
    fn test_step_timing_from_result() {
        let result = StepResult {
            success: true,
            exit_code: Some(0),
            duration_secs: 1.5,
            stdout: Some("hello".to_string()),
            stderr: None,
        };

        let timing = StepTiming::from_step_result(
            0,
            "shell",
            Some("test".to_string()),
            &result,
            Utc::now(),
        );

        assert_eq!(timing.step_index, 0);
        assert_eq!(timing.step_type, "shell");
        assert_eq!(timing.step_label, Some("test".to_string()));
        assert!(timing.success);
        assert_eq!(timing.duration_secs, 1.5);
        assert_eq!(timing.duration_ms, 1500);
        assert_eq!(timing.output_length, Some(5));
    }

    #[test]
    fn test_stage_timing_from_result() {
        let result = create_test_execution_result(true);
        let stage_result = &result.stage_results[0];

        let timing = StageTiming::from_stage_result(
            &stage_result.stage_id,
            &stage_result.stage_name,
            stage_result,
            stage_result.started_at,
        );

        assert_eq!(timing.stage_id, "build");
        assert_eq!(timing.stage_name, "Build");
        assert!(timing.success);
        assert_eq!(timing.total_steps(), 1);
        assert_eq!(timing.successful_steps(), 1);
        assert!(timing.duration_ms > 0);
    }

    #[test]
    fn test_report_to_json() {
        let result = create_test_execution_result(true);
        let report = ExecutionReport::from_execution_result(&result, None);

        let json = ReportGenerator::to_json(&report).unwrap();
        assert!(json.contains("test-pipeline"));
        assert!(json.contains("build"));
    }

    #[test]
    fn test_report_to_human_readable() {
        let result = create_test_execution_result(true);
        let report = ExecutionReport::from_execution_result(&result, None);

        let output = ReportGenerator::to_human_readable(&report);
        assert!(output.contains("test-pipeline"));
        assert!(output.contains("Build"));
        assert!(output.contains("SUCCESS"));
    }

    #[test]
    fn test_report_to_summary() {
        let result = create_test_execution_result(true);
        let report = ExecutionReport::from_execution_result(&result, None);

        let summary = ReportGenerator::to_summary(&report);
        assert!(summary.contains("✅"));
        assert!(summary.contains("test-pipeline"));
    }

    #[test]
    fn test_report_format() {
        let result = create_test_execution_result(true);
        let report = ExecutionReport::from_execution_result(&result, None);

        let json = ReportGenerator::format(&report, ReportFormat::Json).unwrap();
        assert!(json.contains("pipeline_id"));

        let human = ReportGenerator::format(&report, ReportFormat::HumanReadable).unwrap();
        assert!(human.contains("Build"));

        let summary = ReportGenerator::format(&report, ReportFormat::Summary).unwrap();
        assert!(summary.contains("✅") || summary.contains("test-pipeline"));
    }

    #[test]
    fn test_stage_timing_counts() {
        let result = create_test_execution_result(true);
        let stage_timing = StageTiming::from_stage_result(
            "test",
            "Test",
            &result.stage_results[0],
            result.started_at,
        );

        assert_eq!(stage_timing.total_steps(), 1);
        assert_eq!(stage_timing.successful_steps(), 1);
        assert_eq!(stage_timing.failed_steps(), 0);
    }

    #[test]
    fn test_execution_report_retries() {
        let mut result = create_test_execution_result(true);
        result.stage_results[0].retry_count = 2;
        result.total_retries = 2;

        let report = ExecutionReport::from_execution_result(&result, None);
        assert_eq!(report.total_retries, 2);
        assert_eq!(report.stage_timings[0].retry_count, 2);
    }

    #[test]
    fn test_execution_report_with_env_snapshot() {
        use pipeliner_core::spec::EnvSpec;

        let result = create_test_execution_result(true);
        let env = EnvSpec::new()
            .with_var("FOO", "bar")
            .with_var("BAZ", "qux");

        let report = ExecutionReport::from_execution_result(&result, Some(env.clone()));
        assert!(report.env_snapshot.is_some());
        assert_eq!(report.env_snapshot.as_ref().unwrap().get("FOO"), Some("bar"));
    }

    #[test]
    fn test_report_duration_ms() {
        let result = create_test_execution_result(true);
        let report = ExecutionReport::from_execution_result(&result, None);

        assert!(report.total_duration_ms > 0);
        assert_eq!(
            report.total_duration_ms,
            (report.total_duration_secs * 1000.0) as i64
        );
    }
}
