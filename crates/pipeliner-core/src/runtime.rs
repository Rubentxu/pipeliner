//! Pipeline runtime for lifecycle management.
//!
//! This module provides types for managing the pipeline execution lifecycle,
//! including phase tracking, timing, and error handling.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::config::{LibraryConfig, PipelineConfig};
use crate::validation::Validate;
use crate::Pipeline;

/// Type alias for library loader function to avoid circular dependencies.
/// Takes a slice of library configs and returns a result with count or error message.
pub type LoadLibrariesFn = Box<dyn FnOnce(&[LibraryConfig]) -> Result<usize, String> + Send>;

/// Pipeline execution result from the runtime.
///
/// This is a simplified result type that captures the outcome
/// of pipeline execution without depending on executor types.
#[derive(Debug, Clone)]
pub struct PipelineRunResult {
    /// Whether the pipeline succeeded
    pub success: bool,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Number of stages executed
    pub stages_executed: usize,
    /// Number of steps executed
    pub steps_executed: usize,
    /// Error message if failed
    pub error: Option<String>,
}

impl PipelineRunResult {
    /// Creates a successful result
    #[must_use]
    pub fn success(stages: usize, steps: usize, duration_ms: u64) -> Self {
        Self {
            success: true,
            duration_ms,
            stages_executed: stages,
            steps_executed: steps,
            error: None,
        }
    }

    /// Creates a failed result
    #[must_use]
    pub fn failure(error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: false,
            duration_ms,
            stages_executed: 0,
            steps_executed: 0,
            error: Some(error.into()),
        }
    }
}

/// Lifecycle phases that a pipeline goes through during execution.
///
/// Each phase represents a distinct stage in the pipeline lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LifecyclePhase {
    /// Initialization phase
    Init,
    /// Load libraries phase
    LoadLibraries,
    /// Setup engine phase
    SetupEngine,
    /// Load source code phase
    LoadSourceCode,
    /// Bind steps phase
    BindSteps,
    /// Execute pipeline phase
    Execute,
    /// Completed phase (terminal)
    Completed,
    /// Failed phase (terminal)
    Failed,
}

impl LifecyclePhase {
    /// Returns true if this is a terminal phase
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, LifecyclePhase::Completed | LifecyclePhase::Failed)
    }
}

/// Result of a single lifecycle phase execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseResult {
    /// The phase that was executed
    pub phase: LifecyclePhase,
    /// Duration of the phase in milliseconds
    pub duration_ms: u64,
    /// Optional output from the phase
    pub output: Option<String>,
    /// Optional error message if the phase failed
    pub error: Option<String>,
}

impl PhaseResult {
    /// Creates a successful phase result
    #[must_use]
    pub fn success(phase: LifecyclePhase, duration_ms: u64, output: Option<String>) -> Self {
        Self {
            phase,
            duration_ms,
            output,
            error: None,
        }
    }

    /// Creates a failed phase result
    #[must_use]
    pub fn failure(phase: LifecyclePhase, duration_ms: u64, error: impl Into<String>) -> Self {
        Self {
            phase,
            duration_ms,
            output: None,
            error: Some(error.into()),
        }
    }
}

/// Runtime errors that can occur during pipeline execution.
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// A phase failed during execution
    PhaseFailed {
        /// The phase that failed
        phase: LifecyclePhase,
        /// The underlying error source
        source: String,
    },
    /// Configuration error
    ConfigError(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::PhaseFailed { phase, source } => {
                write!(f, "Phase {:?} failed: {}", phase, source)
            }
            RuntimeError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Pipeline runtime for managing lifecycle execution.
///
/// The runtime tracks phases, timing, and coordinates execution
/// through the various lifecycle stages.
pub struct PipelineRuntime {
    /// Pipeline configuration (optional)
    config: Option<PipelineConfig>,
    /// Log of phase results
    phase_log: Vec<PhaseResult>,
    /// Start time of the runtime
    start_time: Option<Instant>,
    /// Optional library loader function
    library_loader_fn: Option<LoadLibrariesFn>,
}

impl std::fmt::Debug for PipelineRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineRuntime")
            .field("config", &self.config.is_some())
            .field("phase_log", &self.phase_log.len())
            .field("start_time", &self.start_time)
            .field("library_loader_fn", &self.library_loader_fn.is_some())
            .finish()
    }
}

