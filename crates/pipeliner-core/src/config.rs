//! Pipeline configuration types for Pipeliner.
//!
//! This module provides types for parsing and validating pipeline configuration
//! from YAML and JSON formats, including libraries, credentials, SCM, and environment settings.
//!
//! # Example
//!
//! ```rust
//! use pipeliner_core::config::{PipelineConfig, RetrieverType, LibraryConfig};
//!
//! let yaml = r#"
//! version: "1"
//! spec:
//!   pipeline:
//!     name: MyPipeline
//!     stages:
//!       - name: Build
//!         steps:
//!           - type: echo
//!             message: Hello
//! "#;
//!
//! let config = PipelineConfig::from_yaml(yaml).expect("Valid YAML");
//! assert_eq!(config.version, "1");
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::pipeline::Pipeline;

/// Configuration error types for pipeline configuration parsing and validation.
#[derive(Debug)]
pub enum ConfigError {
    /// YAML parsing error
    Yaml(serde_yaml::Error),
    /// JSON parsing error
    Json(serde_json::Error),
    /// Validation error with descriptive message
    Validation(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Yaml(e) => write!(f, "YAML parsing error: {e}"),
            ConfigError::Json(e) => write!(f, "JSON parsing error: {e}"),
            ConfigError::Validation(msg) => write!(f, "Validation error: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigError::Yaml(err)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::Json(err)
    }
}

/// Pipeline configuration root type.
///
/// This is the top-level configuration structure that can be parsed from
/// YAML or JSON files defining a complete pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Configuration version
    pub version: String,
    /// Pipeline specification
    pub spec: PipelineSpec,
}

impl PipelineConfig {
    /// Parse configuration from a YAML string.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Yaml` if the YAML is malformed.
    /// Returns `ConfigError::Validation` if the configuration fails validation.
    pub fn from_yaml(yaml: &str) -> Result<Self, ConfigError> {
        let config: PipelineConfig = serde_yaml::from_str(yaml)?;
        config.validate()?;
        Ok(config)
    }

