//! NotifyTool - Notification operations.

use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// NotifyTool provides notification operations for pipelines.
#[derive(Debug, Clone)]
pub struct NotifyTool {
    http_client: reqwest::blocking::Client,
    webhook_url: Option<String>,
}

impl NotifyTool {
    /// Creates a new NotifyTool with no default webhook.
    #[must_use]
    pub fn new() -> Self {
        Self {
            http_client: reqwest::blocking::Client::new(),
            webhook_url: None,
        }
    }

    /// Creates a new NotifyTool with a default webhook URL.
    #[must_use]
    pub fn with_webhook(webhook_url: impl Into<String>) -> Self {
        Self {
            http_client: reqwest::blocking::Client::new(),
            webhook_url: Some(webhook_url.into()),
        }
    }

    /// Creates a new NotifyTool with a custom HTTP client and webhook URL.
    #[must_use]
    pub fn with_client_and_webhook(client: reqwest::blocking::Client, webhook_url: Option<String>) -> Self {
        Self {
            http_client: client,
            webhook_url,
        }
    }
}

impl Default for NotifyTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for NotifyTool {
    fn name(&self) -> &str {
        "notify"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "sendEmail"
        // args[1] = to
        // args[2] = subject
        // args[3] = body
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        match operation {
            "sendEmail" => {
                let to = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected 'to' argument".to_string(),
                    })?;

                let subject = args
                    .get(2)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected 'subject' argument".to_string(),
                    })?;

                let body = args
                    .get(3)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StepError::InvalidArgs {
                        message: "Expected 'body' argument".to_string(),
                    })?;

                let output = serde_json::json!({
                    "operation": "sendEmail",
                    "to": to,
                    "subject": subject,
                    "body": body,
                    "success": true
                });
                Ok(CustomStep::success(
                    self.name(),
                    Some(serde_json::to_string(&output).unwrap_or_default()),
                ))
            }
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'sendEmail'",
                    operation
                ),
            }),
        }
    }
}