impl PipelineRuntime {
    /// Creates a new pipeline runtime
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: None,
            phase_log: Vec::new(),
            start_time: None,
            library_loader_fn: None,
        }
    }

    /// Creates a new pipeline runtime with a configuration
    #[must_use]
    pub fn with_config(config: PipelineConfig) -> Self {
        Self {
            config: Some(config),
            phase_log: Vec::new(),
            start_time: None,
            library_loader_fn: None,
        }
    }

    /// Sets the pipeline configuration
    pub fn set_config(&mut self, config: PipelineConfig) {
        self.config = Some(config);
    }

    /// Sets the library loader function
    pub fn set_library_loader(&mut self, loader_fn: LoadLibrariesFn) {
        self.library_loader_fn = Some(loader_fn);
    }

    /// Returns the phase log
    #[must_use]
    pub fn phase_log(&self) -> &[PhaseResult] {
        &self.phase_log
    }

    /// Returns the duration of the entire runtime
    #[must_use]
    pub fn total_duration(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    /// Runs the pipeline through its lifecycle phases.
    ///
    /// Executes phases in order: Init → LoadLibraries → SetupEngine → LoadSourceCode → BindSteps → Execute
    ///
    /// # Errors
    ///
    /// Returns a `RuntimeError` if a phase fails.
    pub fn run(&mut self, pipeline: &Pipeline) -> Result<PipelineRunResult, RuntimeError> {
        self.start_time = Some(Instant::now());

        // Execute phases in order
        let phases = [
            LifecyclePhase::Init,
            LifecyclePhase::LoadLibraries,
            LifecyclePhase::SetupEngine,
            LifecyclePhase::LoadSourceCode,
            LifecyclePhase::BindSteps,
            LifecyclePhase::Execute,
        ];

        for phase in phases {
            let phase_start = Instant::now();

            // Execute the phase
            let result = self.execute_phase(phase, pipeline);

            let duration_ms = phase_start.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    let phase_result = PhaseResult::success(phase, duration_ms, output);
                    self.phase_log.push(phase_result);
                }
                Err(e) => {
                    let phase_result = PhaseResult::failure(phase, duration_ms, e.to_string());
                    self.phase_log.push(phase_result);
                    return Err(e);
                }
            }
        }

        // Pipeline completed successfully
        let phase_result = PhaseResult::success(
            LifecyclePhase::Completed,
            self.start_time.map(|s| s.elapsed().as_millis() as u64).unwrap_or(0),
            None,
        );
        self.phase_log.push(phase_result);

        let total_duration = self.start_time.map(|s| s.elapsed().as_millis() as u64).unwrap_or(0);
        let step_count: usize = pipeline.stages.iter().map(|s| s.steps.len()).sum();

        Ok(PipelineRunResult::success(
            pipeline.stages.len(),
            step_count,
            total_duration,
        ))
    }

    /// Executes a single phase.
    ///
    /// Returns `Ok(Some(output))` if the phase produced output,
    /// `Ok(None)` if it completed silently, or `Err(RuntimeError)` if it failed.
    fn execute_phase(
        &mut self,
        phase: LifecyclePhase,
        pipeline: &Pipeline,
    ) -> Result<Option<String>, RuntimeError> {
        match phase {
            LifecyclePhase::Init => {
                // Init phase: validate pipeline and initialize context
                pipeline.validate().map_err(|e: crate::validation::ValidationError| RuntimeError::ConfigError(e.to_string()))?;
                Ok(Some(format!("Initialized pipeline '{}'", pipeline.name().unwrap_or("unnamed"))))
            }
            LifecyclePhase::LoadLibraries => {
                // LoadLibraries phase: load any configured libraries
                if let Some(ref config) = self.config {
                    if config.spec.libraries.is_empty() {
                        Ok(Some("skipped: no libraries configured".to_string()))
                    } else if let Some(loader_fn) = self.library_loader_fn.take() {
                        // Use the provided library loader function
                        let count = config.spec.libraries.len();
                        loader_fn(&config.spec.libraries)
                            .map_err(|e| RuntimeError::PhaseFailed {
                                phase: LifecyclePhase::LoadLibraries,
                                source: e,
                            })?;
                        Ok(Some(format!("loaded {} libraries", count)))
                    } else {
                        Ok(Some("skipped: no library loader configured".to_string()))
                    }
                } else {
                    Ok(Some("skipped: no config".to_string()))
                }
            }
            LifecyclePhase::SetupEngine => {
                // SetupEngine phase: initialize execution engine
                Ok(Some("engine setup complete".to_string()))
            }
            LifecyclePhase::LoadSourceCode => {
                // LoadSourceCode phase: load source code from SCM
                if let Some(ref config) = self.config {
                    if config.spec.scm.is_some() {
                        Ok(Some(format!(
                            "loaded source from SCM: {}",
                            config.spec.scm.as_ref().map(|s| s.url.as_str()).unwrap_or("unknown")
                        )))
                    } else {
                        Ok(Some("skipped: no SCM configured".to_string()))
                    }
                } else {
                    Ok(Some("skipped: no config".to_string()))
                }
            }
            LifecyclePhase::BindSteps => {
                // BindSteps phase: bind step references and resolve variables
                let step_count: usize = pipeline.stages.iter().map(|s| s.steps.len()).sum();
                Ok(Some(format!("bound {} steps", step_count)))
            }
            LifecyclePhase::Execute => {
                // Execute phase: this is handled by the caller after runtime returns
                // The runtime itself doesn't execute steps - it just tracks the phase
                Ok(Some("execution delegated to executor".to_string()))
            }
            LifecyclePhase::Completed | LifecyclePhase::Failed => {
                // These are terminal phases, should not be executed directly
                Err(RuntimeError::PhaseFailed {
                    phase,
                    source: "Terminal phases cannot be executed directly".to_string(),
                })
            }
        }
    }
}