    /// Parse configuration from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Json` if the JSON is malformed.
    /// Returns `ConfigError::Validation` if the configuration fails validation.
    pub fn from_json(json: &str) -> Result<Self, ConfigError> {
        let config: PipelineConfig = serde_json::from_str(json)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Validation` if version is empty or other validation fails.
    fn validate(&self) -> Result<(), ConfigError> {
        if self.version.is_empty() {
            return Err(ConfigError::Validation(
                "version must be non-empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Pipeline specification containing all configuration sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineSpec {
    /// Library configurations
    #[serde(default)]
    pub libraries: Vec<LibraryConfig>,
    /// Environment variables
    #[serde(default)]
    pub environment: HashMap<String, String>,
    /// Source code management configuration
    #[serde(default)]
    pub scm: Option<ScmConfig>,
    /// Credential configurations
    #[serde(default)]
    pub credentials: Vec<CredentialConfig>,
    /// Pipeline definition
    #[serde(default)]
    pub pipeline: Option<Pipeline>,
}

impl Default for PipelineSpec {
    fn default() -> Self {
        Self {
            libraries: Vec::new(),
            environment: HashMap::new(),
            scm: None,
            credentials: Vec::new(),
            pipeline: None,
        }
    }
}

/// Source retriever type for library resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RetrieverType {
    /// Git repository source
    GitSource,
    /// Local filesystem source
    LocalSource,
    /// Local library/JAR source
    LocalLib,
}

/// Library module definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryModule {
    /// Module name
    pub name: String,
    /// Module path within the library
    pub path: String,
}

/// Library configuration defining how a library is loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryConfig {
    /// Library name
    pub name: String,
    /// Source path (URL for git, path for local)
    pub source_path: String,
    /// Type of source retriever to use
    pub retriever_type: RetrieverType,
    /// Default version to use if not specified
    #[serde(default)]
    pub default_version: Option<String>,
    /// Modules to load from this library
    #[serde(default)]
    pub modules: Vec<LibraryModule>,
}

/// SCM (Source Code Management) configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScmConfig {
    /// Repository URL
    pub url: String,
    /// Branch to checkout
    pub branch: String,
    /// Credentials ID for authentication
    #[serde(default)]
    pub credentials_id: Option<String>,
    /// Whether to perform shallow clone
    #[serde(default = "default_true")]
    pub shallow_clone: bool,
    /// Whether to initialize submodules recursively
    #[serde(default = "default_true")]
    pub submodule_recursive: bool,
}

fn default_true() -> bool {
    true
}

/// Credential type variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CredentialType {
    /// Username/password credentials
    UsernamePassword,
    /// SSH key credentials
    SSHKey,
    /// Secret text credentials
    SecretText,
    /// Token credentials
    Token,
}

/// Credential configuration defining how to inject credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialConfig {
    /// Credential identifier
    pub id: String,
    /// Type of credential
    pub credential_type: CredentialType,
    /// Credential fields (key-value pairs specific to the credential type)
    #[serde(default)]
    pub fields: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // =======================================================================
    // A1: PipelineConfig Serialization Tests
    // =======================================================================

    #[test]
    fn test_pipeline_config_from_yaml_roundtrip() {
        // SCN-PC-004: Roundtrip serialization
        let yaml = r#"
version: "1"
spec:
  pipeline:
    name: TestPipeline
    stages:
      - name: Build
        steps:
          - type: echo
            message: Hello
"#;

        let config = PipelineConfig::from_yaml(yaml).expect("Should parse YAML");
        assert_eq!(config.version, "1");
        assert!(config.spec.pipeline.is_some());

        // Roundtrip: serialize back to YAML and parse again
        let yaml_out = serde_yaml::to_string(&config).expect("Should serialize to YAML");
        let config2 = PipelineConfig::from_yaml(&yaml_out).expect("Should parse roundtripped YAML");
        assert_eq!(config.version, config2.version);
    }

    #[test]
    fn test_pipeline_config_from_json_with_minimal_yaml() {
        // SCN-PC-003: Minimal JSON - version="1", pipeline with just a name and empty stages
        // Note: stages is required in Pipeline, so we provide minimal stages
        let json = r#"{
            "version": "1",
            "spec": {
                "pipeline": {
                    "name": "MinimalPipeline",
                    "stages": []
                }
            }
        }"#;

        let config = PipelineConfig::from_json(json).expect("Should parse JSON");
        assert_eq!(config.version, "1");
        assert!(config.spec.pipeline.is_some());
        let pipeline = config.spec.pipeline.unwrap();
        assert_eq!(pipeline.name, Some("MinimalPipeline".to_string()));
    }

    #[test]
    fn test_pipeline_config_with_full_structure() {
        // SCN-PC-001: Full structure - libraries, scm, credentials, pipeline
        let yaml = r#"
version: "1"
spec:
  libraries:
    - name: mylib
      sourcePath: https://github.com/example/mylib
      retrieverType: gitSource
      defaultVersion: main
      modules:
        - name: core
          path: src/core
  environment:
    FOO: bar
    BAZ: qux
  scm:
    url: https://github.com/example/repo
    branch: main
    credentialsId: github-creds
    shallowClone: true
    submoduleRecursive: false
  credentials:
    - id: github-creds
      credentialType: usernamePassword
      fields:
        username: user
        password: pass
  pipeline:
    name: FullPipeline
    stages:
      - name: Test
        steps:
          - type: echo
            message: Testing
"#;

        let config = PipelineConfig::from_yaml(yaml).expect("Should parse full YAML");
        assert_eq!(config.version, "1");

        // Libraries
        assert_eq!(config.spec.libraries.len(), 1);
        let lib = &config.spec.libraries[0];
        assert_eq!(lib.name, "mylib");
        assert_eq!(lib.source_path, "https://github.com/example/mylib");
        assert_eq!(lib.retriever_type, RetrieverType::GitSource);
        assert_eq!(lib.default_version, Some("main".to_string()));
        assert_eq!(lib.modules.len(), 1);

        // Environment
        assert_eq!(config.spec.environment.len(), 2);
        assert_eq!(config.spec.environment.get("FOO"), Some(&"bar".to_string()));

        // SCM
        let scm = config.spec.scm.as_ref().expect("SCM should be present");
        assert_eq!(scm.url, "https://github.com/example/repo");
        assert_eq!(scm.branch, "main");
        assert_eq!(scm.credentials_id.as_deref(), Some("github-creds"));
        assert!(scm.shallow_clone);
        assert!(!scm.submodule_recursive);

        // Credentials
        assert_eq!(config.spec.credentials.len(), 1);
        let cred = &config.spec.credentials[0];
        assert_eq!(cred.id, "github-creds");
        assert!(matches!(cred.credential_type, CredentialType::UsernamePassword));
        assert_eq!(cred.fields.get("username"), Some(&"user".to_string()));

        // Pipeline
        let pipeline = config.spec.pipeline.expect("Pipeline should be present");
        assert_eq!(pipeline.name, Some("FullPipeline".to_string()));
        assert_eq!(pipeline.stages.len(), 1);
    }

    #[test]
    fn test_scm_config_serde_defaults() {
        // SCN-PC-005: Serde defaults - shallow_clone=true, submodule_recursive=true when omitted
        let yaml = r#"
url: https://github.com/example/repo
branch: develop
"#;

        let scm: ScmConfig = serde_yaml::from_str(yaml).expect("Should parse SCM");
        assert_eq!(scm.url, "https://github.com/example/repo");
        assert_eq!(scm.branch, "develop");
        assert_eq!(scm.credentials_id, None);
        // Default values
        assert!(scm.shallow_clone, "shallow_clone should default to true");
        assert!(scm.submodule_recursive, "submodule_recursive should default to true");
    }

    // =======================================================================
    // A3: Subconfig Serialization Tests
    // =======================================================================

    #[test]
    fn test_library_config_serde_roundtrip() {
        // SCN-PC-001: LibraryConfig roundtrip
        let lib = LibraryConfig {
            name: "maven-lib".to_string(),
            source_path: "/path/to/lib".to_string(),
            retriever_type: RetrieverType::LocalSource,
            default_version: Some("1.0.0".to_string()),
            modules: vec![
                LibraryModule {
                    name: "core".to_string(),
                    path: "src/core".to_string(),
                },
                LibraryModule {
                    name: "api".to_string(),
                    path: "src/api".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&lib).expect("Should serialize to JSON");
        let parsed: LibraryConfig = serde_json::from_str(&json).expect("Should deserialize from JSON");

        assert_eq!(parsed.name, lib.name);
        assert_eq!(parsed.source_path, lib.source_path);
        assert_eq!(parsed.retriever_type, lib.retriever_type);
        assert_eq!(parsed.default_version, lib.default_version);
        assert_eq!(parsed.modules.len(), lib.modules.len());
    }

    #[test]
    fn test_scm_config_serde() {
        // SCN-PC-005: ScmConfig serde
        let scm = ScmConfig {
            url: "https://github.com/example/repo".to_string(),
            branch: "feature-branch".to_string(),
            credentials_id: Some("ssh-key".to_string()),
            shallow_clone: false,
            submodule_recursive: true,
        };

        let json = serde_json::to_string(&scm).expect("Should serialize SCM to JSON");
        let parsed: ScmConfig = serde_json::from_str(&json).expect("Should deserialize SCM from JSON");

        assert_eq!(parsed.url, scm.url);
        assert_eq!(parsed.branch, scm.branch);
        assert_eq!(parsed.credentials_id, scm.credentials_id);
        assert_eq!(parsed.shallow_clone, scm.shallow_clone);
        assert_eq!(parsed.submodule_recursive, scm.submodule_recursive);
    }

    #[test]
    fn test_credential_config_with_each_type_variant() {
        // SCN-PC-006: CredentialConfig with each type variant

        // UsernamePassword
        let cred_userpass = CredentialConfig {
            id: "userpass-creds".to_string(),
            credential_type: CredentialType::UsernamePassword,
            fields: HashMap::new(),
        };
        let json = serde_json::to_string(&cred_userpass).expect("Should serialize UsernamePassword");
        assert!(json.contains("usernamePassword"));

        // SSHKey
        let cred_ssh = CredentialConfig {
            id: "ssh-creds".to_string(),
            credential_type: CredentialType::SSHKey,
            fields: HashMap::new(),
        };
        let json = serde_json::to_string(&cred_ssh).expect("Should serialize SSHKey");
        assert!(json.contains("sSHKey"));

        // SecretText
        let cred_secret = CredentialConfig {
            id: "secret-creds".to_string(),
            credential_type: CredentialType::SecretText,
            fields: HashMap::new(),
        };
        let json = serde_json::to_string(&cred_secret).expect("Should serialize SecretText");
        assert!(json.contains("secretText"));

        // Token
        let cred_token = CredentialConfig {
            id: "token-creds".to_string(),
            credential_type: CredentialType::Token,
            fields: HashMap::new(),
        };
        let json = serde_json::to_string(&cred_token).expect("Should serialize Token");
        assert!(json.contains("token"));
    }

    #[test]
    fn test_retriever_type_variants() {
        // Test all RetrieverType variants serialize correctly
        let git = RetrieverType::GitSource;
        let json = serde_json::to_string(&git).expect("Should serialize GitSource");
        assert!(json.contains("gitSource"));

        let local = RetrieverType::LocalSource;
        let json = serde_json::to_string(&local).expect("Should serialize LocalSource");
        assert!(json.contains("localSource"));

        let lib = RetrieverType::LocalLib;
        let json = serde_json::to_string(&lib).expect("Should serialize LocalLib");
        assert!(json.contains("localLib"));
    }

    // =======================================================================
    // A5: Error Handling Tests
    // =======================================================================

    #[test]
    fn test_pipeline_config_from_yaml_invalid_returns_err() {
        // A5.1: Invalid YAML returns Err
        let invalid_yaml = r#"
version: "1"
spec:
  pipeline:
    name: [invalid yaml here
"#;

        let result = PipelineConfig::from_yaml(invalid_yaml);
        assert!(result.is_err(), "Should return error for invalid YAML");
        if let Err(ConfigError::Yaml(_)) = result {
            // Expected error type
        } else {
            panic!("Expected ConfigError::Yaml variant");
        }
    }

    #[test]
    fn test_pipeline_config_from_json_invalid_returns_err() {
        // A5.2: Invalid JSON returns Err
        let invalid_json = r#"{"version": "1", "spec": }"#;

        let result = PipelineConfig::from_json(invalid_json);
        assert!(result.is_err(), "Should return error for invalid JSON");
        if let Err(ConfigError::Json(_)) = result {
            // Expected error type
        } else {
            panic!("Expected ConfigError::Json variant");
        }
    }

    #[test]
    fn test_yaml_and_json_equivalent_inputs_produce_equal_config() {
        // A5.3: YAML and JSON equivalent inputs produce equal PipelineConfig
        let yaml_input = r#"
version: "1"
spec:
  pipeline:
    name: EquivPipeline
    stages:
      - name: Build
        steps:
          - type: echo
            message: Test
"#;

        let json_input = r#"{
            "version": "1",
            "spec": {
                "pipeline": {
                    "name": "EquivPipeline",
                    "stages": [
                        {
                            "name": "Build",
                            "steps": [
                                {"type": "echo", "message": "Test"}
                            ]
                        }
                    ]
                }
            }
        }"#;

        let config_yaml = PipelineConfig::from_yaml(yaml_input).expect("Should parse YAML");
        let config_json = PipelineConfig::from_json(json_input).expect("Should parse JSON");

        assert_eq!(config_yaml.version, config_json.version);
        assert_eq!(config_yaml.spec.pipeline, config_json.spec.pipeline);
    }

    // =======================================================================
    // Validation Tests
    // =======================================================================

    #[test]
    fn test_config_error_display() {
        // Test that ConfigError implements Display correctly
        let yaml_err = serde_yaml::from_str::<PipelineConfig>("not: valid").unwrap_err();
        let config_err = ConfigError::Yaml(yaml_err);
        let display = format!("{}", config_err);
        assert!(!display.is_empty(), "ConfigError should implement Display");
    }

    #[test]
    fn test_empty_version_validation() {
        // Validation: empty version should fail
        let yaml = r#"
version: ""
spec: {}
"#;
        let result = PipelineConfig::from_yaml(yaml);
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("version"));
        }
    }

    // =======================================================================
    // Default Value Tests
    // =======================================================================

    #[test]
    fn test_library_config_defaults() {
        // LibraryConfig with minimal fields should use defaults
        let yaml = r#"
name: minimal-lib
sourcePath: /path/to/lib
retrieverType: localSource
"#;

        let lib: LibraryConfig = serde_yaml::from_str(yaml).expect("Should parse minimal LibraryConfig");
        assert_eq!(lib.name, "minimal-lib");
        assert_eq!(lib.default_version, None, "default_version should be None by default");
        assert!(lib.modules.is_empty(), "modules should be empty by default");
    }

    #[test]
    fn test_credential_config_defaults() {
        // CredentialConfig with minimal fields should use defaults
        let yaml = r#"
id: my-creds
credentialType: secretText
"#;

        let cred: CredentialConfig = serde_yaml::from_str(yaml).expect("Should parse minimal CredentialConfig");
        assert_eq!(cred.id, "my-creds");
        assert!(cred.fields.is_empty(), "fields should be empty by default");
    }

    #[test]
    fn test_pipeline_spec_defaults() {
        // PipelineSpec with no optional fields should use defaults
        let yaml = r#"
pipeline:
  name: TestPipeline
  stages:
    - name: Build
      steps:
        - type: echo
          message: Hello
"#;

        let spec: PipelineSpec = serde_yaml::from_str(yaml).expect("Should parse PipelineSpec");
        assert!(spec.libraries.is_empty(), "libraries should be empty by default");
        assert!(spec.environment.is_empty(), "environment should be empty by default");
        assert!(spec.scm.is_none(), "scm should be None by default");
        assert!(spec.credentials.is_empty(), "credentials should be empty by default");
        assert!(spec.pipeline.is_some(), "pipeline should be present");
    }
}
