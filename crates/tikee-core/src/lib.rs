//! Core domain types shared by tikee crates.

#![forbid(unsafe_code)]

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// A lightweight health state exposed by management surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    /// Component is alive and able to respond.
    Ok,
}

impl HealthState {
    /// Returns the stable wire representation for this state.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
        }
    }
}

/// Supported job schedule type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    /// Job is triggered only through an explicit API/SDK/UI management call; it does not mean an HTTP-calling task.
    Api,
    /// Job is triggered by a CRON expression.
    Cron,
    /// Job is triggered at a fixed rate.
    FixedRate,
    /// Job is triggered with a fixed delay after previous completion.
    FixedDelay,
}

impl ScheduleType {
    /// Returns the stable storage and wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Cron => "cron",
            Self::FixedRate => "fixed_rate",
            Self::FixedDelay => "fixed_delay",
        }
    }
}

impl fmt::Display for ScheduleType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ScheduleType {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "api" => Ok(Self::Api),
            "cron" => Ok(Self::Cron),
            "fixed_rate" | "fixed-rate" | "fixedrate" => Ok(Self::FixedRate),
            "fixed_delay" | "fixed-delay" | "fixeddelay" => Ok(Self::FixedDelay),
            _ => Err(ParseEnumError::new("schedule_type", value)),
        }
    }
}

/// Execution fan-out mode for a job instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Dispatch to one eligible worker.
    Single,
    /// Dispatch once to every selected worker.
    Broadcast,
}

impl ExecutionMode {
    /// Returns the stable storage and wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Broadcast => "broadcast",
        }
    }
}

impl fmt::Display for ExecutionMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ExecutionMode {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "single" => Ok(Self::Single),
            "broadcast" => Ok(Self::Broadcast),
            _ => Err(ParseEnumError::new("execution_mode", value)),
        }
    }
}

/// Source that triggered a job instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// Explicit management API/SDK/UI trigger; it does not mean an HTTP-calling task.
    Api,
    /// CRON tikee trigger.
    Cron,
    /// Fixed-rate tikee trigger.
    FixedRate,
    /// Manual operator trigger from UI or CLI.
    Manual,
    /// Workflow shard fan-out trigger.
    WorkflowShard,
}

impl TriggerType {
    /// Returns the stable storage and wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Cron => "cron",
            Self::FixedRate => "fixed_rate",
            Self::Manual => "manual",
            Self::WorkflowShard => "workflow_shard",
        }
    }
}

impl fmt::Display for TriggerType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for TriggerType {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "api" => Ok(Self::Api),
            "cron" => Ok(Self::Cron),
            "fixed_rate" | "fixed-rate" | "fixedrate" => Ok(Self::FixedRate),
            "manual" => Ok(Self::Manual),
            "workflow_shard" | "workflow-shard" | "workflowshard" => Ok(Self::WorkflowShard),
            _ => Err(ParseEnumError::new("trigger_type", value)),
        }
    }
}

/// Job instance lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    /// Instance has been created and is waiting for dispatch.
    Pending,
    /// Instance is being matched to a worker.
    Dispatching,
    /// Worker is executing the instance.
    Running,
    /// Instance completed successfully.
    Succeeded,
    /// Broadcast instance had at least one failed child execution.
    PartialFailed,
    /// Instance failed.
    Failed,
    /// Instance was cancelled.
    Cancelled,
}

impl InstanceStatus {
    /// Returns the stable storage and wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Dispatching => "dispatching",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::PartialFailed => "partial_failed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for InstanceStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for InstanceStatus {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "dispatching" => Ok(Self::Dispatching),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "partial_failed" | "partial-failed" | "partialfailed" => Ok(Self::PartialFailed),
            "failed" => Ok(Self::Failed),
            "cancelled" | "canceled" => Ok(Self::Cancelled),
            _ => Err(ParseEnumError::new("instance_status", value)),
        }
    }
}

/// Result of attempting to dispatch a pending instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum DispatchDecision {
    /// Instance should remain queued until a worker is available.
    Queued,
    /// Instance was assigned to a worker.
    Assigned {
        /// Selected worker identifier.
        worker_id: String,
    },
    /// No eligible worker currently exists.
    NoEligibleWorker,
}

