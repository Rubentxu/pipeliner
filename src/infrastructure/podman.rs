//! Native Podman container runtime implementation using Podman REST API
//!
//! This module provides a native Rust implementation for executing
//! pipelines in Podman containers using the Podman REST API over Unix socket.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use url::Url;

use crate::executor::{ExecutorCapabilities, HealthStatus, PipelineContext, PipelineExecutor};
use crate::pipeline::agent::{AgentType, PodmanConfig};
use crate::pipeline::{Pipeline, Stage, StageResult, Step, StepType, Validate};

const PODMAN_SOCKET_PATH: &str = "/run/podman/podman.sock";
const PODMAN_API_VERSION: &str = "v5.0.0";

#[derive(Error, Debug)]
pub enum PodmanError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("API request failed: {0}")]
    ApiRequest(String),

    #[error("API response error: {0}")]
    ApiResponse(String),

    #[error("Container creation failed: {0}")]
    ContainerCreateFailed(String),

    #[error("Container start failed: {0}")]
    ContainerStartFailed(String),

    #[error("Container wait failed: {0}")]
    ContainerWaitFailed(String),

    #[error("Container logs error: {0}")]
    ContainerLogsError(String),

    #[error("Container remove failed: {0}")]
    ContainerRemoveFailed(String),

    #[error("Image pull failed: {0}")]
    ImagePullFailed(String),

    #[error("Podman socket not found")]
    SocketNotFound,
}

#[derive(Debug, Clone)]
struct PodmanClientConfig {
    socket_path: PathBuf,
    timeout: Duration,
    api_version: String,
}

impl From<&PodmanConfig> for PodmanClientConfig {
    fn from(config: &PodmanConfig) -> Self {
        Self {
            socket_path: PathBuf::from(&config.socket_path),
            timeout: Duration::from_secs(300),
            api_version: config.api_version.clone(),
        }
    }
}

impl Default for PodmanClientConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from(PODMAN_SOCKET_PATH),
            timeout: Duration::from_secs(300),
            api_version: PODMAN_API_VERSION.to_string(),
        }
    }
}

pub struct PodmanClient {
    socket_path: PathBuf,
    base_url: Url,
    timeout: Duration,
}

