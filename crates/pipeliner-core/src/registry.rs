//! Step registry for managing step factories.
//!
//! This module provides a registry for registering and retrieving step factories
//! by name. The registry enables dynamic step creation at runtime.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Error types for step creation and registry operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepError {
    /// Step not found in registry
    #[serde(rename = "notFound")]
    NotFound { name: String },
    /// Invalid arguments provided to step factory
    #[serde(rename = "invalidArgs")]
    InvalidArgs { message: String },
    /// Step creation failed
    #[serde(rename = "creationFailed")]
    CreationFailed { message: String },
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepError::NotFound { name } => write!(f, "Step '{}' not found in registry", name),
            StepError::InvalidArgs { message } => write!(f, "Invalid arguments: {}", message),
            StepError::CreationFailed { message } => write!(f, "Step creation failed: {}", message),
        }
    }
}

impl std::error::Error for StepError {}

/// A factory trait for creating step instances.
///
/// Implement this trait to register custom steps in the registry.
/// The factory is responsible for creating a step with the given configuration.
pub trait StepFactory: Send + Sync {
    /// Returns the name of the step this factory creates.
    fn name(&self) -> &str;

    /// Creates a new step instance with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `args` - Configuration arguments for the step
    ///
    /// # Errors
    ///
    /// Returns `StepError` if the step cannot be created (invalid args, missing resources, etc.)
    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError>;
}

/// A simple step wrapper for custom step execution.
///
/// This represents a dynamically created step from a factory.
#[derive(Debug, Clone)]
pub struct CustomStep {
    /// Step name
    pub name: String,
    /// Whether the step executed successfully
    pub success: bool,
    /// Optional output from the step
    pub output: Option<String>,
}

impl CustomStep {
    /// Creates a new successful step.
    #[must_use]
    pub fn success(name: impl Into<String>, output: Option<String>) -> Self {
        Self {
            name: name.into(),
            success: true,
            output,
        }
    }

    /// Creates a new failed step.
    #[must_use]
    pub fn failure(name: impl Into<String>, output: Option<String>) -> Self {
        Self {
            name: name.into(),
            success: false,
            output,
        }
    }
}

/// Registry for step factories.
///
/// The registry maintains a mapping of step names to their factory instances.
/// It allows registering new factories and retrieving existing ones by name.
pub struct StepRegistry {
    steps: HashMap<String, Arc<dyn StepFactory>>,
}

impl fmt::Debug for StepRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StepRegistry")
            .field("steps", &self.steps.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for StepRegistry {
    fn default() -> Self {
        Self {
            steps: HashMap::new(),
        }
    }
}

impl StepRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            steps: HashMap::new(),
        }
    }

    /// Registers a step factory.
    ///
    /// If a factory with the same name already exists, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `factory` - The step factory to register
    pub fn register(&mut self, factory: Arc<dyn StepFactory>) {
        self.steps.insert(factory.name().to_string(), factory);
    }

    /// Gets a step factory by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the step factory to retrieve
    ///
    /// # Returns
    ///
    /// The factory if found, `None` otherwise.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn StepFactory>> {
        self.steps.get(name).cloned()
    }

    /// Lists all registered step names.
    ///
    /// # Returns
    ///
    /// A vector of step names currently registered.
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        self.steps.keys().cloned().collect()
    }

    /// Returns the number of registered steps.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Returns true if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

// Safety: StepRegistry uses internal synchronization via Arc<dyn StepFactory>
// and HashMap which is safe for concurrent read-only access
unsafe impl Send for StepRegistry {}
unsafe impl Sync for StepRegistry {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A test factory that creates echo steps.
    #[derive(Debug)]
    struct EchoFactory {
        name: String,
        call_count: AtomicUsize,
    }

    impl EchoFactory {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                call_count: AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    impl StepFactory for EchoFactory {
        fn name(&self) -> &str {
            &self.name
        }

        fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let message = args
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();
            Ok(CustomStep::success("echo", Some(message)))
        }
    }

    /// A test factory that fails on creation.
    #[derive(Debug)]
    struct FailingFactory;

    impl StepFactory for FailingFactory {
        fn name(&self) -> &str {
            "failing"
        }

        fn create(&self, _args: &[JsonValue]) -> Result<CustomStep, StepError> {
            Err(StepError::CreationFailed {
                message: "Factory always fails".to_string(),
            })
        }
    }

    /// A test factory that validates arguments.
    #[derive(Debug)]
    struct ValidatingFactory;

    impl StepFactory for ValidatingFactory {
        fn name(&self) -> &str {
            "validating"
        }

        fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
            if args.is_empty() {
                return Err(StepError::InvalidArgs {
                    message: "Expected at least one argument".to_string(),
                });
            }
            Ok(CustomStep::success("validating", Some("ok".to_string())))
        }
    }

    // =======================================================================
    // B1: StepRegistry Tests
    // =======================================================================

    #[test]
    fn test_step_registry_new_is_empty() {
        // B1.1: StepRegistry::new() creates empty registry, get() returns None
        let registry = StepRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_step_registry_register_and_get() {
        // B1.2: register() then get() returns Some(factory) with correct name (SCN-SR-001)
        let mut registry = StepRegistry::new();
        let factory = Arc::new(EchoFactory::new("echo"));
        registry.register(factory.clone());

        let retrieved = registry.get("echo");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "echo");
    }

    #[test]
    fn test_step_registry_register_overwrites() {
        // B1.3: register() duplicate name overwrites previous (SCN-SR-002)
        let mut registry = StepRegistry::new();

        let factory1 = Arc::new(EchoFactory::new("echo"));
        registry.register(factory1.clone());

        let factory2 = Arc::new(EchoFactory::new("echo"));
        registry.register(factory2.clone());

        // Should still only have one entry
        assert_eq!(registry.len(), 1);
        // get() should return a factory
        assert!(registry.get("echo").is_some());
    }

    #[test]
    fn test_step_registry_list_returns_all_names() {
        // B1.4: list() returns all registered step names
        let mut registry = StepRegistry::new();

        let factory1 = Arc::new(EchoFactory::new("step1"));
        let factory2 = Arc::new(EchoFactory::new("step2"));
        let factory3 = Arc::new(EchoFactory::new("step3"));

        registry.register(factory1);
        registry.register(factory2);
        registry.register(factory3);

        let names = registry.list();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"step1".to_string()));
        assert!(names.contains(&"step2".to_string()));
        assert!(names.contains(&"step3".to_string()));
    }

    #[test]
    fn test_step_registry_get_unregistered_returns_none() {
        // B1.2: get() for unregistered name returns None
        let mut registry = StepRegistry::new();
        let factory = Arc::new(EchoFactory::new("registered"));
        registry.register(factory);

        assert!(registry.get("unregistered").is_none());
        assert!(registry.get("").is_none());
    }

    // =======================================================================
    // B3: StepFactory Trait Tests
    // =======================================================================

    #[test]
    fn test_step_factory_name_returns_correct_string() {
        // B3.1: StepFactory trait - name() returns correct string
        let factory = EchoFactory::new("my-step");
        assert_eq!(factory.name(), "my-step");
    }

    #[test]
    fn test_step_factory_create_returns_valid_step() {
        // B3.2: StepFactory create() returns Ok(Step) for valid args
        let factory = EchoFactory::new("echo");
        let args = [JsonValue::String("hello".to_string())];
        let step = factory.create(&args);

        assert!(step.is_ok());
        let step = step.unwrap();
        assert!(step.success);
        assert_eq!(step.output, Some("hello".to_string()));
    }

    #[test]
    fn test_step_factory_create_with_invalid_args_returns_error() {
        // B3.2: StepFactory create() returns Err for invalid args
        let factory = ValidatingFactory;
        let args: [JsonValue; 0] = [];
        let result = factory.create(&args);

        assert!(result.is_err());
        if let Err(StepError::InvalidArgs { message }) = result {
            assert!(message.contains("Expected at least one argument"));
        } else {
            panic!("Expected StepError::InvalidArgs");
        }
    }

    #[test]
    fn test_step_factory_create_failure_returns_error() {
        // B3.2: StepFactory create() returns Err for creation failure
        let factory = FailingFactory;
        let args: [JsonValue; 0] = [];
        let result = factory.create(&args);

        assert!(result.is_err());
        if let Err(StepError::CreationFailed { message }) = result {
            assert_eq!(message, "Factory always fails");
        } else {
            panic!("Expected StepError::CreationFailed");
        }
    }

    #[test]
    fn test_echo_factory_counts_calls() {
        // B3.3: TestFactory impl - a concrete factory for testing (call counting)
        let factory = Arc::new(EchoFactory::new("counted"));
        let mut registry = StepRegistry::new();
        registry.register(factory.clone());

        // Create steps multiple times
        let args = [JsonValue::String("test".to_string())];
        for _ in 0..3 {
            let factory = registry.get("counted").unwrap();
            let _ = factory.create(&args);
        }

        assert_eq!(factory.call_count(), 3);
    }

    // =======================================================================
    // StepError Tests
    // =======================================================================

    #[test]
    fn test_step_error_display_not_found() {
        let error = StepError::NotFound {
            name: "missing".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("missing"));
        assert!(display.contains("not found"));
    }

    #[test]
    fn test_step_error_display_invalid_args() {
        let error = StepError::InvalidArgs {
            message: "too few arguments".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("too few arguments"));
        assert!(display.contains("Invalid arguments"));
    }

    #[test]
    fn test_step_error_display_creation_failed() {
        let error = StepError::CreationFailed {
            message: "out of memory".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("out of memory"));
        assert!(display.contains("creation failed"));
    }

    #[test]
    fn test_step_error_serialize_deserialize() {
        let error = StepError::NotFound {
            name: "test-step".to_string(),
        };
        let json = serde_json::to_string(&error).expect("Should serialize");
        assert!(json.contains("notFound"));

        let parsed: StepError = serde_json::from_str(&json).expect("Should deserialize");
        assert!(matches!(parsed, StepError::NotFound { .. }));
    }

    // =======================================================================
    // CustomStep Tests
    // =======================================================================

    #[test]
    fn test_custom_step_success() {
        let step = CustomStep::success("test", Some("output".to_string()));
        assert!(step.success);
        assert_eq!(step.name, "test");
        assert_eq!(step.output, Some("output".to_string()));
    }

    #[test]
    fn test_custom_step_failure() {
        let step = CustomStep::failure("test", Some("error".to_string()));
        assert!(!step.success);
        assert_eq!(step.name, "test");
        assert_eq!(step.output, Some("error".to_string()));
    }
}
