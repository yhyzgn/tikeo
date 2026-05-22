//! Core domain types shared by scheduler crates.

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
    /// Job is triggered only through an explicit API call.
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
    /// Explicit management API trigger.
    Api,
    /// CRON scheduler trigger.
    Cron,
    /// Fixed-rate scheduler trigger.
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
    /// Node.js / JavaScript / TypeScript script.
    Node,
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
            Self::Node => "node",
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
            "node" | "nodejs" | "javascript" | "js" | "typescript" | "ts" => Ok(Self::Node),
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
}

impl WasmRuntimeKind {
    /// Returns the stable wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wasmtime => "wasmtime",
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
            _ => Err(ParseEnumError::new("wasm_runtime", value)),
        }
    }
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
        ScriptSecretPolicy, ScriptStatus, TriggerType, WasmCapabilities, WasmProcessorSpec,
        WasmRuntimeKind, WasmSpecError,
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
        assert_eq!(ScriptLanguage::from_str("js"), Ok(ScriptLanguage::Node));
        assert_eq!(ScriptLanguage::Wasm.as_str(), "wasm");
        assert_eq!(
            ScriptStatus::from_str("approved"),
            Ok(ScriptStatus::Approved)
        );
        assert_eq!(ScriptStatus::from_str("active"), Ok(ScriptStatus::Approved));
        assert_eq!(ScriptStatus::Disabled.as_str(), "disabled");
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
    fn wasm_runtime_and_processor_spec_are_stable() {
        assert_eq!(
            WasmRuntimeKind::from_str("wasmtime"),
            Ok(WasmRuntimeKind::Wasmtime)
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