/// Script language supported by the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptLanguage {
    /// Shell / Bash script.
    Shell,
    /// Python script.
    Python,
    /// JavaScript script.
    Js,
    /// TypeScript script.
    Ts,
    /// PowerShell script.
    PowerShell,
    /// Rhai embedded script.
    Rhai,
    /// WebAssembly module.
    Wasm,
}

impl ScriptLanguage {
    /// Returns the stable storage and wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Python => "python",
            Self::Js => "js",
            Self::Ts => "ts",
            Self::PowerShell => "powershell",
            Self::Rhai => "rhai",
            Self::Wasm => "wasm",
        }
    }
}

impl fmt::Display for ScriptLanguage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ScriptLanguage {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "shell" | "bash" | "sh" => Ok(Self::Shell),
            "python" | "py" => Ok(Self::Python),
            "node" | "nodejs" | "javascript" | "js" => Ok(Self::Js),
            "typescript" | "ts" => Ok(Self::Ts),
            "powershell" | "ps1" | "pwsh" => Ok(Self::PowerShell),
            "rhai" => Ok(Self::Rhai),
            "wasm" | "webassembly" => Ok(Self::Wasm),
            _ => Err(ParseEnumError::new("script_language", value)),
        }
    }
}

/// Approval status for a script definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptStatus {
    /// Script is a draft and not yet approved for execution.
    Draft,
    /// Script has been approved by an admin for execution.
    Approved,
    /// Script has been disabled / deprecated.
    Disabled,
}

impl ScriptStatus {
    /// Returns the stable storage and wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Approved => "approved",
            Self::Disabled => "disabled",
        }
    }
}

impl fmt::Display for ScriptStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ScriptStatus {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "approved" | "active" => Ok(Self::Approved),
            "disabled" | "deprecated" | "inactive" => Ok(Self::Disabled),
            _ => Err(ParseEnumError::new("script_status", value)),
        }
    }
}

/// WASM runtime chosen for high-isolation processor execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmRuntimeKind {
    /// Wasmtime runtime.
    Wasmtime,
    /// WasmEdge runtime.
    WasmEdge,
}

impl WasmRuntimeKind {
    /// Returns the stable wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wasmtime => "wasmtime",
            Self::WasmEdge => "wasmedge",
        }
    }
}

impl fmt::Display for WasmRuntimeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for WasmRuntimeKind {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "wasmtime" => Ok(Self::Wasmtime),
            "wasmedge" | "wasm_edge" | "wasm-edge" => Ok(Self::WasmEdge),
            _ => Err(ParseEnumError::new("wasm_runtime", value)),
        }
    }
}

/// Sandbox backend selected for dynamic script execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptSandboxBackend {
    /// Let the worker choose the safest available backend for the language and content.
    Auto,
    /// Execute through Wasmtime.
    Wasmtime,
    /// Execute through WasmEdge.
    WasmEdge,
    /// Execute through Anthropic Sandbox Runtime.
    Srt,
    /// Execute JavaScript or TypeScript through Deno.
    Deno,
    /// Execute JavaScript or TypeScript through a V8 isolate.
    V8,
    /// Execute through Docker.
    Docker,
    /// Execute through Podman.
    Podman,
    /// Execute through a worker-local custom backend.
    Custom,
}

impl Default for ScriptSandboxBackend {
    fn default() -> Self {
        Self::Auto
    }
}

impl ScriptSandboxBackend {
    /// Returns the stable wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Wasmtime => "wasmtime",
            Self::WasmEdge => "wasmedge",
            Self::Srt => "srt",
            Self::Deno => "deno",
            Self::V8 => "v8",
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::Custom => "custom",
        }
    }
}