impl PodmanClient {
    pub async fn new(config: PodmanClientConfig) -> Result<Self, PodmanError> {
        let socket_path = config.socket_path.clone();

        let stream = UnixStream::connect(&socket_path).await.map_err(|e| {
            PodmanError::ConnectionFailed(format!("Failed to connect to Podman socket: {}", e))
        })?;

        drop(stream);

        let base_url = Url::parse(&format!(
            "http://localhost/{}",
            config.api_version.trim_start_matches('v')
        ))
        .map_err(|e| PodmanError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            socket_path,
            base_url,
            timeout: config.timeout,
        })
    }

    pub async fn ping(&self) -> Result<(), PodmanError> {
        self.request("GET", "/_ping", None).await?;
        Ok(())
    }

    async fn send_http_request(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
    ) -> Result<(StatusCode, Vec<u8>), PodmanError> {
        let mut stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            PodmanError::ConnectionFailed(format!("Failed to connect to Podman socket: {}", e))
        })?;

        let host = "localhost";
        let api_version = self.base_url.path().trim_start_matches('/');

        let path_prefixed = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };

        let mut request = format!("{} {} HTTP/1.1\r\n", method, path_prefixed);
        request.push_str(&format!("Host: {}\r\n", host));
        request.push_str(&format!("Accept: application/json\r\n",));
        request.push_str(&format!("Api-Version: {}\r\n", api_version));

        if let Some(body) = body {
            request.push_str(&format!("Content-Type: application/json\r\n"));
            request.push_str(&format!("Content-Length: {}\r\n", body.len()));
            request.push_str("\r\n");
            request.push_str(
                std::str::from_utf8(body).map_err(|e| {
                    PodmanError::ApiRequest(format!("Invalid UTF-8 in body: {}", e))
                })?,
            );
        } else {
            request.push_str("\r\n");
        }

        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| PodmanError::ApiRequest(format!("Failed to write request: {}", e)))?;

        let mut response = Vec::new();
        let mut buf = [0u8; 8192];

        loop {
            let n = stream
                .read(&mut buf)
                .await
                .map_err(|e| PodmanError::ApiResponse(format!("Failed to read response: {}", e)))?;

            if n == 0 {
                break;
            }

            response.extend_from_slice(&buf[..n]);

            if response.ends_with(b"\r\n\r\n") {
                break;
            }
        }

        let body_start = response
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .unwrap_or(response.len());
        let header_end = body_start + 4;

        let status_line = std::str::from_utf8(&response[..body_start])
            .map_err(|e| PodmanError::ApiResponse(format!("Invalid status line: {}", e)))?;

        let status_code = parse_status_code(status_line)?;

        let body = response[header_end..].to_vec();

        Ok((status_code, body))
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
    ) -> Result<serde_json::Value, PodmanError> {
        let (status, body) = self.send_http_request(method, path, body).await?;

        if !status.is_success() {
            let msg = String::from_utf8_lossy(&body);
            return Err(PodmanError::ApiResponse(format!(
                "API error {}: {}",
                status.as_u16(),
                msg
            )));
        }

        if body.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        let json: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| PodmanError::ApiResponse(format!("JSON parse failed: {}", e)))?;

        Ok(json)
    }

    pub async fn create_container(
        &self,
        image: &str,
        command: &str,
        env: &HashMap<String, String>,
        working_dir: &str,
        cgroup_manager: Option<&str>,
    ) -> Result<String, PodmanError> {
        let mut host_config = serde_json::Map::new();

        if let Some(cg) = cgroup_manager {
            host_config.insert(
                "CgroupManager".to_string(),
                serde_json::Value::String(cg.to_string()),
            );
        } else {
            host_config.insert(
                "CgroupManager".to_string(),
                serde_json::Value::String("cgroupfs".to_string()),
            );
        }

        let mut env_vec: Vec<String> = vec![];
        for (k, v) in env {
            env_vec.push(format!("{}={}", k, v));
        }

        let container_config = serde_json::json!({
            "Image": image,
            "Cmd": ["sh", "-c", command],
            "WorkingDir": working_dir,
            "Env": env_vec,
            "HostConfig": host_config,
            "Tty": false,
            "OpenStdin": false,
        });

        let body = serde_json::to_string(&container_config).map_err(|e| {
            PodmanError::ContainerCreateFailed(format!("JSON serialization failed: {}", e))
        })?;

        let response: serde_json::Value = self
            .request("POST", "/containers/create", Some(body.as_bytes()))
            .await?;

        let id = response
            .get("Id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                PodmanError::ContainerCreateFailed("No container ID in response".to_string())
            })?
            .to_string();

        Ok(id)
    }

    pub async fn start_container(&self, container_id: &str) -> Result<(), PodmanError> {
        let path = format!("/containers/{}/start", container_id);
        self.request("POST", &path, None).await?;
        Ok(())
    }

    pub async fn wait_container(&self, container_id: &str) -> Result<i32, PodmanError> {
        let path = format!("/containers/{}/wait", container_id);
        let response: serde_json::Value = self.request("POST", &path, None).await?;

        let exit_code = response.as_i64().unwrap_or(0) as i32;
        Ok(exit_code)
    }

    pub async fn logs(&self, container_id: &str) -> Result<(Vec<u8>, Vec<u8>), PodmanError> {
        let path = format!("/containers/{}/logs?stdout=true&stderr=true", container_id);
        let response = self.request("GET", &path, None).await;

        match response {
            Ok(json) => {
                let logs = json.to_string().into_bytes();
                Ok((logs, Vec::new()))
            }
            Err(PodmanError::ApiResponse(_)) => Ok((Vec::new(), Vec::new())),
            Err(e) => Err(e),
        }
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<(), PodmanError> {
        let path = format!("/containers/{}", container_id);
        self.request("DELETE", &path, None).await?;
        Ok(())
    }

    pub async fn pull_image(&self, image: &str) -> Result<(), PodmanError> {
        let path = format!("/images/pull?reference={}", image);
        self.request("POST", &path, None).await?;
        Ok(())
    }
}

fn parse_status_code(status_line: &str) -> Result<StatusCode, PodmanError> {
    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(PodmanError::ApiResponse(format!(
            "Invalid status line: {}",
            status_line
        )));
    }

    let code: u16 = parts[1].parse().map_err(|_| {
        PodmanError::ApiResponse(format!("Invalid status code in: {}", status_line))
    })?;

    StatusCode::from_u16(code)
        .map_err(|_| PodmanError::ApiResponse(format!("Unknown status code: {}", code)))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum StatusCode {
    Continue = 100,
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritative = 203,
    NoContent = 204,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    RequestTimeout = 408,
    Conflict = 409,
    UnprocessableEntity = 422,
    InternalServerError = 500,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
}

