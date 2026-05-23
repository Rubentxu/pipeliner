//! Step specification types for declarative pipeline definitions.
//!
//! This module defines the step types used in pipeline specifications,
//! including shell commands, echo messages, and configuration for
//! interpolation modes and shell kinds.

use serde::{Deserialize, Serialize};

use super::env_spec::EnvSpec;

/// A step specification that can be serialized to/from JSON.
///
/// This enum uses a `#[serde(tag)]` to serialize the step type as a JSON field,
/// allowing different step variants to be deserialized based on their type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepSpec {
    /// Shell command execution step
    Shell(ShellStepSpec),
    /// Echo message step
    Echo(EchoStepSpec),
    /// Directory-based steps execution
    Dir(DirStepSpec),
    /// Steps with environment variables
    WithEnv(WithEnvStepSpec),
    /// Steps that capture output to a variable
    LetOutput(LetOutputStepSpec),
    /// Steps with credentials injected into environment
    WithCredentials(WithCredentialsStepSpec),
    /// JUnit test report step
    JUnit(JUnitStepSpec),
    /// Archive step
    Archive(ArchiveStepSpec),
}

/// Specification for a shell command step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShellStepSpec {
    /// The shell interpreter to use
    pub kind: ShellKind,

    /// The script/command to execute
    pub script: String,

    /// Optional label for the step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Interpolation mode for variable expansion
    #[serde(default)]
    pub interpolation: InterpolationMode,

    /// Whether to capture stdout
    #[serde(default)]
    pub capture_stdout: bool,

    /// Whether to capture return status
    #[serde(default)]
    pub return_status: bool,

    /// Whether to fail on non-zero exit code
    #[serde(default = "default_fail_on_nonzero")]
    pub fail_on_nonzero: bool,
}

fn default_fail_on_nonzero() -> bool {
    true
}

impl ShellStepSpec {
    /// Creates a new shell step with the given script using the default shell (sh).
    #[must_use]
    pub fn new(script: &str) -> Self {
        Self {
            kind: ShellKind::Sh,
            script: script.to_string(),
            label: None,
            interpolation: InterpolationMode::default(),
            capture_stdout: false,
            return_status: false,
            fail_on_nonzero: true,
        }
    }

    /// Sets the label for this shell step.
    #[must_use]
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    /// Sets the shell kind for this step.
    #[must_use]
    pub fn with_kind(mut self, kind: ShellKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets the interpolation mode for this step.
    #[must_use]
    pub fn with_interpolation(mut self, interpolation: InterpolationMode) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Enables stdout capture.
    #[must_use]
    pub fn with_capture_stdout(mut self) -> Self {
        self.capture_stdout = true;
        self
    }

    /// Enables return status capture.
    #[must_use]
    pub fn with_return_status(mut self) -> Self {
        self.return_status = true;
        self
    }

    /// Disables fail on non-zero exit code.
    #[must_use]
    pub fn with_allow_failure(mut self) -> Self {
        self.fail_on_nonzero = false;
        self
    }
}

/// Specification for an echo step that outputs a message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EchoStepSpec {
    /// The message to output
    pub message: String,
}

/// Specification for a directory-based steps execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirStepSpec {
    /// The directory path to execute steps in
    pub path: String,

    /// Steps to execute in the directory
    pub steps: Vec<StepSpec>,
}

impl DirStepSpec {
    /// Creates a new directory step specification.
    #[must_use]
    pub fn new(path: impl Into<String>, steps: Vec<StepSpec>) -> Self {
        Self {
            path: path.into(),
            steps,
        }
    }
}

/// Specification for steps with environment variables.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WithEnvStepSpec {
    /// Environment variables to apply
    pub env: EnvSpec,

    /// Steps to execute with the environment
    pub steps: Vec<StepSpec>,
}

impl WithEnvStepSpec {
    /// Creates a new with-env step specification.
    #[must_use]
    pub fn new(env: EnvSpec, steps: Vec<StepSpec>) -> Self {
        Self { env, steps }
    }
}

/// Specification for capturing step output to a variable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LetOutputStepSpec {
    /// The variable name to capture output to
    pub var_name: String,

    /// The step whose output to capture
    pub inner: Box<StepSpec>,
}

