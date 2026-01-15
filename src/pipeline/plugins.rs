//! Plugin system for custom steps
//!
//! This module provides a way to define and register custom reusable steps.

use crate::executor::PipelineContext;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Trait for custom step plugins
pub trait CustomStep: Send + Sync {
    /// Execute the custom step
    fn execute(&self, context: &PipelineContext) -> Result<(), crate::pipeline::PipelineError>;

    /// Get the name of this step
    fn name(&self) -> &str;

    /// Get the description of this step
    fn description(&self) -> &str;
}

/// Registry for custom steps
#[derive(Default)]
pub struct CustomStepRegistry {
    steps: HashMap<String, Arc<dyn CustomStep>>,
}

impl CustomStepRegistry {
    /// Creates a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            steps: HashMap::new(),
        }
    }

    /// Registers a custom step
    pub fn register<T: CustomStep + 'static>(&mut self, step: T) {
        let arc: Arc<dyn CustomStep> = Arc::new(step);
        self.steps.insert(arc.name().to_string(), arc);
    }

    /// Gets a custom step by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn CustomStep>> {
        self.steps.get(name).cloned()
    }

    /// Checks if a custom step exists
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.steps.contains_key(name)
    }

    /// Gets all registered step names
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.steps.keys().map(|s| s.as_str()).collect()
    }
}

/// A custom step that runs a shell command
#[derive(Debug)]
pub struct ShellCustomStep {
    name: String,
    description: String,
    command: String,
}

impl ShellCustomStep {
    /// Creates a new shell custom step
    #[must_use]
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        let cmd = command.into();
        Self {
            name: name.into(),
            description: format!("Runs shell command: {}", cmd),
            command: cmd,
        }
    }

    /// Sets the description
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

impl CustomStep for ShellCustomStep {
    fn execute(&self, context: &PipelineContext) -> Result<(), crate::pipeline::PipelineError> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .current_dir(&context.cwd)
            .envs(&context.env)
            .output()
            .map_err(|e| crate::pipeline::PipelineError::Io(e.to_string()))?;

        if !output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }

        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }

        if !output.status.success() {
            return Err(crate::pipeline::PipelineError::CommandFailed {
                code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// A custom step that prints a message
#[derive(Debug)]
pub struct EchoCustomStep {
    name: String,
    message: String,
}

impl EchoCustomStep {
    /// Creates a new echo custom step
    #[must_use]
    pub fn new(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            message: message.into(),
        }
    }
}

impl CustomStep for EchoCustomStep {
    fn execute(&self, _context: &PipelineContext) -> Result<(), crate::pipeline::PipelineError> {
        println!("[{}] {}", self.name, self.message);
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.name
    }
}

/// Thread-safe registry holder
#[derive(Clone, Default)]
pub struct SharedRegistry {
    inner: Arc<Mutex<CustomStepRegistry>>,
}

impl SharedRegistry {
    /// Creates a new shared registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CustomStepRegistry::new())),
        }
    }

    /// Registers a custom step
    pub fn register<T: CustomStep + 'static>(&self, step: T) {
        let mut guard = self.inner.lock().unwrap();
        guard.register(step);
    }

    /// Gets a custom step by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn CustomStep>> {
        let guard = self.inner.lock().unwrap();
        guard.get(name)
    }

    /// Checks if a custom step exists
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        let guard = self.inner.lock().unwrap();
        guard.contains(name)
    }
}

/// Creates a custom step from a closure
#[derive(Clone)]
pub struct ClosureCustomStep<S> {
    name: String,
    description: String,
    closure: S,
}

impl<S> ClosureCustomStep<S>
where
    S: Fn(&PipelineContext) -> Result<(), crate::pipeline::PipelineError> + Send + Sync + 'static,
{
    /// Creates a new closure custom step
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>, closure: S) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            closure,
        }
    }
}

impl<S> CustomStep for ClosureCustomStep<S>
where
    S: Fn(&PipelineContext) -> Result<(), crate::pipeline::PipelineError> + Send + Sync + 'static,
{
    fn execute(&self, context: &PipelineContext) -> Result<(), crate::pipeline::PipelineError> {
        (self.closure)(context)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Result step that always succeeds
#[derive(Debug)]
pub struct SuccessCustomStep {
    name: String,
}

impl SuccessCustomStep {
    /// Creates a new success step
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl CustomStep for SuccessCustomStep {
    fn execute(&self, _context: &PipelineContext) -> Result<(), crate::pipeline::PipelineError> {
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "A step that always succeeds"
    }
}

/// Result step that always fails
#[derive(Debug)]
pub struct FailCustomStep {
    name: String,
    message: String,
}

impl FailCustomStep {
    /// Creates a new fail step
    #[must_use]
    pub fn new(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            message: message.into(),
        }
    }
}

impl CustomStep for FailCustomStep {
    fn execute(&self, _context: &PipelineContext) -> Result<(), crate::pipeline::PipelineError> {
        Err(crate::pipeline::PipelineError::CommandFailed {
            code: -1,
            stderr: self.message.clone(),
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "A step that always fails"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::PipelineContext;

    #[test]
    fn test_registry_creation() {
        let registry = CustomStepRegistry::new();
        assert!(registry.names().is_empty());
    }

    #[test]
    fn test_registry_contains() {
        let mut registry = CustomStepRegistry::new();
        let step = ShellCustomStep::new("test", "echo hello");
        registry.register(step);

        assert!(registry.contains("test"));
        assert!(!registry.contains("nonexistent"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = CustomStepRegistry::new();
        let step = ShellCustomStep::new("test", "echo hello");
        registry.register(step);

        let retrieved = registry.get("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test");
    }

    #[test]
    fn test_shell_custom_step_execution() {
        let step = ShellCustomStep::new("test", "echo 'hello world'");
        let context = PipelineContext::new();

        let result = step.execute(&context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shell_custom_step_failure() {
        let step = ShellCustomStep::new("test", "exit 1");
        let context = PipelineContext::new();

        let result = step.execute(&context);
        assert!(result.is_err());
    }

    #[test]
    fn test_echo_custom_step() {
        let step = EchoCustomStep::new("notify", "Build completed");
        let context = PipelineContext::new();

        let result = step.execute(&context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_success_custom_step() {
        let step = SuccessCustomStep::new("always-pass");
        let context = PipelineContext::new();

        let result = step.execute(&context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fail_custom_step() {
        let step = FailCustomStep::new("always-fail", "This always fails");
        let context = PipelineContext::new();

        let result = step.execute(&context);
        assert!(result.is_err());
    }

    #[test]
    fn test_closure_custom_step() {
        let step = ClosureCustomStep::new("test", "A test step", move |_| Ok(()));
        let context = PipelineContext::new();

        let result = step.execute(&context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shared_registry() {
        let registry = SharedRegistry::new();
        let step = ShellCustomStep::new("shared", "echo shared");
        registry.register(step);

        assert!(registry.contains("shared"));

        let retrieved = registry.get("shared");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_registry_names() {
        let mut registry = CustomStepRegistry::new();
        registry.register(ShellCustomStep::new("step1", "echo 1"));
        registry.register(ShellCustomStep::new("step2", "echo 2"));
        registry.register(EchoCustomStep::new("step3", "message"));

        let names = registry.names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"step1"));
        assert!(names.contains(&"step2"));
        assert!(names.contains(&"step3"));
    }
}