impl fmt::Display for ScriptSandboxBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ScriptSandboxBackend {
    type Err = ParseEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "auto" => Ok(Self::Auto),
            "wasmtime" => Ok(Self::Wasmtime),
            "wasmedge" | "wasm_edge" | "wasm-edge" => Ok(Self::WasmEdge),
            "srt" | "anthropic_srt" | "anthropic-srt" | "sandbox_runtime" | "sandbox-runtime" => {
                Ok(Self::Srt)
            }
            "deno" => Ok(Self::Deno),
            "v8" | "v8_isolate" | "v8-isolate" => Ok(Self::V8),
            "docker" => Ok(Self::Docker),
            "podman" => Ok(Self::Podman),
            "custom" => Ok(Self::Custom),
            _ => Err(ParseEnumError::new("script_sandbox_backend", value)),
        }
    }
}

/// Sandbox backend policy for dynamic script execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptSandboxPolicy {
    /// Preferred backend. Auto lets worker choose according to language/content.
    #[serde(default)]
    pub backend: ScriptSandboxBackend,
}

/// Filesystem capability policy for dynamic script execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptFilesystemPolicy {
    /// Explicit read-only paths granted to the runner. Empty by default.
    pub read_only_paths: Vec<String>,
    /// Explicit writable paths granted to the runner. Empty by default.
    pub writable_paths: Vec<String>,
}

impl ScriptFilesystemPolicy {
    /// Returns true when any filesystem access has been requested.
    #[must_use]
    pub const fn grants_host_access(&self) -> bool {
        !self.read_only_paths.is_empty() || !self.writable_paths.is_empty()
    }
}

/// Network policy for dynamic script execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptNetworkPolicy {
    /// Whether outbound network access is requested. Default is denied.
    pub enabled: bool,
    /// Allowed hosts or URL policy references. Empty means no egress targets.
    pub allowed_hosts: Vec<String>,
}

/// Secret capability policy for dynamic script execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptSecretPolicy {
    /// Secret references that may be mounted or injected as ephemeral credentials.
    pub refs: Vec<String>,
}

/// URL/File/Secret grants supplied with a script release request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptReleaseGrantSet {
    /// URL hosts or URL policy references approved for this release.
    #[serde(default)]
    pub url: Vec<String>,
    /// Read-only file paths or file policy references approved for this release.
    #[serde(default)]
    pub file_read: Vec<String>,
    /// Writable file paths or file policy references approved for this release.
    #[serde(default)]
    pub file_write: Vec<String>,
    /// Secret references approved for this release.
    #[serde(default)]
    pub secret: Vec<String>,
}

impl ScriptReleaseGrantSet {
    /// Returns true when no grant category is populated.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.url.is_empty()
            && self.file_read.is_empty()
            && self.file_write.is_empty()
            && self.secret.is_empty()
    }

    /// Validate that a release grant set is well-formed and deny enabling it for now.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptReleaseGrantError`] when grant values are malformed or when
    /// any grant is requested before verified grant enforcement exists.
    pub fn validate_fail_closed(&self) -> Result<(), ScriptReleaseGrantError> {
        self.validate_values()?;
        if self.is_empty() {
            Ok(())
        } else {
            Err(ScriptReleaseGrantError::GrantVerificationUnavailable)
        }
    }

    /// Validate grant values without deciding whether they may be enforced.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptReleaseGrantError`] when grant values are malformed.
    pub fn validate_values(&self) -> Result<(), ScriptReleaseGrantError> {
        for value in self
            .url
            .iter()
            .chain(self.file_read.iter())
            .chain(self.file_write.iter())
            .chain(self.secret.iter())
        {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(ScriptReleaseGrantError::EmptyGrantValue);
            }
            if trimmed != value {
                return Err(ScriptReleaseGrantError::UntrimmedGrantValue);
            }
        }
        Ok(())
    }
}

/// Script release grant validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptReleaseGrantError {
    /// A grant value was empty or whitespace only.
    EmptyGrantValue,
    /// A grant value contained surrounding whitespace.
    UntrimmedGrantValue,
    /// Grant enforcement has not been wired to signature/KMS verification yet.
    GrantVerificationUnavailable,
}