impl LetOutputStepSpec {
    /// Creates a new let-output step specification.
    #[must_use]
    pub fn new(var_name: impl Into<String>, inner: StepSpec) -> Self {
        Self {
            var_name: var_name.into(),
            inner: Box::new(inner),
        }
    }
}

/// Specification for steps with credentials injected into environment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WithCredentialsStepSpec {
    /// Credential bindings to inject
    pub bindings: Vec<CredentialBinding>,
    /// Steps to execute with the credentials
    pub steps: Vec<StepSpec>,
}

impl WithCredentialsStepSpec {
    /// Creates a new with-credentials step specification.
    #[must_use]
    pub fn new(bindings: Vec<CredentialBinding>, steps: Vec<StepSpec>) -> Self {
        Self { bindings, steps }
    }
}

/// A credential binding - maps a credential ID to an environment variable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CredentialBinding {
    /// The environment variable name to set
    pub variable: String,
    /// The credential identifier to fetch
    pub credentials_id: String,
    /// Optional provider name (uses default chain if None)
    pub provider: Option<String>,
}

impl CredentialBinding {
    /// Creates a new credential binding.
    #[must_use]
    pub fn new(variable: impl Into<String>, credentials_id: impl Into<String>) -> Self {
        Self {
            variable: variable.into(),
            credentials_id: credentials_id.into(),
            provider: None,
        }
    }

    /// Sets the provider for this binding.
    #[must_use]
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }
}

/// Specification for JUnit test report step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JUnitStepSpec {
    /// Path to the JUnit XML report file
    pub report_path: String,
    /// Whether to allow failure of this step
    #[serde(default)]
    pub allow_failure: bool,
}

impl JUnitStepSpec {
    /// Creates a new JUnit step specification.
    #[must_use]
    pub fn new(report_path: impl Into<String>) -> Self {
        Self {
            report_path: report_path.into(),
            allow_failure: false,
        }
    }

    /// Allows failure for this step.
    #[must_use]
    pub fn with_allow_failure(mut self) -> Self {
        self.allow_failure = true;
        self
    }
}

/// Specification for archive step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchiveStepSpec {
    /// Paths to archive (glob patterns supported)
    pub paths: Vec<String>,
    /// Name of the archive artifact
    pub artifact_name: String,
    /// Optional compression type (zip, tar.gz, etc.)
    #[serde(default)]
    pub compression: Option<String>,
}

impl ArchiveStepSpec {
    /// Creates a new archive step specification.
    #[must_use]
    pub fn new(paths: Vec<String>, artifact_name: impl Into<String>) -> Self {
        Self {
            paths,
            artifact_name: artifact_name.into(),
            compression: None,
        }
    }

    /// Sets the compression type.
    #[must_use]
    pub fn with_compression(mut self, compression: impl Into<String>) -> Self {
        self.compression = Some(compression.into());
        self
    }
}

/// Shell interpreter kinds supported by pipeline steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellKind {
    /// POSIX shell (sh)
    Sh,
    /// PowerShell
    PowerShell,
    /// Windows Command Prompt
    Cmd,
}

