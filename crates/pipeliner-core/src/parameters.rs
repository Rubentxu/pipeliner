//! Pipeline parameters for runtime configuration.
//!
//! This module provides types for defining pipeline parameters
//! that can be provided at runtime when triggering a pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Collection of pipeline parameters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Parameters(pub Vec<Parameter>);

impl Parameters {
    /// Creates a new empty parameters collection
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Creates parameters from a list
    #[must_use]
    pub fn from_vec(params: Vec<Parameter>) -> Self {
        Self(params)
    }

    /// Adds a parameter
    pub fn push(&mut self, param: Parameter) {
        self.0.push(param);
    }

    /// Gets a parameter by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Parameter> {
        self.0.iter().find(|p| p.name() == name)
    }

    /// Returns an iterator over all parameters
    pub fn iter(&self) -> impl Iterator<Item = &Parameter> {
        self.0.iter()
    }

    /// Returns the number of parameters
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no parameters
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Individual parameter definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Parameter {
    /// String parameter
    String {
        /// Parameter name
        name: String,
        /// Description
        #[serde(default)]
        description: String,
        /// Default value
        #[serde(skip_serializing_if = "Option::is_none")]
        default_value: Option<String>,
        /// Trim spaces from value
        #[serde(default)]
        trim: bool,
    },

    /// Text/textarea parameter
    Text {
        /// Parameter name
        name: String,
        /// Description
        #[serde(default)]
        description: String,
        /// Default value
        #[serde(skip_serializing_if = "Option::is_none")]
        default_value: Option<String>,
        /// Maximum length
        #[serde(skip_serializing_if = "Option::is_none")]
        max_length: Option<usize>,
    },

    /// Boolean parameter
    Boolean {
        /// Parameter name
        name: String,
        /// Description
        #[serde(default)]
        description: String,
        /// Default value
        #[serde(default)]
        default_value: bool,
    },

    /// Choice parameter
    Choice {
        /// Parameter name
        name: String,
        /// Description
        #[serde(default)]
        description: String,
        /// Available choices
        choices: Vec<String>,
        /// Default choice index
        #[serde(skip_serializing_if = "Option::is_none")]
        default_choice: Option<usize>,
    },

    /// Password/secret parameter
    Password {
        /// Parameter name
        name: String,
        /// Description
        #[serde(default)]
        description: String,
    },

    /// File parameter
    File {
        /// Parameter name
        name: String,
        /// Description
        #[serde(default)]
        description: String,
        /// Allowed file patterns
        #[serde(default)]
        file_patterns: Vec<String>,
        /// Maximum file size in bytes
        #[serde(skip_serializing_if = "Option::is_none")]
        max_size: Option<u64>,
    },

    /// Run parameter (select a previous build)
    Run {
        /// Parameter name
        name: String,
        /// Description
        #[serde(default)]
        description: String,
        /// Filter expression
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<String>,
    },
}

/// Parameter type enumeration for type checking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParameterType {
    /// String parameter
    String,
    /// Text parameter
    Text,
    /// Boolean parameter
    Boolean,
    /// Choice parameter
    Choice,
    /// Password parameter
    Password,
    /// File parameter
    File,
    /// Run parameter
    Run,
}

impl Parameter {
    /// Returns the parameter name
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Parameter::String { name, .. }
            | Parameter::Text { name, .. }
            | Parameter::Boolean { name, .. }
            | Parameter::Choice { name, .. }
            | Parameter::Password { name, .. }
            | Parameter::File { name, .. }
            | Parameter::Run { name, .. } => name,
        }
    }

    /// Returns the parameter description
    #[must_use]
    pub fn description(&self) -> &str {
        match self {
            Parameter::String { description, .. }
            | Parameter::Text { description, .. }
            | Parameter::Boolean { description, .. }
            | Parameter::Choice { description, .. }
            | Parameter::Password { description, .. }
            | Parameter::File { description, .. }
            | Parameter::Run { description, .. } => description,
        }
    }

    /// Returns the parameter type
    #[must_use]
    pub fn parameter_type(&self) -> ParameterType {
        match self {
            Parameter::String { .. } => ParameterType::String,
            Parameter::Text { .. } => ParameterType::Text,
            Parameter::Boolean { .. } => ParameterType::Boolean,
            Parameter::Choice { .. } => ParameterType::Choice,
            Parameter::Password { .. } => ParameterType::Password,
            Parameter::File { .. } => ParameterType::File,
            Parameter::Run { .. } => ParameterType::Run,
        }
    }
}

/// Resolved parameter values
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterValues(pub HashMap<String, serde_json::Value>);

impl ParameterValues {
    /// Creates a new empty parameter values
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Creates from a hash map
    #[must_use]
    pub fn from_map(map: HashMap<String, serde_json::Value>) -> Self {
        Self(map)
    }

    /// Gets a string value
    #[must_use]
    pub fn get_string(&self, name: &str) -> Option<&str> {
        self.0.get(name).and_then(|v| v.as_str())
    }

    /// Gets a boolean value
    #[must_use]
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.0.get(name).and_then(|v| v.as_bool())
    }

    /// Sets a value
    pub fn set(&mut self, name: impl Into<String>, value: serde_json::Value) {
        self.0.insert(name.into(), value);
    }
}

impl crate::Validate for Parameters {
    type Error = crate::ValidationError;

    fn validate(&self) -> Result<(), Self::Error> {
        let mut names = std::collections::HashSet::new();
        for param in &self.0 {
            if !names.insert(param.name()) {
                return Err(crate::ValidationError::InvalidParameter {
                    name: param.name().to_string(),
                    reason: "duplicate parameter name".to_string(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::Validate;

    #[test]
    fn test_parameters_creation() {
        let params = Parameters::new();
        assert!(params.is_empty());
    }

    #[test]
    fn test_parameters_add_string() {
        let mut params = Parameters::new();
        params.push(Parameter::String {
            name: "VERSION".to_string(),
            description: "Version to build".to_string(),
            default_value: Some("1.0.0".to_string()),
            trim: true,
        });
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_parameters_get() {
        let mut params = Parameters::new();
        params.push(Parameter::String {
            name: "FOO".to_string(),
            description: "".to_string(),
            default_value: None,
            trim: false,
        });

        let foo = params.get("FOO");
        assert!(foo.is_some());
        assert_eq!(foo.unwrap().name(), "FOO");
    }

    #[test]
    fn test_parameter_name() {
        let param = Parameter::String {
            name: "TEST".to_string(),
            description: "Test param".to_string(),
            default_value: None,
            trim: false,
        };
        assert_eq!(param.name(), "TEST");
    }

    #[test]
    fn test_parameter_description() {
        let param = Parameter::Boolean {
            name: "DEBUG".to_string(),
            description: "Enable debug mode".to_string(),
            default_value: false,
        };
        assert_eq!(param.description(), "Enable debug mode");
    }

    #[test]
    fn test_parameter_type() {
        let param = Parameter::Choice {
            name: "ENV".to_string(),
            description: "Environment".to_string(),
            choices: vec!["dev".to_string(), "prod".to_string()],
            default_choice: None,
        };
        assert_eq!(param.parameter_type(), ParameterType::Choice);
    }

    #[test]
    fn test_parameters_validate_duplicate() {
        let params = Parameters::from_vec(vec![
            Parameter::String {
                name: "FOO".to_string(),
                description: "".to_string(),
                default_value: None,
                trim: false,
            },
            Parameter::String {
                name: "FOO".to_string(),
                description: "duplicate".to_string(),
                default_value: None,
                trim: false,
            },
        ]);
        assert!(params.validate().is_err());
    }
}
