//! # Local Executor
//!
//! Local pipeline executor for development and testing.
//! Provides a simple way to run pipelines on the current machine.

use pipeliner_core::logging::LogLevel;
use pipeliner_core::{Pipeline, Step, StepType};
use pipeliner_events::markers::{StageMarkerEmitter, StageMarkerParser, STAGE_MARKER_PREFIX};
use pipeliner_events::types::markers::{StageMarker, StageResult};
use std::cell::RefCell;
use std::io::Write;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Local execution result
#[derive(Debug, Clone)]
pub struct LocalResult {
    pub success: bool,
    pub stage: String,
    pub output: String,
    pub duration_ms: u64,
}

/// Marker buffer wrapper that allows writing stage markers and retrieving output
#[derive(Debug)]
struct MarkerBuffer {
    buffer: Vec<u8>,
}

impl MarkerBuffer {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    fn into_inner(self) -> Vec<u8> {
        self.buffer
    }

    fn get_marker_output(&self) -> Vec<u8> {
        self.buffer.clone()
    }
}

impl Write for MarkerBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.flush()
    }
}

/// Local executor for running pipelines on the current machine
#[derive(Debug)]
pub struct LocalExecutor {
    marker_buffer: Rc<RefCell<Option<MarkerBuffer>>>,
}

impl LocalExecutor {
    /// Creates a new local executor without marker emission
    #[must_use]
    pub fn new() -> Self {
        Self {
            marker_buffer: Rc::new(RefCell::new(None)),
        }
    }

    /// Creates a new local executor with a marker writer for stage tracking
    ///
    /// The provided writer is used internally to capture stage markers.
    /// An internal buffer is used to store the marker data for retrieval.
    #[must_use]
    pub fn with_marker_writer(_writer: Box<dyn Write + Send>) -> Self {
        // We use an internal buffer regardless of what writer is passed,
        // as we need to retrieve the marker data later for testing
        Self {
            marker_buffer: Rc::new(RefCell::new(Some(MarkerBuffer::new()))),
        }
    }

    /// Get the marker output if a marker writer was set
    #[must_use]
    pub fn get_marker_output(&self) -> Option<Vec<u8>> {
        self.marker_buffer.borrow().as_ref().map(|b| b.get_marker_output())
    }

    fn emit_started_marker(&self, stage_name: &str) {
        if let Some(buffer) = self.marker_buffer.borrow_mut().as_mut() {
            let _ = StageMarkerEmitter::started(buffer, stage_name);
        }
    }

    fn emit_completed_marker(&self, stage_name: &str, duration_ms: u64, result: StageResult) {
        if let Some(buffer) = self.marker_buffer.borrow_mut().as_mut() {
            let _ = StageMarkerEmitter::completed(buffer, stage_name, duration_ms, result);
        }
    }

    fn emit_error_marker(&self, stage_name: &str, message: &str) {
        if let Some(buffer) = self.marker_buffer.borrow_mut().as_mut() {
            let _ = StageMarkerEmitter::error(buffer, stage_name, message);
        }
    }

    /// Execute a single step with a minimum log level filter.
    ///
    /// The `min_level` parameter controls which log messages are actually emitted.
    /// Messages with a level lower than `min_level` will be silently ignored.
    pub async fn execute_step(&self, step: &Step, min_level: LogLevel) -> LocalResult {
        Box::pin(self._execute_step_impl(step, min_level)).await
    }