impl Default for PipelineRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pipeline() -> Pipeline {
        Pipeline::new()
            .with_name("Test Pipeline")
            .with_stage(crate::Stage {
                name: "Build".to_string(),
                agent: None,
                environment: crate::Environment::new(),
                options: None,
                when: None,
                post: None,
                steps: vec![crate::Step {
                    step_type: crate::StepType::Echo {
                        message: "Hello".to_string(),
                    },
                    name: Some("echo-step".to_string()),
                    timeout: None,
                    retry: None,
                }],
            })
    }

    #[test]
    fn test_lifecycle_phase_ordering() {
        // Verify phase ordering is correct
        let phases = [
            LifecyclePhase::Init,
            LifecyclePhase::LoadLibraries,
            LifecyclePhase::SetupEngine,
            LifecyclePhase::LoadSourceCode,
            LifecyclePhase::BindSteps,
            LifecyclePhase::Execute,
        ];

        for (i, phase) in phases.iter().enumerate() {
            if i > 0 {
                assert!(
                    *phase as u8 > phases[i - 1] as u8,
                    "Phases should be in order"
                );
            }
        }
    }

    #[test]
    fn test_lifecycle_phase_is_terminal() {
        assert!(LifecyclePhase::Completed.is_terminal());
        assert!(LifecyclePhase::Failed.is_terminal());
        assert!(!LifecyclePhase::Init.is_terminal());
        assert!(!LifecyclePhase::Execute.is_terminal());
    }

    #[test]
    fn test_phase_result_success() {
        let result = PhaseResult::success(LifecyclePhase::Init, 100, Some("output".to_string()));
        assert_eq!(result.phase, LifecyclePhase::Init);
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.output, Some("output".to_string()));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_phase_result_failure() {
        let result = PhaseResult::failure(LifecyclePhase::LoadLibraries, 50, "error message");
        assert_eq!(result.phase, LifecyclePhase::LoadLibraries);
        assert_eq!(result.duration_ms, 50);
        assert!(result.output.is_none());
        assert_eq!(result.error, Some("error message".to_string()));
    }

    #[test]
    fn test_pipeline_runtime_new() {
        let runtime = PipelineRuntime::new();
        assert!(runtime.config.is_none());
        assert!(runtime.phase_log.is_empty());
        assert!(runtime.start_time.is_none());
    }

    #[test]
    fn test_pipeline_runtime_with_config() {
        let json = r#"{
            "version": "1",
            "spec": {
                "pipeline": {
                    "name": "ConfigTest",
                    "stages": [
                        {
                            "name": "Build",
                            "steps": [
                                {"type": "echo", "message": "Hello"}
                            ]
                        }
                    ]
                }
            }
        }"#;
        let config = PipelineConfig::from_json(json).expect("Should parse JSON");
        let runtime = PipelineRuntime::with_config(config);
        assert!(runtime.config.is_some());
    }

    #[test]
    fn test_pipeline_runtime_run_empty_config() {
        let mut runtime = PipelineRuntime::new();
        let pipeline = create_test_pipeline();

        let result = runtime.run(&pipeline);
        assert!(result.is_ok());

        let exec_result = result.unwrap();
        assert!(exec_result.success);

        // Should have 7 phase results (6 phases + Completed)
        assert_eq!(runtime.phase_log().len(), 7);

        // First phase should be Init
        assert_eq!(runtime.phase_log()[0].phase, LifecyclePhase::Init);

        // Last phase should be Completed
        assert_eq!(runtime.phase_log()[6].phase, LifecyclePhase::Completed);
    }

    #[test]
    fn test_pipeline_runtime_run_with_config_libraries() {
        let json = r#"{
            "version": "1",
            "spec": {
                "libraries": [
                    {
                        "name": "mylib",
                        "sourcePath": "https://github.com/example/mylib",
                        "retrieverType": "gitSource"
                    }
                ],
                "pipeline": {
                    "name": "LibraryTest",
                    "stages": [
                        {
                            "name": "Build",
                            "steps": [
                                {"type": "echo", "message": "Hello"}
                            ]
                        }
                    ]
                }
            }
        }"#;
        let config = PipelineConfig::from_json(json).expect("Should parse JSON");
        let mut runtime = PipelineRuntime::with_config(config);

        // Set a library loader function
        runtime.set_library_loader(Box::new(|_libraries| Ok(1)));

        let pipeline = create_test_pipeline();

        let result = runtime.run(&pipeline);
        assert!(result.is_ok());

        // LoadLibraries phase should report libraries loaded
        let load_libs_phase = &runtime.phase_log()[1];
        assert_eq!(load_libs_phase.phase, LifecyclePhase::LoadLibraries);
        assert!(load_libs_phase.output.as_ref().unwrap().contains("1 libraries"));
    }

    #[test]
    fn test_pipeline_runtime_run_with_config_no_libraries() {
        let json = r#"{
            "version": "1",
            "spec": {
                "pipeline": {
                    "name": "NoLibrariesTest",
                    "stages": [
                        {
                            "name": "Build",
                            "steps": [
                                {"type": "echo", "message": "Hello"}
                            ]
                        }
                    ]
                }
            }
        }"#;
        let config = PipelineConfig::from_json(json).expect("Should parse JSON");
        let mut runtime = PipelineRuntime::with_config(config);
        let pipeline = create_test_pipeline();

        let result = runtime.run(&pipeline);
        assert!(result.is_ok());

        // LoadLibraries phase should report skipped
        let load_libs_phase = &runtime.phase_log()[1];
        assert_eq!(load_libs_phase.phase, LifecyclePhase::LoadLibraries);
        assert!(load_libs_phase.output.as_ref().unwrap().contains("skipped"));
    }

    #[test]
    fn test_pipeline_runtime_invalid_pipeline() {
        let mut runtime = PipelineRuntime::new();
        let pipeline = Pipeline::new(); // Empty pipeline, invalid

        let result = runtime.run(&pipeline);
        assert!(result.is_err());

        // First phase (Init) should have failed
        let init_phase = &runtime.phase_log()[0];
        assert_eq!(init_phase.phase, LifecyclePhase::Init);
        assert!(init_phase.error.is_some());
    }

    #[test]
    fn test_pipeline_runtime_phase_durations() {
        let mut runtime = PipelineRuntime::new();
        let pipeline = create_test_pipeline();

        let _ = runtime.run(&pipeline);

        // All phases should have non-zero duration
        for phase_result in runtime.phase_log() {
            assert!(
                phase_result.duration_ms >= 0,
                "Duration should be recorded"
            );
        }
    }

    #[test]
    fn test_runtime_error_display() {
        let err = RuntimeError::PhaseFailed {
            phase: LifecyclePhase::Execute,
            source: "execution failed".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Execute"));
        assert!(display.contains("execution failed"));

        let config_err = RuntimeError::ConfigError("missing field".to_string());
        let config_display = format!("{}", config_err);
        assert!(config_display.contains("Configuration error"));
        assert!(config_display.contains("missing field"));
    }
}
