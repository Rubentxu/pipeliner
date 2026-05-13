//! # Execution Report
//! Structured execution result with stage/step hierarchy.

use serde::Serialize;

/// Full pipeline execution report
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionReport {
    pub pipeline_name: String,
    pub success: bool,
    pub total_duration_ms: u64,
    pub stages: Vec<StageReport>,
}

/// Stage execution report
#[derive(Debug, Clone, Serialize)]
pub struct StageReport {
    pub name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub skipped: bool,
    pub steps: Vec<StepReport>,
}

/// Step execution report
#[derive(Debug, Clone, Serialize)]
pub struct StepReport {
    pub name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub output: String,
}

impl ExecutionReport {
    /// Creates a new execution report for a pipeline
    pub fn new(pipeline_name: &str) -> Self {
        Self {
            pipeline_name: pipeline_name.to_string(),
            success: true,
            total_duration_ms: 0,
            stages: Vec::new(),
        }
    }

    /// Sets the total duration in milliseconds
    pub fn with_total_duration(mut self, ms: u64) -> Self {
        self.total_duration_ms = ms;
        self
    }

    /// Adds a stage report to the execution report
    pub fn add_stage(&mut self, stage: StageReport) {
        if !stage.success {
            self.success = false;
        }
        self.stages.push(stage);
    }

    /// Returns the number of stages in this report
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Returns the total number of steps across all stages
    pub fn step_count(&self) -> usize {
        self.stages.iter().map(|s| s.steps.len()).sum()
    }
}

impl StageReport {
    /// Creates a new stage report
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            success: true,
            duration_ms: 0,
            skipped: false,
            steps: Vec::new(),
        }
    }

    /// Sets the stage duration
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Creates a skipped stage report
    pub fn skipped(name: &str) -> Self {
        Self {
            name: name.to_string(),
            success: true,
            duration_ms: 0,
            skipped: true,
            steps: Vec::new(),
        }
    }

    /// Adds a step report to this stage
    pub fn add_step(&mut self, step: StepReport) {
        if !step.success {
            self.success = false;
        }
        self.steps.push(step);
    }
}

impl StepReport {
    /// Creates a new step report
    pub fn new(name: &str, success: bool, duration_ms: u64, output: String) -> Self {
        Self {
            name: name.to_string(),
            success,
            duration_ms,
            output,
        }
    }

    /// Creates a step report from a LocalResult
    pub fn from_local_result(result: &crate::LocalResult) -> Self {
        // LocalResult.stage is actually the step name (legacy naming)
        Self {
            name: result.stage.clone(), // This IS the step name
            success: result.success,
            duration_ms: result.duration_ms,
            output: result.output.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_report_new() {
        let report = ExecutionReport::new("test-pipeline");
        assert_eq!(report.pipeline_name, "test-pipeline");
        assert!(report.success);
        assert_eq!(report.total_duration_ms, 0);
        assert!(report.stages.is_empty());
    }

    #[test]
    fn test_execution_report_with_total_duration() {
        let report = ExecutionReport::new("test-pipeline").with_total_duration(1000);
        assert_eq!(report.total_duration_ms, 1000);
    }

    #[test]
    fn test_execution_report_add_stage() {
        let mut report = ExecutionReport::new("test-pipeline");
        let mut stage = StageReport::new("build");
        stage.add_step(StepReport::new("step1", true, 100, "output".to_string()));
        report.add_stage(stage);

        assert_eq!(report.stage_count(), 1);
        assert_eq!(report.step_count(), 1);
    }

    #[test]
    fn test_execution_report_add_failing_stage() {
        let mut report = ExecutionReport::new("test-pipeline");
        let mut stage = StageReport::new("build");
        stage.add_step(StepReport::new("step1", false, 100, "error".to_string()));
        report.add_stage(stage);

        assert!(!report.success);
        assert_eq!(report.stage_count(), 1);
    }

    #[test]
    fn test_stage_count_empty() {
        let report = ExecutionReport::new("test-pipeline");
        assert_eq!(report.stage_count(), 0);
    }

    #[test]
    fn test_step_count_empty() {
        let report = ExecutionReport::new("test-pipeline");
        assert_eq!(report.step_count(), 0);
    }

    #[test]
    fn test_step_count_multiple_stages() {
        let mut report = ExecutionReport::new("test-pipeline");
        let mut stage1 = StageReport::new("build");
        stage1.add_step(StepReport::new("step1", true, 100, "output".to_string()));
        stage1.add_step(StepReport::new("step2", true, 100, "output".to_string()));
        report.add_stage(stage1);

        let mut stage2 = StageReport::new("test");
        stage2.add_step(StepReport::new("step3", true, 100, "output".to_string()));
        report.add_stage(stage2);

        assert_eq!(report.stage_count(), 2);
        assert_eq!(report.step_count(), 3);
    }

    #[test]
    fn test_stage_report_new() {
        let stage = StageReport::new("build");
        assert_eq!(stage.name, "build");
        assert!(stage.success);
        assert_eq!(stage.duration_ms, 0);
        assert!(!stage.skipped);
        assert!(stage.steps.is_empty());
    }

    #[test]
    fn test_stage_report_with_duration() {
        let stage = StageReport::new("build").with_duration(500);
        assert_eq!(stage.duration_ms, 500);
    }

    #[test]
    fn test_stage_report_skipped() {
        let stage = StageReport::skipped("build");
        assert_eq!(stage.name, "build");
        assert!(stage.success);
        assert!(stage.skipped);
        assert!(stage.steps.is_empty());
    }

    #[test]
    fn test_stage_report_add_step() {
        let mut stage = StageReport::new("build");
        stage.add_step(StepReport::new("step1", true, 100, "output".to_string()));
        stage.add_step(StepReport::new("step2", false, 50, "error".to_string()));

        assert_eq!(stage.steps.len(), 2);
        assert!(!stage.success); // One step failed
    }

    #[test]
    fn test_step_report_new() {
        let step = StepReport::new("test-step", true, 250, "success output".to_string());
        assert_eq!(step.name, "test-step");
        assert!(step.success);
        assert_eq!(step.duration_ms, 250);
        assert_eq!(step.output, "success output");
    }

    #[test]
    fn test_step_report_from_local_result() {
        let local_result = crate::LocalResult {
            success: true,
            stage: "echo-step".to_string(),
            output: "Hello".to_string(),
            duration_ms: 100,
        };
        let step = StepReport::from_local_result(&local_result);
        assert_eq!(step.name, "echo-step");
        assert!(step.success);
        assert_eq!(step.duration_ms, 100);
        assert_eq!(step.output, "Hello");
    }

    #[test]
    fn test_execution_report_serialization() {
        let mut report = ExecutionReport::new("test-pipeline");
        let mut stage = StageReport::new("build");
        stage.add_step(StepReport::new("step1", true, 100, "output".to_string()));
        report.add_stage(stage);

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"pipeline_name\":\"test-pipeline\""));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"name\":\"build\""));
        assert!(json.contains("\"name\":\"step1\""));
    }

    #[test]
    fn test_stage_report_serialization() {
        let stage = StageReport::new("test-stage");
        let json = serde_json::to_string(&stage).unwrap();
        assert!(json.contains("\"name\":\"test-stage\""));
        assert!(json.contains("\"skipped\":false"));
    }

    #[test]
    fn test_step_report_serialization() {
        let step = StepReport::new("test-step", true, 50, "test output".to_string());
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"name\":\"test-step\""));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"duration_ms\":50"));
        assert!(json.contains("\"output\":\"test output\""));
    }
}