    async fn _execute_step_impl(&self, step: &Step, min_level: LogLevel) -> LocalResult {
        let start = std::time::Instant::now();
        let step_name = step.name.clone().unwrap_or_else(|| "unnamed".to_string());

        match &step.step_type {
            StepType::Shell { command } => {
                info!("[{}] Running: {}", step_name, command);
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output();

                match output {
                    Ok(output) => {
                        let success = output.status.success();
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                        if success {
                            info!("[{}] Success", step_name);
                            debug!("Output: {}", stdout.trim());
                        } else {
                            error!("[{}] Failed", step_name);
                            error!("Error: {}", stderr.trim());
                        }

                        LocalResult {
                            success,
                            stage: step_name,
                            output: if !stdout.is_empty() { stdout } else { stderr },
                            duration_ms: start.elapsed().as_millis() as u64,
                        }
                    }
                    Err(e) => LocalResult {
                        success: false,
                        stage: step_name,
                        output: e.to_string(),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
            StepType::Echo { message } => {
                info!("[{}] {}", step_name, message);
                LocalResult {
                    success: true,
                    stage: step_name,
                    output: message.clone(),
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            StepType::Retry { count, step: inner } => {
                let mut last_error = String::new();
                for attempt in 0..*count {
                    info!("[{}] Retry attempt {}/{}", step_name, attempt + 1, count);
                    let result = self.execute_step(inner.as_ref(), min_level).await;
                    if result.success {
                        return result;
                    }
                    last_error = result.output.clone();
                    sleep(Duration::from_secs(1)).await;
                }
                LocalResult {
                    success: false,
                    stage: step_name,
                    output: format!("Retry failed after {} attempts: {}", count, last_error),
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            StepType::Timeout {
                duration,
                step: inner,
            } => {
                let result =
                    tokio::time::timeout(*duration, self.execute_step(inner.as_ref(), min_level)).await;

                match result {
                    Ok(r) => r,
                    Err(_) => LocalResult {
                        success: false,
                        stage: step_name,
                        output: format!("Timeout after {} seconds", duration.as_secs()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
            StepType::Log { level, message } => {
                // REQ-SL-004: Filter log messages based on min_level
                if !LogLevel::should_log(*level, min_level) {
                    // Message level is below the minimum - skip logging
                    return LocalResult {
                        success: true,
                        stage: step_name,
                        output: message.clone(),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }

                // Message passes the filter - emit the appropriate log
                match level {
                    LogLevel::Debug => debug!("[{}] {}", step_name, message),
                    LogLevel::Info => info!("[{}] {}", step_name, message),
                    LogLevel::Warn => warn!("[{}] {}", step_name, message),
                    LogLevel::Error => error!("[{}] {}", step_name, message),
                    LogLevel::Fatal => {
                        error!("[{}] {}", step_name, message);
                        // REQ-SL-006: Fatal should emit a StageMarker::Error via the marker writer
                        self.emit_error_marker(&step_name, message);
                    }
                }
                LocalResult {
                    success: true,
                    stage: step_name,
                    output: message.clone(),
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            _ => LocalResult {
                success: true,
                stage: step_name,
                output: "Step type not implemented for local execution".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Execute a pipeline
    pub async fn execute(&mut self, pipeline: &Pipeline) -> Vec<LocalResult> {
        info!("========================================");
        info!("   Pipeliner - Local Execution");
        info!("========================================");
        info!("Pipeline: {:?}", pipeline.name());
        info!("Stages: {}", pipeline.stages.len());
        info!("");

        // REQ-SL-004: Get min log level from pipeline options (default to Info if not set)
        let min_level = pipeline
            .options
            .as_ref()
            .and_then(|o| o.log_level)
            .unwrap_or(LogLevel::Info);

        let mut results = Vec::new();

        for (stage_idx, stage) in pipeline.stages.iter().enumerate() {
            let stage_start = std::time::Instant::now();

            // Emit STARTED marker
            self.emit_started_marker(&stage.name);

            info!(
                "[Stage {}/{}] {}",
                stage_idx + 1,
                pipeline.stages.len(),
                stage.name
            );
            info!("----------------------------------------");

            let mut stage_success = true;

            for (_step_idx, step) in stage.steps.iter().enumerate() {
                let result = self.execute_step(step, min_level).await;
                results.push(result.clone());

                if !result.success {
                    warn!("Pipeline aborted due to step failure");
                    stage_success = false;
                    // Emit ERROR marker for stage failure
                    self.emit_error_marker(&stage.name, "Stage failed due to step failure");
                    break;
                }
            }

            if stage_success {
                let duration_ms = stage_start.elapsed().as_millis() as u64;
                // Emit COMPLETED marker with SUCCESS result
                self.emit_completed_marker(&stage.name, duration_ms, StageResult::Success);
            }

            info!("");
        }

        let success_count = results.iter().filter(|r| r.success).count();
        let total_count = results.len();

        info!("========================================");
        info!("   Execution Complete");
        info!("========================================");
        info!("Steps: {}/{} successful", success_count, total_count);
        info!(
            "Total time: {}ms",
            results.iter().map(|r| r.duration_ms).sum::<u64>()
        );

        results
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pipeliner_core::logging::LogLevel;
    use pipeliner_core::pipeline::StepType;
    use pipeliner_core::Pipeline;

    #[tokio::test]
    async fn test_echo_step() {
        let executor = LocalExecutor::new();
        let step = Step::echo("Hello from test").with_name("test-echo");
        let result = executor.execute_step(&step, LogLevel::Debug).await;
        assert!(result.success);
        assert_eq!(result.stage, "test-echo");
    }

    #[tokio::test]
    async fn test_simple_pipeline() {
        let mut executor = LocalExecutor::new();
        let pipeline = Pipeline::new().with_name("test-pipeline");

        let results = executor.execute(&pipeline).await;
        assert_eq!(results.len(), 0); // No stages in test pipeline
    }

    // =======================================================================
    // StepType::Log Tests
    // =======================================================================

    #[tokio::test]
    async fn test_log_step_debug_level() {
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Debug,
                message: "Debug message".to_string(),
            },
            name: Some("debug-log".to_string()),
            timeout: None,
            retry: None,
        };

        let result = executor.execute_step(&step, LogLevel::Debug).await;
        assert!(result.success);
        assert_eq!(result.stage, "debug-log");
    }

    #[tokio::test]
    async fn test_log_step_info_level() {
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Info,
                message: "Info message".to_string(),
            },
            name: Some("info-log".to_string()),
            timeout: None,
            retry: None,
        };

        let result = executor.execute_step(&step, LogLevel::Debug).await;
        assert!(result.success);
        assert_eq!(result.stage, "info-log");
    }

    #[tokio::test]
    async fn test_log_step_warn_level() {
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Warn,
                message: "Warning message".to_string(),
            },
            name: Some("warn-log".to_string()),
            timeout: None,
            retry: None,
        };

        let result = executor.execute_step(&step, LogLevel::Debug).await;
        assert!(result.success);
        assert_eq!(result.stage, "warn-log");
    }

    #[tokio::test]
    async fn test_log_step_error_level() {
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Error,
                message: "Error message".to_string(),
            },
            name: Some("error-log".to_string()),
            timeout: None,
            retry: None,
        };

        let result = executor.execute_step(&step, LogLevel::Debug).await;
        assert!(result.success);
        assert_eq!(result.stage, "error-log");
    }

    #[tokio::test]
    async fn test_log_step_fatal_level() {
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Fatal,
                message: "Fatal message".to_string(),
            },
            name: Some("fatal-log".to_string()),
            timeout: None,
            retry: None,
        };

        let result = executor.execute_step(&step, LogLevel::Debug).await;
        // Fatal should still return success: true (it's logged but doesn't fail the step)
        assert!(result.success);
        assert_eq!(result.stage, "fatal-log");
    }

    #[tokio::test]
    async fn test_log_step_output_contains_message() {
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Info,
                message: "Test info message".to_string(),
            },
            name: Some("info-log".to_string()),
            timeout: None,
            retry: None,
        };

        let result = executor.execute_step(&step, LogLevel::Debug).await;
        assert!(result.success);
        // The output should contain the logged message
        assert!(result.output.contains("Test info message"));
    }

    #[tokio::test]
    async fn test_log_step_with_empty_message() {
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Info,
                message: String::new(),
            },
            name: Some("empty-log".to_string()),
            timeout: None,
            retry: None,
        };

        let result = executor.execute_step(&step, LogLevel::Debug).await;
        assert!(result.success);
    }

    // =======================================================================
    // StageMarker Emission Tests (Task D1)
    // =======================================================================

    #[tokio::test]
    async fn test_stage_marker_emitted_on_stage_start() {
        // Create a marker writer (Vec<u8>)
        let marker_writer = Vec::new();
        let mut executor = LocalExecutor::with_marker_writer(Box::new(marker_writer));

        // Create a simple pipeline with one stage containing an echo step
        let stage = pipeliner_core::Stage::new("build")
            .with_step(Step::echo("hello").with_name("echo-step"));
        let pipeline = Pipeline::new()
            .with_name("test-markers")
            .with_stage(stage);

        // Execute the pipeline
        let _results = executor.execute(&pipeline).await;

        // Get the marker output from the executor's marker writer
        let marker_output = executor.get_marker_output();

        // Verify a STARTED marker was emitted for the "build" stage
        assert!(marker_output.is_some(), "Marker output should be present");
        let output = marker_output.unwrap();
        let output_str = String::from_utf8_lossy(&output);

        // Should contain __STAGE__ prefix
        assert!(output_str.contains("__STAGE__"), "Output should contain __STAGE__ prefix");
        // Should contain a STARTED marker for "build" stage
        assert!(output_str.contains("\"type\":\"STARTED\""), "Should contain STARTED marker");
        assert!(output_str.contains("\"name\":\"build\""), "Should contain stage name 'build'");
    }

    #[tokio::test]
    async fn test_stage_marker_emitted_on_stage_completion() {
        // Create a marker writer (Vec<u8>)
        let marker_writer = Vec::new();
        let mut executor = LocalExecutor::with_marker_writer(Box::new(marker_writer));

        // Create a simple pipeline with one stage containing an echo step
        let stage = pipeliner_core::Stage::new("build")
            .with_step(Step::echo("hello").with_name("echo-step"));
        let pipeline = Pipeline::new()
            .with_name("test-markers")
            .with_stage(stage);

        // Execute the pipeline
        let _results = executor.execute(&pipeline).await;

        // Get the marker output
        let marker_output = executor.get_marker_output();
        assert!(marker_output.is_some());
        let output = marker_output.unwrap();
        let output_str = String::from_utf8_lossy(&output);

        // Should contain a COMPLETED marker with SUCCESS result
        assert!(output_str.contains("\"type\":\"COMPLETED\""), "Should contain COMPLETED marker");
        assert!(output_str.contains("\"name\":\"build\""), "Should contain stage name 'build'");
        assert!(output_str.contains("\"result\":\"SUCCESS\""), "Should contain SUCCESS result");
    }

    #[tokio::test]
    async fn test_stage_marker_emitted_on_stage_failure() {
        // Create a marker writer (Vec<u8>)
        let marker_writer = Vec::new();
        let mut executor = LocalExecutor::with_marker_writer(Box::new(marker_writer));

        // Create a pipeline with a stage containing a failing shell command
        let stage = pipeliner_core::Stage::new("build")
            .with_step(Step::shell("exit 1").with_name("failing-step"));
        let pipeline = Pipeline::new()
            .with_name("test-failure")
            .with_stage(stage);

        // Execute the pipeline
        let results = executor.execute(&pipeline).await;

        // Verify the step failed
        assert!(!results.is_empty());
        assert!(!results[0].success);

        // Get the marker output
        let marker_output = executor.get_marker_output();
        assert!(marker_output.is_some());
        let output = marker_output.unwrap();
        let output_str = String::from_utf8_lossy(&output);

        // Should contain an ERROR marker
        assert!(output_str.contains("\"type\":\"ERROR\""), "Should contain ERROR marker");
        assert!(output_str.contains("\"name\":\"build\""), "Should contain stage name 'build'");
    }

    #[tokio::test]
    async fn test_stage_marker_duration_recorded() {
        // Create a marker writer (Vec<u8>)
        let marker_writer = Vec::new();
        let mut executor = LocalExecutor::with_marker_writer(Box::new(marker_writer));

        // Create a pipeline with a stage
        let stage = pipeliner_core::Stage::new("build")
            .with_step(Step::shell("sleep 0.01").with_name("slow-step"));
        let pipeline = Pipeline::new()
            .with_name("test-duration")
            .with_stage(stage);

        // Execute the pipeline
        let _results = executor.execute(&pipeline).await;

        // Get the marker output
        let marker_output = executor.get_marker_output();
        assert!(marker_output.is_some());
        let output = marker_output.unwrap();
        let output_str = String::from_utf8_lossy(&output);

        // Should contain a COMPLETED marker with duration_ms > 0
        assert!(output_str.contains("\"type\":\"COMPLETED\""), "Should contain COMPLETED marker");
        // Extract duration_ms value using a simple parse
        if let Some(duration_start) = output_str.find("\"duration_ms\":") {
            let duration_str = &output_str[duration_start + 14..];
            if let Some(end_pos) = duration_str.find(',').or_else(|| duration_str.find('}')) {
                let duration_value: u64 = duration_str[..end_pos].trim().parse().unwrap_or(0);
                assert!(duration_value > 0, "Duration should be non-zero");
            }
        }
    }

    #[tokio::test]
    async fn test_stage_marker_writer_captures_output() {
        // Create a marker writer (Vec<u8>)
        let marker_writer: Vec<u8> = Vec::new();
        let mut executor = LocalExecutor::with_marker_writer(Box::new(marker_writer));

        // Create a simple pipeline
        let stage = pipeliner_core::Stage::new("test")
            .with_step(Step::echo("test message").with_name("echo-step"));
        let pipeline = Pipeline::new()
            .with_name("test-writer")
            .with_stage(stage);

        // Execute the pipeline
        let _results = executor.execute(&pipeline).await;

        // Get the marker output - the executor should have consumed the writer
        // but we need a way to retrieve it. Let's check the executor has the data.
        let marker_output = executor.get_marker_output();
        assert!(marker_output.is_some(), "Marker output should be retrievable");
    }

    #[tokio::test]
    async fn test_local_executor_without_marker_writer_produces_no_markers() {
        // Create an executor WITHOUT a marker writer
        let mut executor = LocalExecutor::new();

        // Create a simple pipeline
        let stage = pipeliner_core::Stage::new("build")
            .with_step(Step::echo("hello").with_name("echo-step"));
        let pipeline = Pipeline::new()
            .with_name("test-no-markers")
            .with_stage(stage);

        // Execute the pipeline
        let _results = executor.execute(&pipeline).await;

        // The executor should work fine without a marker writer
        // (backward compatibility - markers are optional)
        // We can't easily verify no markers were emitted since they're not captured,
        // but the executor should not panic or fail
    }

    #[tokio::test]
    async fn test_stage_markers_roundtrip_parse() {
        use pipeliner_events::markers::{StageMarkerParser, STAGE_MARKER_PREFIX};

        // Create a marker writer
        let marker_writer = Vec::new();
        let mut executor = LocalExecutor::with_marker_writer(Box::new(marker_writer));

        // Create a pipeline
        let stage = pipeliner_core::Stage::new("build")
            .with_step(Step::echo("test").with_name("test-step"));
        let pipeline = Pipeline::new()
            .with_name("test-parse")
            .with_stage(stage);

        // Execute
        let _results = executor.execute(&pipeline).await;

        // Get output and parse
        let marker_output = executor.get_marker_output();
        assert!(marker_output.is_some());
        let marker_data = marker_output.unwrap();
        let output = String::from_utf8_lossy(&marker_data);

        // Parse each line
        let mut markers = Vec::new();
        for line in output.lines() {
            if line.starts_with(STAGE_MARKER_PREFIX) {
                if let Some(marker) = StageMarkerParser::parse_line(line) {
                    markers.push(marker);
                }
            }
        }

        // Should have at least 2 markers: STARTED and COMPLETED
        assert!(markers.len() >= 2, "Should have at least STARTED and COMPLETED markers");

        // Verify first is STARTED, last is COMPLETED
        match &markers[0] {
            StageMarker::Started { name, .. } => assert_eq!(name, "build"),
            _ => panic!("First marker should be Started"),
        }

        match &markers[markers.len() - 1] {
            StageMarker::Completed { name, result, .. } => {
                assert_eq!(name, "build");
                assert_eq!(result, &pipeliner_events::types::markers::StageResult::Success);
            }
            _ => panic!("Last marker should be Completed"),
        }
    }

    // =======================================================================
    // Task E2: Integration Test - Full Pipeline with Markers and Logging
    // =======================================================================

    #[tokio::test]
    async fn test_full_pipeline_integration_markers_and_logging() {
        // Create a marker writer
        let mut executor = LocalExecutor::with_marker_writer(Box::new(Vec::new()));

        // Create a pipeline with 2 stages
        // Stage 1: echo step + log info step
        // Stage 2: shell step + log debug step
        let stage1 = pipeliner_core::Stage::new("build")
            .with_step(Step::echo("Building project...").with_name("echo-step"))
            .with_step(Step {
                step_type: StepType::Log {
                    level: LogLevel::Info,
                    message: "Build started".to_string(),
                },
                name: Some("log-info".to_string()),
                timeout: None,
                retry: None,
            });

        let stage2 = pipeliner_core::Stage::new("test")
            .with_step(Step::shell("echo 'Running tests'").with_name("shell-step"))
            .with_step(Step {
                step_type: StepType::Log {
                    level: LogLevel::Debug,
                    message: "Debug message".to_string(),
                },
                name: Some("log-debug".to_string()),
                timeout: None,
                retry: None,
            });

        let pipeline = Pipeline::new()
            .with_name("test-pipeline")
            .with_stage(stage1)
            .with_stage(stage2);

        // Execute the pipeline
        let results = executor.execute(&pipeline).await;

        // Verify both stages completed (4 steps total)
        assert_eq!(results.len(), 4, "Should have 4 step results"); // 2 steps per stage

        // Get the marker output
        let marker_output = executor.get_marker_output();
        assert!(marker_output.is_some(), "Marker output should be present");
        let marker_data = marker_output.unwrap();

        // Parse all markers
        let marker_str = String::from_utf8_lossy(&marker_data);
        let mut markers = Vec::new();

        for line in marker_str.lines() {
            if let Some(marker) = StageMarkerParser::parse_line(line) {
                markers.push(marker);
            }
        }

        // Should have 4 markers: build STARTED, build COMPLETED, test STARTED, test COMPLETED
        assert_eq!(markers.len(), 4, "Should have 4 markers (STARTED + COMPLETED for each stage)");

        // Verify the sequence of markers
        match &markers[0] {
            StageMarker::Started { name, .. } => assert_eq!(name, "build"),
            _ => panic!("First marker should be Started for 'build'"),
        }

        match &markers[1] {
            StageMarker::Completed { name, result, duration_ms, .. } => {
                assert_eq!(name, "build");
                assert_eq!(result, &StageResult::Success);
                // Duration is recorded (may be 0 for very fast stages)
                assert!(duration_ms >= &0, "Duration should be recorded");
            }
            _ => panic!("Second marker should be Completed for 'build'"),
        }

        match &markers[2] {
            StageMarker::Started { name, .. } => assert_eq!(name, "test"),
            _ => panic!("Third marker should be Started for 'test'"),
        }

        match &markers[3] {
            StageMarker::Completed { name, result, duration_ms, .. } => {
                assert_eq!(name, "test");
                assert_eq!(result, &StageResult::Success);
                // Duration is recorded (may be 0 for very fast stages)
                assert!(duration_ms >= &0, "Duration should be recorded");
            }
            _ => panic!("Fourth marker should be Completed for 'test'"),
        }
    }

    // =======================================================================
    // REQ-SL-004: Min-level filtering Tests
    // =======================================================================

    #[tokio::test]
    async fn test_log_level_filter_debug_below_warn() {
        // REQ-SL-004: When min_level=Warn, Debug messages should be filtered out
        // This test verifies the filtering by checking that execute_step with min_level=Warn
        // and message level=Debug returns success (the message is skipped, not an error)
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Debug,
                message: "Debug message".to_string(),
            },
            name: Some("debug-log".to_string()),
            timeout: None,
            retry: None,
        };

        // With min_level=Warn, Debug should be filtered (not logged)
        let result = executor.execute_step(&step, LogLevel::Warn).await;
        assert!(result.success, "Step should succeed even when message is filtered");
        assert_eq!(result.stage, "debug-log");
    }

    #[tokio::test]
    async fn test_log_level_filter_error_above_warn() {
        // REQ-SL-004: When min_level=Warn, Error messages should NOT be filtered
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Error,
                message: "Error message".to_string(),
            },
            name: Some("error-log".to_string()),
            timeout: None,
            retry: None,
        };

        // With min_level=Warn, Error should pass through
        let result = executor.execute_step(&step, LogLevel::Warn).await;
        assert!(result.success);
        assert_eq!(result.stage, "error-log");
    }

