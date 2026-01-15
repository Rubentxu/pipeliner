//! Pipeline domain types and logic

// Make submodules public
pub mod agent;
pub mod errors;
pub mod options;
pub mod pipeline_def;
pub mod plugins;
pub mod post;
pub mod shared_library;
pub mod stage;
pub mod steps;
pub mod types;

// Add serde use for derive macros in this module
pub use serde::{Deserialize, Serialize};

// Re-export public types from submodules
pub use agent::{AgentConfig, AgentType, DockerConfig, KubernetesConfig, PodmanConfig};
pub use errors::{PipelineError, ValidationError};
pub use options::{BuildDiscarder, PipelineOptions, Trigger, ValidateDuration};
pub use pipeline_def::{Pipeline, PipelineBuilder};
pub use plugins::{CustomStep, CustomStepRegistry, SharedRegistry};
pub use post::PostCondition;
pub use shared_library::{LibraryStep, SharedLibrary, SharedLibraryError};
pub use stage::{Stage, StageBuilder, WhenCondition};
pub use steps::{Step, StepType};
pub use types::{PipelineResult, StageResult, Validate};

/// Defines environment variables that can be used in pipeline steps.
///
/// Variables can be resolved using the [`resolve`][Environment::resolve] method
/// which supports `${VAR}` and `$VAR` syntax.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Environment {
    /// Environment variables as key-value pairs.
    #[serde(flatten)]
    pub vars: std::collections::HashMap<String, String>,
}

impl Environment {
    /// Creates a new empty environment.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an environment variable.
    #[must_use]
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    /// Gets an environment variable by name.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.vars.get(key)
    }

    /// Resolves a value that may contain variable expansions like `${VAR}`.
    #[must_use]
    pub fn resolve(&self, value: &str) -> String {
        let mut result = value.to_string();
        let mut start = 0;

        while let Some(dollar_pos) = result[start..].find('$') {
            let var_start = start + dollar_pos + 1;
            if var_start >= result.len() {
                break;
            }

            if result.chars().nth(var_start) == Some('{') {
                if let Some(end_brace) = result[var_start..].find('}') {
                    let var_end = var_start + end_brace + 1;
                    let var_name = &result[var_start + 1..var_end - 1];

                    if let Some(var_value) = self.vars.get(var_name) {
                        result.replace_range(start..var_end, var_value);
                        start += var_value.len();
                    } else {
                        start = var_end;
                    }
                } else {
                    break;
                }
            } else {
                start += dollar_pos + 1;
            }
        }

        result
    }
}

/// Defines build parameters that can be provided at pipeline execution time.
///
/// Parameters support different types:
/// - Boolean parameters
/// - String parameters
/// - Choice parameters (enumeration)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Parameters {
    /// Boolean parameters.
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty", default)]
    pub boolean: std::collections::HashMap<String, bool>,

    /// String parameters.
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty", default)]
    pub string: std::collections::HashMap<String, String>,

    /// Choice parameters with predefined options.
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty", default)]
    pub choice: std::collections::HashMap<String, Vec<String>>,
}

impl Parameters {
    /// Creates a new empty parameters set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a boolean parameter.
    #[must_use]
    pub fn boolean(mut self, name: impl Into<String>, value: bool) -> Self {
        self.boolean.insert(name.into(), value);
        self
    }

    /// Adds a string parameter.
    #[must_use]
    pub fn string(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.string.insert(name.into(), value.into());
        self
    }

    /// Adds a string parameter with a default value.
    #[must_use]
    pub fn string_with_default(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        _default: impl Into<String>,
    ) -> Self {
        self.string.insert(name.into(), value.into());
        self
    }

    /// Adds a choice parameter (enumeration).
    #[must_use]
    pub fn choice(mut self, name: impl Into<String>, choices: Vec<String>) -> Self {
        self.choice.insert(name.into(), choices);
        self
    }

    /// Validates parameter names (no spaces, valid characters).
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::InvalidNameChars`] if any parameter name
    /// contains invalid characters.
    #[allow(clippy::missing_errors_doc)]
    pub fn validate(&self) -> Result<(), ValidationError> {
        let valid_name = |name: &str| -> bool {
            !name.is_empty()
                && !name.contains(' ')
                && name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        };

        for name in self.boolean.keys() {
            if !valid_name(name) {
                return Err(ValidationError::InvalidNameChars { name: name.clone() });
            }
        }

        for name in self.string.keys() {
            if !valid_name(name) {
                return Err(ValidationError::InvalidNameChars { name: name.clone() });
            }
        }

        for name in self.choice.keys() {
            if !valid_name(name) {
                return Err(ValidationError::InvalidNameChars { name: name.clone() });
            }
        }

        Ok(())
    }
}

/// Configuration for matrix execution
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MatrixConfig {
    /// Axes of the matrix
    pub axes: Vec<MatrixAxis>,
    /// Exclusions from the matrix
    pub excludes: Vec<MatrixExclude>,
}

/// A single axis of the matrix
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixAxis {
    /// Name of the axis
    pub name: String,
    /// Values for this axis
    pub values: Vec<String>,
}

/// Exclusion rule for matrix
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixExclude {
    /// Key-value pairs that should be excluded
    pub conditions: Vec<(String, String)>,
}

impl MatrixConfig {
    /// Creates a new empty matrix configuration
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an axis to the matrix
    #[must_use]
    pub fn add_axis(mut self, name: impl Into<String>, values: Vec<String>) -> Self {
        self.axes.push(MatrixAxis {
            name: name.into(),
            values,
        });
        self
    }

    /// Adds an exclusion rule
    #[must_use]
    pub fn add_exclude(mut self, conditions: Vec<(String, String)>) -> Self {
        self.excludes.push(MatrixExclude { conditions });
        self
    }

    /// Generates all combinations from the matrix axes
    #[must_use]
    pub fn generate_combinations(&self) -> Vec<Vec<(String, String)>> {
        if self.axes.is_empty() {
            return vec![];
        }

        let mut combinations = vec![vec![]];

        for axis in &self.axes {
            let mut new_combinations = vec![];
            for combo in &combinations {
                for value in &axis.values {
                    let mut new_combo = combo.clone();
                    new_combo.push((axis.name.clone(), value.clone()));
                    new_combinations.push(new_combo);
                }
            }
            combinations = new_combinations;
        }

        // Apply exclusions
        combinations
            .into_iter()
            .filter(|combo| {
                for exclude in &self.excludes {
                    let mut all_match = true;
                    for (key, value) in &exclude.conditions {
                        if !combo.iter().any(|(k, v)| k == key && v == value) {
                            all_match = false;
                            break;
                        }
                    }
                    if all_match {
                        return false; // Excluded
                    }
                }
                true
            })
            .collect()
    }
}

impl Validate for MatrixConfig {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        for axis in &self.axes {
            if axis.name.is_empty() {
                return Err(ValidationError::InvalidNameChars {
                    name: String::new(),
                });
            }
            if axis.values.is_empty() {
                return Err(ValidationError::InvalidAgentType(
                    "Matrix axis must have at least one value".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// A parallel branch in pipeline execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParallelBranch {
    /// Name of the branch
    pub name: String,
    /// Stage to execute
    pub stage: Stage,
}

impl Validate for ParallelBranch {
    type Error = ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.name.is_empty() {
            return Err(ValidationError::EmptyName);
        }
        self.stage.validate()
    }
}
