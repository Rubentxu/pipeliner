//! ConfigTool - Load configuration values from PipelineConfig.

use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// ConfigTool provides operations to load configuration values.
#[derive(Debug, Clone, Default)]
pub struct ConfigTool;

impl ConfigTool {
    /// Creates a new ConfigTool instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl StepFactory for ConfigTool {
    fn name(&self) -> &str {
        "config"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "load"
        // args[1] = profiles (optional Vec<String>)
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "load" => {
                let profiles: Vec<String> = args
                    .get(1)
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let output = if profiles.is_empty() {
                    serde_json::json!({
                        "operation": "load",
                        "source": "environment"
                    })
                } else {
                    serde_json::json!({
                        "operation": "load",
                        "source": "environment",
                        "profiles": profiles
                    })
                };

                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or_default()),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!("Unknown operation: '{}'. Expected 'load'", operation),
            }),
        }
    }
}