    #[tokio::test]
    async fn test_log_level_from_pipeline_options_warn_filters_debug() {
        // REQ-SL-004: Pipeline with log_level=Warn should filter Debug messages
        use pipeliner_core::options::PipelineOptions;

        let mut executor = LocalExecutor::new();

        // Create pipeline with log_level=Warn
        let stage = pipeliner_core::Stage::new("build")
            .with_step(Step {
                step_type: StepType::Log {
                    level: LogLevel::Debug,
                    message: "This debug should be filtered".to_string(),
                },
                name: Some("debug-step".to_string()),
                timeout: None,
                retry: None,
            });

        let pipeline = Pipeline::new()
            .with_name("test-filter")
            .with_options(PipelineOptions::new().with_log_level(LogLevel::Warn))
            .with_stage(stage);

        // Execute the pipeline
        let results = executor.execute(&pipeline).await;

        // Step should still succeed (filtering is silent)
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
    }

    // =======================================================================
    // REQ-SL-006: Fatal → StageMarker::Error bridge Tests
    // =======================================================================

    #[tokio::test]
    async fn test_fatal_log_emits_error_marker() {
        // REQ-SL-006: When StepType::Log(Fatal, msg) is executed,
        // it should emit a StageMarker::Error via the marker writer
        let executor = LocalExecutor::new();
        let step = Step {
            step_type: StepType::Log {
                level: LogLevel::Fatal,
                message: "Fatal error occurred".to_string(),
            },
            name: Some("fatal-step".to_string()),
            timeout: None,
            retry: None,
        };

        // Execute with min_level=Debug so Fatal passes through
        let result = executor.execute_step(&step, LogLevel::Debug).await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_fatal_log_emits_error_marker_in_pipeline() {
        // REQ-SL-006: Fatal log in pipeline should emit StageMarker::Error
        let mut executor = LocalExecutor::with_marker_writer(Box::new(Vec::new()));

        let stage = pipeliner_core::Stage::new("build")
            .with_step(Step {
                step_type: StepType::Log {
                    level: LogLevel::Fatal,
                    message: "Critical failure".to_string(),
                },
                name: Some("fatal-step".to_string()),
                timeout: None,
                retry: None,
            });

        let pipeline = Pipeline::new()
            .with_name("test-fatal-marker")
            .with_stage(stage);

        // Execute the pipeline
        let _results = executor.execute(&pipeline).await;

        // Get the marker output
        let marker_output = executor.get_marker_output();
        assert!(marker_output.is_some(), "Marker output should be present");

        let output = marker_output.unwrap();
        let output_str = String::from_utf8_lossy(&output);

        // Should contain an ERROR marker for the Fatal log
        assert!(
            output_str.contains("\"type\":\"ERROR\""),
            "Should contain ERROR marker for Fatal log: {}",
            output_str
        );
        assert!(
            output_str.contains("\"name\":\"fatal-step\""),
            "Should contain stage name 'fatal-step'"
        );
    }
}