impl StatusCode {
    fn from_u16(code: u16) -> Result<Self, ()> {
        Ok(match code {
            100 => Self::Continue,
            200 => Self::Ok,
            201 => Self::Created,
            202 => Self::Accepted,
            203 => Self::NonAuthoritative,
            204 => Self::NoContent,
            301 => Self::MovedPermanently,
            302 => Self::Found,
            303 => Self::SeeOther,
            304 => Self::NotModified,
            400 => Self::BadRequest,
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            404 => Self::NotFound,
            405 => Self::MethodNotAllowed,
            406 => Self::NotAcceptable,
            408 => Self::RequestTimeout,
            409 => Self::Conflict,
            422 => Self::UnprocessableEntity,
            500 => Self::InternalServerError,
            502 => Self::BadGateway,
            503 => Self::ServiceUnavailable,
            504 => Self::GatewayTimeout,
            _ => return Err(()),
        })
    }

    fn is_success(&self) -> bool {
        matches!(
            self,
            Self::Ok | Self::Created | Self::Accepted | Self::NoContent
        )
    }

    fn as_u16(&self) -> u16 {
        *self as u16
    }
}

#[derive(Debug, Clone)]
pub struct PodmanExecutor {
    config: PodmanClientConfig,
}

impl PodmanExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: PodmanClientConfig::default(),
        }
    }

    #[must_use]
    pub fn with_socket(mut self, socket: impl Into<PathBuf>) -> Self {
        self.config.socket_path = socket.into();
        self
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }
}

impl Default for PodmanExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineExecutor for PodmanExecutor {
    fn execute(&self, pipeline: &Pipeline) -> Result<StageResult, crate::pipeline::PipelineError> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            crate::pipeline::PipelineError::Io(format!("Failed to create runtime: {}", e))
        })?;

        rt.block_on(async { self.execute_async(pipeline).await })
    }

    fn validate(&self, pipeline: &Pipeline) -> Result<(), crate::pipeline::ValidationError> {
        pipeline.validate()
    }

    fn dry_run(&self, pipeline: &Pipeline) -> Result<StageResult, crate::pipeline::PipelineError> {
        tracing::info!(
            pipeline = %pipeline.name.clone().unwrap_or_default(),
            "Starting dry run"
        );

        pipeline
            .validate()
            .map_err(crate::pipeline::PipelineError::Validation)?;

        for stage in &pipeline.stages {
            tracing::info!(stage = %stage.name, "Would execute stage in Podman");
            for step in &stage.steps {
                tracing::debug!(step = %step.step_type, "Would execute step");
            }
        }

        Ok(StageResult::Success)
    }

    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            can_execute_shell: true,
            can_run_docker: true,
            can_run_kubernetes: false,
            supports_parallel: false,
            supports_caching: false,
            supports_timeout: true,
            supports_retry: true,
        }
    }

    fn health_check(&self) -> HealthStatus {
        let socket_path = &self.config.socket_path;
        if std::fs::exists(socket_path).unwrap_or(false) {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy {
                reason: format!("Podman socket not found at {}", socket_path.display()),
            }
        }
    }
}