impl fmt::Display for ScriptReleaseGrantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGrantValue => formatter.write_str("script release grants cannot be empty"),
            Self::UntrimmedGrantValue => {
                formatter.write_str("script release grants must not contain surrounding whitespace")
            }
            Self::GrantVerificationUnavailable => formatter.write_str(
                "script URL/File/Secret grants require verified release grant enforcement",
            ),
        }
    }
}

impl std::error::Error for ScriptReleaseGrantError {}

/// Resource policy shared by dynamic script runners.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptResourcePolicy {
    /// Maximum wall-clock runtime in milliseconds.
    pub timeout_ms: u64,
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Maximum output bytes captured by the runner.
    pub max_output_bytes: u64,
}

impl Default for ScriptResourcePolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            max_memory_bytes: 64 * 1024 * 1024,
            max_output_bytes: 1024 * 1024,
        }
    }
}

/// Stable policy snapshot for non-WASM dynamic script execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptExecutionPolicy {
    /// Resource limits enforced by the runner.
    pub resources: ScriptResourcePolicy,
    /// Outbound network policy. Default denies network.
    pub network: ScriptNetworkPolicy,
    /// Filesystem policy. Default grants no host paths.
    pub filesystem: ScriptFilesystemPolicy,
    /// Secret references. Default grants no secrets.
    pub secrets: ScriptSecretPolicy,
    /// Allowed environment variable names.
    pub env_vars: Vec<String>,
    /// Sandbox backend selection. Defaults to auto.
    #[serde(default)]
    pub sandbox: ScriptSandboxPolicy,
}

impl ScriptExecutionPolicy {
    /// Validate a script execution policy before storing or releasing it.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptPolicyError`] when resource limits are zero or a dangerous
    /// capability is requested without a later approval/signature gate.
    pub const fn validate_default_deny(&self) -> Result<(), ScriptPolicyError> {
        if self.resources.timeout_ms == 0 {
            return Err(ScriptPolicyError::ZeroTimeout);
        }
        if self.resources.max_memory_bytes == 0 {
            return Err(ScriptPolicyError::ZeroMemory);
        }
        if self.resources.max_output_bytes == 0 {
            return Err(ScriptPolicyError::ZeroOutput);
        }
        if self.network.enabled || !self.network.allowed_hosts.is_empty() {
            return Err(ScriptPolicyError::NetworkRequiresPolicyGrant);
        }
        if self.filesystem.grants_host_access() {
            return Err(ScriptPolicyError::FilesystemRequiresPolicyGrant);
        }
        if !self.secrets.refs.is_empty() {
            return Err(ScriptPolicyError::SecretsRequirePolicyGrant);
        }
        Ok(())
    }
}

/// Dynamic script policy validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptPolicyError {
    /// Timeout is zero.
    ZeroTimeout,
    /// Memory limit is zero.
    ZeroMemory,
    /// Output limit is zero.
    ZeroOutput,
    /// Network access needs a future URL policy and approval grant.
    NetworkRequiresPolicyGrant,
    /// Filesystem access needs a future filesystem policy and approval grant.
    FilesystemRequiresPolicyGrant,
    /// Secret access needs a future secret policy and approval grant.
    SecretsRequirePolicyGrant,
}

impl fmt::Display for ScriptPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTimeout => formatter.write_str("script timeout must be greater than zero"),
            Self::ZeroMemory => {
                formatter.write_str("script memory limit must be greater than zero")
            }
            Self::ZeroOutput => {
                formatter.write_str("script output limit must be greater than zero")
            }
            Self::NetworkRequiresPolicyGrant => formatter
                .write_str("script network access requires a future URL policy and approval grant"),
            Self::FilesystemRequiresPolicyGrant => formatter.write_str(
                "script filesystem access requires a future filesystem policy and approval grant",
            ),
            Self::SecretsRequirePolicyGrant => formatter.write_str(
                "script secret access requires a future secret policy and approval grant",
            ),
        }
    }
}

impl std::error::Error for ScriptPolicyError {}

/// Capability toggles for WASM/WASI processor execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmCapabilities {
    /// Whether outbound network access is requested. Phase 3 default is false and should remain denied unless a later URL policy grants it.
    pub network: bool,
    /// Preopened host directories. Empty by default to avoid ambient filesystem access.
    pub preopened_dirs: Vec<String>,
    /// Allowed environment variable names. Empty by default.
    pub env_vars: Vec<String>,
}

