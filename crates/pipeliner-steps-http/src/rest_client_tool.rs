//! RestClientTool - HTTP/REST client operations.

use pipeliner_core::registry::{CustomStep, StepError, StepFactory};
use serde_json::Value as JsonValue;

/// RestClientTool provides HTTP operations for pipelines.
#[derive(Debug, Clone)]
pub struct RestClientTool {
    timeout_secs: u64,
}

impl RestClientTool {
    /// Creates a new RestClientTool with default timeout.
    #[must_use]
    pub fn new() -> Self {
        Self { timeout_secs: 30 }
    }

    /// Creates a new RestClientTool with custom timeout.
    #[must_use]
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self { timeout_secs }
    }
}

impl Default for RestClientTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StepFactory for RestClientTool {
    fn name(&self) -> &str {
        "restClient"
    }

    fn create(&self, args: &[JsonValue]) -> Result<CustomStep, StepError> {
        // args[0] = operation: "get", "post", "put"
        // args[1] = url
        // args[2] = body/headers (optional)
        let operation = args
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected operation argument".to_string(),
            })?;

        let url = args
            .get(1)
            .and_then(|v| v.as_str())
            .ok_or_else(|| StepError::InvalidArgs {
                message: "Expected URL argument".to_string(),
            })?;

        let extra = args.get(2);

        match operation {
            "get" => self.do_get(url, extra),
            "post" => self.do_post(url, extra),
            "put" => self.do_put(url, extra),
            _ => Err(StepError::InvalidArgs {
                message: format!(
                    "Unknown operation: '{}'. Expected 'get', 'post', or 'put'",
                    operation
                ),
            }),
        }
    }
}

impl RestClientTool {
    fn do_get(&self, url: &str, extra: Option<&JsonValue>) -> Result<CustomStep, StepError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        let mut request = client.get(url);

        if let Some(extra_val) = extra {
            if let Some(headers) = extra_val.get("headers").and_then(|h| h.as_object()) {
                for (key, value) in headers {
                    if let Some(val_str) = value.as_str() {
                        request = request.header(key, val_str);
                    }
                }
            }
        }

        let response = request.send().map_err(|e| StepError::CreationFailed {
            message: format!("GET request failed: {}", e),
        })?;

        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();

        let output = serde_json::json!({
            "operation": "get",
            "url": url,
            "status": status,
            "body": body
        });

        Ok(CustomStep::success(
            self.name(),
            Some(serde_json::to_string(&output).unwrap_or_default()),
        ))
    }

    fn do_post(&self, url: &str, extra: Option<&JsonValue>) -> Result<CustomStep, StepError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        let mut request = client.post(url);

        if let Some(extra_val) = extra {
            if let Some(headers) = extra_val.get("headers").and_then(|h| h.as_object()) {
                for (key, value) in headers {
                    if let Some(val_str) = value.as_str() {
                        request = request.header(key, val_str);
                    }
                }
            }
            if let Some(body) = extra_val.get("body") {
                let body_str = body.to_string();
                request = request.body(body_str);
            }
        }

        let response = request.send().map_err(|e| StepError::CreationFailed {
            message: format!("POST request failed: {}", e),
        })?;

        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();

        let output = serde_json::json!({
            "operation": "post",
            "url": url,
            "status": status,
            "body": body
        });

        Ok(CustomStep::success(
            self.name(),
            Some(serde_json::to_string(&output).unwrap_or_default()),
        ))
    }

    fn do_put(&self, url: &str, extra: Option<&JsonValue>) -> Result<CustomStep, StepError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| StepError::CreationFailed {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        let mut request = client.put(url);

        if let Some(extra_val) = extra {
            if let Some(headers) = extra_val.get("headers").and_then(|h| h.as_object()) {
                for (key, value) in headers {
                    if let Some(val_str) = value.as_str() {
                        request = request.header(key, val_str);
                    }
                }
            }
            if let Some(body) = extra_val.get("body") {
                let body_str = body.to_string();
                request = request.body(body_str);
            }
        }

        let response = request.send().map_err(|e| StepError::CreationFailed {
            message: format!("PUT request failed: {}", e),
        })?;

        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();

        let output = serde_json::json!({
            "operation": "put",
            "url": url,
            "status": status,
            "body": body
        });

        Ok(CustomStep::success(
            self.name(),
            Some(serde_json::to_string(&output).unwrap_or_default()),
        ))
    }
}
