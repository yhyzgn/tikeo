//! Core domain types shared by scheduler crates.

#![forbid(unsafe_code)]

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

#[cfg(test)]
mod tests {
    use super::HealthState;

    #[test]
    fn health_state_wire_value_is_stable() {
        assert_eq!(HealthState::Ok.as_str(), "ok");
    }
}