/// Resource policy for a WASM processor execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmResourcePolicy {
    /// Maximum wall-clock runtime in milliseconds.
    pub timeout_ms: u64,
    /// Maximum linear memory in bytes.
    pub max_memory_bytes: u64,
    /// Fuel/instruction budget for deterministic interruption.
    pub fuel: u64,
}

impl Default for WasmResourcePolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            max_memory_bytes: 64 * 1024 * 1024,
            fuel: 100_000_000,
        }
    }
}

/// Stable worker-side execution contract for a WASM processor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmProcessorSpec {
    /// Runtime implementation selected for the worker.
    pub runtime: WasmRuntimeKind,
    /// Exported function to call.
    pub entrypoint: String,
    /// Resource limits enforced by the worker runtime.
    pub resources: WasmResourcePolicy,
    /// Explicit capability grants. Defaults deny ambient host access.
    pub capabilities: WasmCapabilities,
}

impl Default for WasmProcessorSpec {
    fn default() -> Self {
        Self {
            runtime: WasmRuntimeKind::Wasmtime,
            entrypoint: "_start".to_owned(),
            resources: WasmResourcePolicy::default(),
            capabilities: WasmCapabilities::default(),
        }
    }
}

impl WasmProcessorSpec {
    /// Validate the processor spec before persisting or handing it to a worker.
    ///
    /// # Errors
    ///
    /// Returns [`WasmSpecError`] when required limits are zero, the entrypoint is empty,
    /// or the requested capabilities would grant ambient network/filesystem access that
    /// this phase deliberately denies.
    pub fn validate(&self) -> Result<(), WasmSpecError> {
        if self.entrypoint.trim().is_empty() {
            return Err(WasmSpecError::EmptyEntrypoint);
        }
        if self.resources.timeout_ms == 0 {
            return Err(WasmSpecError::ZeroTimeout);
        }
        if self.resources.max_memory_bytes == 0 {
            return Err(WasmSpecError::ZeroMemory);
        }
        if self.resources.fuel == 0 {
            return Err(WasmSpecError::ZeroFuel);
        }
        if self.capabilities.network {
            return Err(WasmSpecError::NetworkNotSupported);
        }
        if !self.capabilities.preopened_dirs.is_empty() {
            return Err(WasmSpecError::FilesystemNotSupported);
        }
        Ok(())
    }
}

/// WASM processor spec validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmSpecError {
    /// Entrypoint is empty.
    EmptyEntrypoint,
    /// Timeout is zero.
    ZeroTimeout,
    /// Memory limit is zero.
    ZeroMemory,
    /// Fuel budget is zero.
    ZeroFuel,
    /// Network access is not yet granted in Phase 3.
    NetworkNotSupported,
    /// Filesystem preopens are not yet granted in Phase 3.
    FilesystemNotSupported,
}

impl fmt::Display for WasmSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEntrypoint => formatter.write_str("wasm entrypoint must not be empty"),
            Self::ZeroTimeout => formatter.write_str("wasm timeout must be greater than zero"),
            Self::ZeroMemory => formatter.write_str("wasm memory limit must be greater than zero"),
            Self::ZeroFuel => formatter.write_str("wasm fuel budget must be greater than zero"),
            Self::NetworkNotSupported => {
                formatter.write_str("wasm network capability requires a future URL policy grant")
            }
            Self::FilesystemNotSupported => formatter
                .write_str("wasm filesystem preopens require a future filesystem policy grant"),
        }
    }
}

impl std::error::Error for WasmSpecError {}

/// Error returned when parsing a wire enum fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEnumError {
    field: &'static str,
    value: String,
}

impl ParseEnumError {
    #[must_use]
    fn new(field: &'static str, value: &str) -> Self {
        Self {
            field,
            value: value.to_owned(),
        }
    }
}

impl fmt::Display for ParseEnumError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid {}: {}", self.field, self.value)
    }
}

