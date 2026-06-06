//! HTTP management client for app-scoped SDK API-key access.

use std::time::Duration;

use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::WorkerSdkError;

const API_KEY_HEADER: &str = "x-tikee-api-key";

/// App-scoped management client authenticated with `X-Tikee-API-Key`.
#[derive(Debug, Clone)]
pub struct ManagementClient {
    http: reqwest::Client,
    endpoint: String,
    api_key: String,
    namespace: String,
    app: String,
}

impl ManagementClient {
    /// Build a management client for one namespace/app API key scope.
    #[must_use]
    pub fn new(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        namespace: impl Into<String>,
        app: impl Into<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            endpoint: trim_trailing_slash(&endpoint.into()),
            api_key: api_key.into(),
            namespace: defaulted(&namespace.into(), "default"),
            app: defaulted(&app.into(), "default"),
        }
    }

    /// List jobs visible to this API key.
    ///
    /// # Errors
    ///
    /// Returns an SDK error for transport, server, or decoding failures.
    pub async fn list_jobs(&self) -> Result<Vec<JobDefinition>, WorkerSdkError> {
        let page: Page<JobDefinition> = self
            .send(reqwest::Method::GET, "/jobs", Option::<&()>::None)
            .await?;
        Ok(page
            .items
            .into_iter()
            .filter(|job| job.namespace == self.namespace && job.app == self.app)
            .collect())
    }

    /// Create one job in the configured namespace/app scope.
    ///
    /// # Errors
    ///
    /// Returns an SDK error for transport, server, or decoding failures.
    pub async fn create_job(
        &self,
        request: CreateJobRequest,
    ) -> Result<JobDefinition, WorkerSdkError> {
        let payload = ScopedCreateJobRequest {
            namespace: &self.namespace,
            app: &self.app,
            name: &request.name,
            schedule_type: request.schedule_type.as_deref(),
            schedule_expr: request.schedule_expr.as_deref(),
            processor_name: request.processor_name.as_deref(),
            processor_type: request.processor_type.as_deref(),
            script_id: request.script_id.as_deref(),
            enabled: request.enabled,
            retry_policy: request.retry_policy.as_ref(),
        };
        self.send(reqwest::Method::POST, "/jobs", Some(&payload))
            .await
    }

    async fn send<T, B>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, WorkerSdkError>
    where
        T: DeserializeOwned,
        B: Serialize + Sync + ?Sized,
    {
        let url = format!("{}/api/v1{}", self.endpoint, path);
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            API_KEY_HEADER,
            HeaderValue::from_str(&self.api_key).map_err(|error| {
                WorkerSdkError::ManagementRequestFailed(format!("invalid api key header: {error}"))
            })?,
        );
        let mut request = self.http.request(method, url).headers(headers);
        if let Some(body) = body {
            request = request.header(CONTENT_TYPE, "application/json").json(body);
        }
        let response = request.send().await.map_err(|error| {
            WorkerSdkError::ManagementRequestFailed(format!("request failed: {error}"))
        })?;
        let status = response.status();
        let envelope: ApiEnvelope<T> = response.json().await.map_err(|error| {
            WorkerSdkError::ManagementRequestFailed(format!("response decode failed: {error}"))
        })?;
        if !status.is_success() || envelope.code != 0 {
            return Err(WorkerSdkError::ManagementRequestFailed(format!(
                "management request failed: status={status} message={}",
                envelope.message
            )));
        }
        envelope.data.ok_or_else(|| {
            WorkerSdkError::ManagementRequestFailed("management response data was null".to_owned())
        })
    }
}

/// Structured failure retry policy. `max_attempts` includes the first execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRetryPolicy {
    /// Whether failed executions should be retried.
    pub enabled: bool,
    /// Total attempts including the first execution.
    pub max_attempts: i32,
    /// Delay before the first retry attempt, in seconds.
    pub initial_delay_seconds: i64,
    /// Integer multiplier applied to each subsequent retry delay.
    pub backoff_multiplier: i32,
    /// Upper bound for any retry delay, in seconds.
    pub max_delay_seconds: i64,
}

impl Default for JobRetryPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            initial_delay_seconds: 5,
            backoff_multiplier: 2,
            max_delay_seconds: 60,
        }
    }
}

/// Job definition returned by the tikee management API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDefinition {
    /// Job id.
    pub id: String,
    /// Namespace name.
    pub namespace: String,
    /// App name.
    pub app: String,
    /// Display name.
    pub name: String,
    /// Schedule type.
    pub schedule_type: String,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Optional processor binding.
    pub processor_name: Option<String>,
    /// Optional structured plugin processor type.
    pub processor_type: Option<String>,
    /// Optional script binding.
    pub script_id: Option<String>,
    /// Whether this job is enabled.
    pub enabled: bool,
    /// Structured failure retry policy applied to this job.
    pub retry_policy: JobRetryPolicy,
}

/// Job creation request for Rust management SDK users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateJobRequest {
    /// Display name.
    pub name: String,
    /// Schedule type. Defaults server-side when omitted.
    pub schedule_type: Option<String>,
    /// Optional schedule expression.
    pub schedule_expr: Option<String>,
    /// Optional processor binding.
    pub processor_name: Option<String>,
    /// Optional structured plugin processor type.
    pub processor_type: Option<String>,
    /// Optional script binding.
    pub script_id: Option<String>,
    /// Optional enabled flag.
    pub enabled: Option<bool>,
    /// Optional structured failure retry policy.
    pub retry_policy: Option<JobRetryPolicy>,
}

impl CreateJobRequest {
    /// Build an API-triggered processor job request.
    #[must_use]
    pub fn api(name: impl Into<String>, processor_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schedule_type: Some("api".to_owned()),
            schedule_expr: None,
            processor_name: Some(processor_name.into()),
            processor_type: None,
            script_id: None,
            enabled: Some(true),
            retry_policy: Some(JobRetryPolicy::default()),
        }
    }

    /// Build an API-triggered plugin processor job request.
    #[must_use]
    pub fn plugin_api(
        name: impl Into<String>,
        processor_type: impl Into<String>,
        processor_name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            schedule_type: Some("api".to_owned()),
            schedule_expr: None,
            processor_name: Some(processor_name.into()),
            processor_type: Some(processor_type.into()),
            script_id: None,
            enabled: Some(true),
            retry_policy: Some(JobRetryPolicy::default()),
        }
    }

    /// Build an API-triggered script job request.
    #[must_use]
    pub fn script_api(name: impl Into<String>, script_id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schedule_type: Some("api".to_owned()),
            schedule_expr: None,
            processor_name: None,
            processor_type: None,
            script_id: Some(script_id.into()),
            enabled: Some(true),
            retry_policy: Some(JobRetryPolicy::default()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page<T> {
    items: Vec<T>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScopedCreateJobRequest<'a> {
    namespace: &'a str,
    app: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedule_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedule_expr: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    processor_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    processor_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    script_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_policy: Option<&'a JobRetryPolicy>,
}

fn trim_trailing_slash(value: &str) -> String {
    value.trim_end_matches('/').to_owned()
}

fn defaulted(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}
