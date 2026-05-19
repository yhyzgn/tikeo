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

    use super::{ExecutionMode, HealthState, InstanceStatus, ScheduleType, TriggerType};

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
    fn trigger_and_status_values_are_stable() {
        assert_eq!(TriggerType::Api.as_str(), "api");
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