impl std::error::Error for ParseEnumError {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{
        ExecutionMode, HealthState, InstanceStatus, ScheduleType, ScriptExecutionPolicy,
        ScriptFilesystemPolicy, ScriptLanguage, ScriptNetworkPolicy, ScriptPolicyError,
        ScriptReleaseGrantError, ScriptReleaseGrantSet, ScriptSandboxBackend, ScriptSecretPolicy,
        ScriptStatus, TriggerType, WasmCapabilities, WasmProcessorSpec, WasmRuntimeKind,
        WasmSpecError,
    };

    #[test]
    fn health_state_wire_value_is_stable() {
        assert_eq!(HealthState::Ok.as_str(), "ok");
    }

    #[test]
    fn schedule_type_parses_aliases() {
        assert_eq!(
            ScheduleType::from_str("fixed-rate"),
            Ok(ScheduleType::FixedRate)
        );
        assert_eq!(ScheduleType::Cron.as_str(), "cron");
    }

    #[test]
    fn script_enums_parse_aliases() {
        assert_eq!(
            ScriptLanguage::from_str("python"),
            Ok(ScriptLanguage::Python)
        );
        assert_eq!(ScriptLanguage::from_str("js"), Ok(ScriptLanguage::Js));
        assert_eq!(
            ScriptLanguage::from_str("typescript"),
            Ok(ScriptLanguage::Ts)
        );
        assert_eq!(ScriptLanguage::Js.as_str(), "js");
        assert_eq!(ScriptLanguage::Ts.as_str(), "ts");
        assert_eq!(ScriptLanguage::Wasm.as_str(), "wasm");
        assert_eq!(
            ScriptStatus::from_str("approved"),
            Ok(ScriptStatus::Approved)
        );
        assert_eq!(ScriptStatus::from_str("active"), Ok(ScriptStatus::Approved));
        assert_eq!(ScriptStatus::Disabled.as_str(), "disabled");
    }

    #[test]
    fn script_sandbox_policy_defaults_to_auto_and_accepts_explicit_backends() {
        let policy = ScriptExecutionPolicy::default();
        assert_eq!(policy.sandbox.backend, ScriptSandboxBackend::Auto);
        assert_eq!(policy.sandbox.backend.as_str(), "auto");
        assert_eq!(
            ScriptSandboxBackend::from_str("wasmtime"),
            Ok(ScriptSandboxBackend::Wasmtime)
        );
        assert_eq!(
            ScriptSandboxBackend::from_str("wasmedge"),
            Ok(ScriptSandboxBackend::WasmEdge)
        );
        assert_eq!(
            ScriptSandboxBackend::from_str("srt"),
            Ok(ScriptSandboxBackend::Srt)
        );
        assert_eq!(
            ScriptSandboxBackend::from_str("v8"),
            Ok(ScriptSandboxBackend::V8)
        );
        assert_eq!(
            ScriptSandboxBackend::from_str("deno"),
            Ok(ScriptSandboxBackend::Deno)
        );
    }

    #[test]
    fn script_execution_policy_defaults_deny_dangerous_capabilities() {
        let policy = ScriptExecutionPolicy::default();
        assert!(policy.validate_default_deny().is_ok());
        assert_eq!(policy.resources.timeout_ms, 30_000);
        assert_eq!(policy.resources.max_memory_bytes, 64 * 1024 * 1024);
        assert_eq!(policy.resources.max_output_bytes, 1024 * 1024);
        assert!(!policy.network.enabled);
        assert!(policy.filesystem.read_only_paths.is_empty());
        assert!(policy.secrets.refs.is_empty());
        assert_eq!(policy.sandbox.backend, ScriptSandboxBackend::Auto);

        let with_network = ScriptExecutionPolicy {
            network: ScriptNetworkPolicy {
                enabled: true,
                allowed_hosts: vec!["example.com".to_owned()],
            },
            ..ScriptExecutionPolicy::default()
        };
        assert_eq!(
            with_network.validate_default_deny(),
            Err(ScriptPolicyError::NetworkRequiresPolicyGrant)
        );

        let with_filesystem = ScriptExecutionPolicy {
            filesystem: ScriptFilesystemPolicy {
                read_only_paths: vec!["/data/input".to_owned()],
                writable_paths: Vec::new(),
            },
            ..ScriptExecutionPolicy::default()
        };
        assert_eq!(
            with_filesystem.validate_default_deny(),
            Err(ScriptPolicyError::FilesystemRequiresPolicyGrant)
        );

        let with_secret = ScriptExecutionPolicy {
            secrets: ScriptSecretPolicy {
                refs: vec!["secret:db-readonly".to_owned()],
            },
            ..ScriptExecutionPolicy::default()
        };
        assert_eq!(
            with_secret.validate_default_deny(),
            Err(ScriptPolicyError::SecretsRequirePolicyGrant)
        );
    }

