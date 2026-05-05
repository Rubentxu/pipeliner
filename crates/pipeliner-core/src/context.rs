//! Pipeline context for dependency injection and environment management.
//!
//! This module provides a context that holds components, environment variables,
//! and a step registry for pipeline execution.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::registry::{StepFactory, StepRegistry};
use crate::Environment;

/// Pipeline context for dependency injection and step resolution.
///
/// The context provides:
/// - Component storage for dependency injection
/// - Environment variable management
/// - Step registry for custom step resolution
/// - Credential storage for WithCredentials step
#[derive(Debug)]
pub struct PipelineContext {
    /// Environment variables
    env: HashMap<String, String>,
    /// Registered components for dependency injection
    components: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    /// Step registry for custom step resolution
    step_registry: StepRegistry,
    /// Credentials storage: credential_id -> field_name -> value
    credentials: HashMap<String, HashMap<String, String>>,
}

impl PipelineContext {
    /// Creates a new empty pipeline context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            env: HashMap::new(),
            components: HashMap::new(),
            step_registry: StepRegistry::new(),
            credentials: HashMap::new(),
        }
    }

    /// Creates a new pipeline context with a step registry.
    #[must_use]
    pub fn with_registry(registry: StepRegistry) -> Self {
        Self {
            env: HashMap::new(),
            components: HashMap::new(),
            step_registry: registry,
            credentials: HashMap::new(),
        }
    }

    /// Registers a component for dependency injection.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The component type, must be `'static + Send + Sync`
    ///
    /// # Arguments
    ///
    /// * `component` - The component to register
    pub fn register_component<T: 'static + Send + Sync>(&mut self, component: T) {
        self.components.insert(TypeId::of::<T>(), Box::new(component));
    }

    /// Retrieves a registered component by type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The component type to retrieve, must be `'static + Send + Sync`
    ///
    /// # Returns
    ///
    /// The component if found, `None` otherwise.
    #[must_use]
    pub fn get_component<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Gets a step factory by name from the registry.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the step factory to retrieve
    ///
    /// # Returns
    ///
    /// The factory if found, `None` otherwise.
    #[must_use]
    pub fn get_step(&self, name: &str) -> Option<Arc<dyn StepFactory>> {
        self.step_registry.get(name)
    }

    /// Registers a step factory.
    ///
    /// # Arguments
    ///
    /// * `factory` - The step factory to register
    pub fn register_step(&mut self, factory: Arc<dyn StepFactory>) {
        self.step_registry.register(factory);
    }

    /// Gets an environment variable.
    ///
    /// # Arguments
    ///
    /// * `key` - The environment variable key
    ///
    /// # Returns
    ///
    /// The value if found, `None` otherwise.
    #[must_use]
    pub fn get_env(&self, key: &str) -> Option<&str> {
        self.env.get(key).map(String::as_str)
    }

    /// Sets an environment variable.
    ///
    /// # Arguments
    ///
    /// * `key` - The environment variable key
    /// * `value` - The environment variable value
    pub fn set_env(&mut self, key: String, value: String) {
        self.env.insert(key, value);
    }

    /// Registers a credential for use by WithCredentials steps.
    ///
    /// # Arguments
    ///
    /// * `id` - The credential ID
    /// * `fields` - The credential fields (key-value pairs)
    pub fn register_credential(&mut self, id: String, fields: HashMap<String, String>) {
        self.credentials.insert(id, fields);
    }

    /// Gets a credential by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The credential ID
    ///
    /// # Returns
    ///
    /// The credential fields if found, `None` otherwise.
    #[must_use]
    pub fn get_credential(&self, id: &str) -> Option<&HashMap<String, String>> {
        self.credentials.get(id)
    }

    /// Returns the number of registered credentials.
    #[must_use]
    pub fn credential_count(&self) -> usize {
        self.credentials.len()
    }

    /// Returns the number of registered components.
    #[must_use]
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns the number of environment variables.
    #[must_use]
    pub fn env_count(&self) -> usize {
        self.env.len()
    }

    /// Returns true if the context has no components.
    #[must_use]
    pub fn has_components(&self) -> bool {
        !self.components.is_empty()
    }

    /// Converts the environment to a pipeliner Environment.
    ///
    /// # Returns
    ///
    /// An `Environment` instance with the context's environment variables.
    #[must_use]
    pub fn to_environment(&self) -> Environment {
        let mut env = Environment::new();
        for (key, value) in &self.env {
            env.insert(key, value);
        }
        env
    }
}

impl Default for PipelineContext {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: PipelineContext uses internal synchronization via HashMap with
// Box<dyn Any + Send + Sync> and StepRegistry which is Send + Sync
unsafe impl Send for PipelineContext {}
unsafe impl Sync for PipelineContext {}

#[cfg(test)]
mod tests {
    use super::*;

    // A simple component for testing
    #[derive(Debug, PartialEq)]
    struct TestComponent {
        value: String,
    }