impl PodmanExecutor {
    async fn execute_async(
        &self,
        pipeline: &Pipeline,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        let pipeline_id = pipeline
            .name
            .clone()
            .unwrap_or_else(|| "unnamed".to_string());

        tracing::info!(pipeline_id = %pipeline_id, "Starting Podman pipeline execution");

        let podman_config = match &pipeline.agent {
            AgentType::Podman(config) => config.clone(),
            _ => {
                return Err(crate::pipeline::PipelineError::AgentConfig(
                    "Pipeline must have a Podman agent configured".to_string(),
                ));
            }
        };

        let socket_path = std::path::PathBuf::from(&podman_config.socket_path);
        let config = PodmanClientConfig {
            socket_path,
            timeout: Duration::from_secs(300),
            api_version: podman_config.api_version.clone(),
        };

        let client = match PodmanClient::new(config).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "Failed to connect to Podman");
                return Err(crate::pipeline::PipelineError::AgentConfig(format!(
                    "Podman connection failed: {}",
                    e
                )));
            }
        };

        if let Err(e) = client.ping().await {
            tracing::error!(error = %e, "Podman ping failed");
            return Err(crate::pipeline::PipelineError::AgentConfig(format!(
                "Podman ping failed: {}",
                e
            )));
        }

        let mut context = PipelineContext::new();

        for (key, value) in &pipeline.environment.vars {
            context.set_env(key, value);
        }

        for stage in &pipeline.stages {
            let stage_name = stage.name.clone();
            tracing::info!(stage = %stage_name, "Executing stage in Podman");

            let result = self.execute_stage(&stage, &context, &client).await?;

            context.record_stage_result(&stage_name, result);

            if result.is_failure() && pipeline.options.retry.is_none() {
                tracing::error!(stage = %stage_name, "Stage failed, stopping pipeline");
                return Ok(result);
            }
        }

        Ok(StageResult::Success)
    }

    async fn execute_stage(
        &self,
        stage: &Stage,
        context: &PipelineContext,
        client: &PodmanClient,
    ) -> Result<StageResult, crate::pipeline::PipelineError> {
        let podman_config = match &stage.agent {
            Some(crate::pipeline::AgentType::Podman(config)) => config.clone(),
            Some(_) => {
                return Err(crate::pipeline::PipelineError::AgentConfig(
                    "Stage agent must be Podman for PodmanExecutor".to_string(),
                ));
            }
            None => {
                return Err(crate::pipeline::PipelineError::AgentConfig(
                    "Stage must have a Podman agent configured".to_string(),
                ));
            }
        };

        for step in &stage.steps {
            self.execute_step(step, &podman_config, context, client)
                .await?;
        }

        Ok(StageResult::Success)
    }

    async fn execute_step(
        &self,
        step: &Step,
        config: &PodmanConfig,
        context: &PipelineContext,
        client: &PodmanClient,
    ) -> Result<(), crate::pipeline::PipelineError> {
        match &step.step_type {
            StepType::Shell { command } => {
                self.run_in_container(config, command, context, client)
                    .await?;
            }
            StepType::Echo { message } => {
                println!("{message}");
            }
            _ => {
                tracing::warn!(step_type = %step.step_type, "Step type not implemented in Podman");
            }
        }
        Ok(())
    }

    async fn run_in_container(
        &self,
        config: &PodmanConfig,
        command: &str,
        context: &PipelineContext,
        client: &PodmanClient,
    ) -> Result<(), crate::pipeline::PipelineError> {
        let working_dir = config
            .working_dir
            .clone()
            .unwrap_or_else(|| context.cwd.to_string_lossy().into_owned());

        let env: HashMap<String, String> = config
            .environment
            .iter()
            .chain(context.env.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        tracing::debug!(image = %config.image, command = %command, "Creating container");

        let container_id = client
            .create_container(
                &config.image,
                command,
                &env,
                &working_dir,
                config.cgroup_manager.as_deref(),
            )
            .await
            .map_err(|e| {
                crate::pipeline::PipelineError::AgentConfig(format!("Container create: {}", e))
            })?;

        tracing::debug!(container_id = %container_id, "Starting container");

        client.start_container(&container_id).await.map_err(|e| {
            crate::pipeline::PipelineError::AgentConfig(format!("Container start: {}", e))
        })?;

        tracing::debug!(container_id = %container_id, "Waiting for container");

        let exit_code = client.wait_container(&container_id).await.map_err(|e| {
            crate::pipeline::PipelineError::AgentConfig(format!("Container wait: {}", e))
        })?;

        tracing::debug!(container_id = %container_id, exit_code = %exit_code, "Getting logs");

        let (stdout, stderr) = client.logs(&container_id).await.map_err(|e| {
            crate::pipeline::PipelineError::AgentConfig(format!("Container logs: {}", e))
        })?;

        if !stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(&stdout));
        }

        if !stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&stderr));
        }

        if config.remove {
            client.remove_container(&container_id).await.ok();
        }

        if exit_code != 0 {
            return Err(crate::pipeline::PipelineError::CommandFailed {
                code: exit_code,
                stderr: String::from_utf8_lossy(&stderr).to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_podman_executor_creation() {
        let executor = PodmanExecutor::new();
        let _ = executor;
    }

    #[test]
    fn test_podman_executor_capabilities() {
        let executor = PodmanExecutor::new();
        let caps = executor.capabilities();

        assert!(caps.can_execute_shell);
        assert!(caps.supports_timeout);
        assert!(caps.supports_retry);
    }
}