    #[test]
    fn script_release_grants_are_explicit_but_fail_closed_until_verified() {
        assert!(
            ScriptReleaseGrantSet::default()
                .validate_fail_closed()
                .is_ok()
        );

        let grants = ScriptReleaseGrantSet {
            url: vec!["https://api.example.com".to_owned()],
            file_read: vec!["/data/input".to_owned()],
            file_write: vec!["/data/output".to_owned()],
            secret: vec!["secret:db-readonly".to_owned()],
        };
        assert_eq!(
            grants.validate_fail_closed(),
            Err(ScriptReleaseGrantError::GrantVerificationUnavailable)
        );

        let malformed = ScriptReleaseGrantSet {
            url: vec![" example.com".to_owned()],
            ..ScriptReleaseGrantSet::default()
        };
        assert_eq!(
            malformed.validate_fail_closed(),
            Err(ScriptReleaseGrantError::UntrimmedGrantValue)
        );
    }

    #[test]
    fn wasm_runtime_and_processor_spec_are_stable() {
        assert_eq!(
            WasmRuntimeKind::from_str("wasmtime"),
            Ok(WasmRuntimeKind::Wasmtime)
        );
        assert_eq!(
            WasmRuntimeKind::from_str("wasmedge"),
            Ok(WasmRuntimeKind::WasmEdge)
        );
        let spec = WasmProcessorSpec::default();
        assert_eq!(spec.runtime.as_str(), "wasmtime");
        assert_eq!(spec.entrypoint, "_start");
        assert!(!spec.capabilities.network);
        assert!(spec.capabilities.preopened_dirs.is_empty());
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn wasm_processor_spec_denies_ambient_host_access() {
        let mut with_network = WasmProcessorSpec::default();
        with_network.capabilities.network = true;
        assert_eq!(
            with_network.validate(),
            Err(WasmSpecError::NetworkNotSupported)
        );

        let with_filesystem = WasmProcessorSpec {
            capabilities: WasmCapabilities {
                preopened_dirs: vec!["/tmp".to_owned()],
                ..WasmCapabilities::default()
            },
            ..WasmProcessorSpec::default()
        };
        assert_eq!(
            with_filesystem.validate(),
            Err(WasmSpecError::FilesystemNotSupported)
        );
    }

    #[test]
    fn wasm_processor_spec_serializes_as_worker_contract() {
        let value = match serde_json::to_value(WasmProcessorSpec::default()) {
            Ok(value) => value,
            Err(error) => panic!("serialize wasm spec: {error}"),
        };
        assert_eq!(value["runtime"], "wasmtime");
        assert_eq!(value["entrypoint"], "_start");
        assert_eq!(value["capabilities"]["network"], false);
        assert_eq!(value["resources"]["timeout_ms"], 30_000);
    }

    #[test]
    fn trigger_and_status_values_are_stable() {
        assert_eq!(TriggerType::Api.as_str(), "api");
        assert_eq!(TriggerType::WorkflowShard.as_str(), "workflow_shard");
        assert_eq!(InstanceStatus::Pending.as_str(), "pending");
        assert_eq!(
            InstanceStatus::from_str("partial_failed"),
            Ok(InstanceStatus::PartialFailed)
        );
        assert_eq!(
            ExecutionMode::from_str("broadcast"),
            Ok(ExecutionMode::Broadcast)
        );
    }
}