/// Interpolation mode for variable expansion in scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMode {
    /// Pipeliner-style interpolation (expand variables)
    #[default]
    Pipeliner,
    /// Raw mode (no interpolation)
    Raw,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_spec_shell_variant() {
        let shell = StepSpec::Shell(ShellStepSpec::new("echo hello"));
        assert!(matches!(shell, StepSpec::Shell(_)));
    }

    #[test]
    fn test_step_spec_echo_variant() {
        let echo = StepSpec::Echo(EchoStepSpec {
            message: "test message".to_string(),
        });
        assert!(matches!(echo, StepSpec::Echo(_)));
    }

    #[test]
    fn test_shell_step_spec_builder_pattern() {
        let step = ShellStepSpec::new("echo hello")
            .with_label("my step")
            .with_kind(ShellKind::PowerShell)
            .with_capture_stdout()
            .with_allow_failure();

        assert_eq!(step.script, "echo hello");
        assert_eq!(step.label, Some("my step".to_string()));
        assert_eq!(step.kind, ShellKind::PowerShell);
        assert!(step.capture_stdout);
        assert!(!step.fail_on_nonzero);
    }

    #[test]
    fn test_shell_kind_serialization() {
        assert_eq!(serde_json::to_string(&ShellKind::Sh).unwrap(), "\"sh\"");
        assert_eq!(serde_json::to_string(&ShellKind::PowerShell).unwrap(), "\"power_shell\"");
        assert_eq!(serde_json::to_string(&ShellKind::Cmd).unwrap(), "\"cmd\"");
    }

    #[test]
    fn test_interpolation_mode_default() {
        assert_eq!(InterpolationMode::default(), InterpolationMode::Pipeliner);
    }

    #[test]
    fn test_step_spec_shell_json_roundtrip() {
        let original = StepSpec::Shell(
            ShellStepSpec::new("echo hello")
                .with_label("test")
        );

        let json = serde_json::to_string(&original).unwrap();
        let parsed: StepSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_step_spec_echo_json_roundtrip() {
        let original = StepSpec::Echo(EchoStepSpec {
            message: "Hello, World!".to_string(),
        });

        let json = serde_json::to_string(&original).unwrap();
        let parsed: StepSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_dir_step_spec_creation() {
        let steps = vec![
            StepSpec::Echo(EchoStepSpec {
                message: "in dir".to_string(),
            }),
        ];
        let dir = DirStepSpec::new("/tmp", steps);

        assert_eq!(dir.path, "/tmp");
        assert_eq!(dir.steps.len(), 1);
    }

    #[test]
    fn test_dir_step_spec_in_step_spec() {
        let steps = vec![StepSpec::Echo(EchoStepSpec {
            message: "test".to_string(),
        })];
        let dir = StepSpec::Dir(DirStepSpec::new("/app", steps));

        assert!(matches!(dir, StepSpec::Dir(_)));
    }

    #[test]
    fn test_dir_step_spec_json_roundtrip() {
        let original = StepSpec::Dir(DirStepSpec::new(
            "/tmp",
            vec![StepSpec::Echo(EchoStepSpec {
                message: "hello".to_string(),
            })],
        ));

        let json = serde_json::to_string(&original).unwrap();
        let parsed: StepSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_with_env_step_spec_creation() {
        let env = EnvSpec::new()
            .with_var("FOO", "bar")
            .with_var("BAZ", "qux");

        let steps = vec![StepSpec::Echo(EchoStepSpec {
            message: "with env".to_string(),
        })];

        let with_env = WithEnvStepSpec::new(env, steps);

        assert_eq!(with_env.env.get("FOO"), Some("bar"));
        assert_eq!(with_env.env.get("BAZ"), Some("qux"));
        assert_eq!(with_env.steps.len(), 1);
    }

    #[test]
    fn test_with_env_step_spec_in_step_spec() {
        let env = EnvSpec::new().with_var("KEY", "value");
        let steps = vec![StepSpec::Echo(EchoStepSpec {
            message: "test".to_string(),
        })];

        let with_env = StepSpec::WithEnv(WithEnvStepSpec::new(env, steps));

        assert!(matches!(with_env, StepSpec::WithEnv(_)));
    }

    #[test]
    fn test_with_env_step_spec_json_roundtrip() {
        let env = EnvSpec::new().with_var("TEST", "value");
        let original = StepSpec::WithEnv(WithEnvStepSpec::new(
            env,
            vec![StepSpec::Echo(EchoStepSpec {
                message: "hello".to_string(),
            })],
        ));

        let json = serde_json::to_string(&original).unwrap();
        let parsed: StepSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_let_output_step_spec_creation() {
        let inner = StepSpec::Echo(EchoStepSpec {
            message: "captured".to_string(),
        });

        let let_output = LetOutputStepSpec::new("RESULT", inner);

        assert_eq!(let_output.var_name, "RESULT");
    }

    #[test]
    fn test_let_output_step_spec_in_step_spec() {
        let inner = StepSpec::Echo(EchoStepSpec {
            message: "test".to_string(),
        });

        let let_output = StepSpec::LetOutput(LetOutputStepSpec::new("MY_VAR", inner));

        assert!(matches!(let_output, StepSpec::LetOutput(_)));
    }

    #[test]
    fn test_let_output_step_spec_json_roundtrip() {
        let original = StepSpec::LetOutput(LetOutputStepSpec::new(
            "OUTPUT",
            StepSpec::Echo(EchoStepSpec {
                message: "hello".to_string(),
            }),
        ));

        let json = serde_json::to_string(&original).unwrap();
        let parsed: StepSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, original);
    }
}