    // Another component type for testing
    #[derive(Debug, PartialEq)]
    struct AnotherComponent {
        count: i32,
    }

    // =======================================================================
    // C1: PipelineContext CDI Tests
    // =======================================================================

    #[test]
    fn test_pipeline_context_new_is_empty() {
        // C1.1: PipelineContext::new() creates empty components map
        let ctx = PipelineContext::new();
        assert!(!ctx.has_components());
        assert_eq!(ctx.component_count(), 0);
        assert_eq!(ctx.env_count(), 0);
    }

    #[test]
    fn test_pipeline_context_register_and_get_component() {
        // C1.2: register_component::<T>(instance) then get_component::<T>() returns Some (SCN-SR-004)
        let mut ctx = PipelineContext::new();
        let component = TestComponent {
            value: "test".to_string(),
        };
        ctx.register_component(component);

        let retrieved = ctx.get_component::<TestComponent>();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "test");
    }

    #[test]
    fn test_pipeline_context_get_unregistered_component_returns_none() {
        // C1.3: get_component::<T>() when never registered returns None (SCN-SR-005)
        let ctx = PipelineContext::new();
        let result = ctx.get_component::<TestComponent>();
        assert!(result.is_none());
    }

    #[test]
    fn test_pipeline_context_different_types_independent() {
        // C1.3: Register different types work independently
        let mut ctx = PipelineContext::new();
        let comp1 = TestComponent {
            value: "test".to_string(),
        };
        let comp2 = AnotherComponent { count: 42 };

        ctx.register_component(comp1);
        ctx.register_component(comp2);

        let retrieved1 = ctx.get_component::<TestComponent>();
        let retrieved2 = ctx.get_component::<AnotherComponent>();

        assert!(retrieved1.is_some());
        assert!(retrieved2.is_some());
        assert_eq!(retrieved1.unwrap().value, "test");
        assert_eq!(retrieved2.unwrap().count, 42);
    }

    #[test]
    fn test_pipeline_context_send_sync() {
        // C1.4: Send + Sync compile-time assertions (SCN-SR-008)
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PipelineContext>();
    }

    // =======================================================================
    // C3: PipelineContext step_registry integration Tests
    // =======================================================================

    #[test]
    fn test_pipeline_context_get_step_delegates_to_registry() {
        // C3.1: PipelineContext with StepRegistry integration — get_step(name) delegates correctly
        use crate::registry::{CustomStep, StepFactory};
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct TestFactory {
            name: String,
            call_count: AtomicUsize,
        }

        impl TestFactory {
            fn new(name: &str) -> Self {
                Self {
                    name: name.to_string(),
                    call_count: AtomicUsize::new(0),
                }
            }
        }

        impl StepFactory for TestFactory {
            fn name(&self) -> &str {
                &self.name
            }

            fn create(&self, args: &[serde_json::Value]) -> Result<CustomStep, crate::registry::StepError> {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Ok(CustomStep::success("test", Some("output".to_string())))
            }
        }

        let mut ctx = PipelineContext::new();
        let factory = Arc::new(TestFactory::new("my-step"));
        ctx.register_step(factory.clone());

        let retrieved = ctx.get_step("my-step");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "my-step");
    }

    #[test]
    fn test_pipeline_context_get_unregistered_step_returns_none() {
        let ctx = PipelineContext::new();
        let result = ctx.get_step("nonexistent");
        assert!(result.is_none());
    }

    // =======================================================================
    // Environment Tests
    // =======================================================================

    #[test]
    fn test_pipeline_context_env_operations() {
        // C3.2: env var access
        let mut ctx = PipelineContext::new();

        // Initially empty
        assert!(ctx.get_env("FOO").is_none());

        // Set and get
        ctx.set_env("FOO".to_string(), "bar".to_string());
        assert_eq!(ctx.get_env("FOO"), Some("bar"));

        // Update
        ctx.set_env("FOO".to_string(), "baz".to_string());
        assert_eq!(ctx.get_env("FOO"), Some("baz"));

        // Multiple vars
        ctx.set_env("BAR".to_string(), "qux".to_string());
        assert_eq!(ctx.env_count(), 2);
    }

    #[test]
    fn test_pipeline_context_to_environment() {
        let mut ctx = PipelineContext::new();
        ctx.set_env("KEY1".to_string(), "value1".to_string());
        ctx.set_env("KEY2".to_string(), "value2".to_string());

        let env = ctx.to_environment();
        assert_eq!(env.get("KEY1"), Some(&crate::environment::EnvVarValue::Value("value1".to_string())));
        assert_eq!(env.get("KEY2"), Some(&crate::environment::EnvVarValue::Value("value2".to_string())));
    }

    // =======================================================================
    // C2: Constructor Tests
    // =======================================================================

    #[test]
    fn test_pipeline_context_with_registry() {
        let registry = StepRegistry::new();
        let ctx = PipelineContext::with_registry(registry);
        assert!(ctx.step_registry.is_empty());
    }
}
