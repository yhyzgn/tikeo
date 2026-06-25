//! Scheduler shard map policy shared by storage repositories.

use std::sync::OnceLock;

use sha2::{Digest, Sha256};

const DEFAULT_SHARD_MAP_VERSION: i64 = 1;
const DEFAULT_SHARD_COUNT: i32 = 64;
const MAX_SHARD_COUNT: i32 = 32_768;

static SHARD_POLICY: OnceLock<std::sync::RwLock<SchedulerShardPolicy>> = OnceLock::new();

/// Durable scheduler shard map policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerShardPolicy {
    /// Monotonic shard map version. Version changes define a new hash ring.
    pub shard_map_version: i64,
    /// Total number of logical scheduler shards in this version.
    pub shard_count: i32,
}

impl Default for SchedulerShardPolicy {
    fn default() -> Self {
        Self {
            shard_map_version: DEFAULT_SHARD_MAP_VERSION,
            shard_count: DEFAULT_SHARD_COUNT,
        }
    }
}

impl SchedulerShardPolicy {
    /// Build a validated policy.
    ///
    /// # Errors
    ///
    /// Returns an error when version/count are outside supported bounds.
    pub fn new(shard_map_version: i64, shard_count: i32) -> Result<Self, String> {
        if shard_map_version <= 0 {
            return Err("scheduler shard_map_version must be greater than zero".to_owned());
        }
        if !(1..=MAX_SHARD_COUNT).contains(&shard_count) {
            return Err(format!(
                "scheduler shard_count must be between 1 and {MAX_SHARD_COUNT}"
            ));
        }
        Ok(Self {
            shard_map_version,
            shard_count,
        })
    }

    /// Compute a stable shard id for one durable work key.
    #[must_use]
    /// Shard id for.
    pub fn shard_id_for(self, namespace: &str, app: &str, durable_id: &str) -> i32 {
        stable_scheduler_shard_id(namespace, app, durable_id, self.shard_count)
    }
}

/// Return the process-wide scheduler shard policy.
#[must_use]
/// Scheduler shard policy.
pub fn scheduler_shard_policy() -> SchedulerShardPolicy {
    *SHARD_POLICY
        .get_or_init(|| std::sync::RwLock::new(SchedulerShardPolicy::default()))
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Set the process-wide scheduler shard policy.
///
/// # Errors
///
/// Returns an error when version/count are invalid.
pub fn set_scheduler_shard_policy(
    shard_map_version: i64,
    shard_count: i32,
) -> Result<SchedulerShardPolicy, String> {
    let policy = SchedulerShardPolicy::new(shard_map_version, shard_count)?;
    let lock = SHARD_POLICY.get_or_init(|| std::sync::RwLock::new(SchedulerShardPolicy::default()));
    *lock
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = policy;
    Ok(policy)
}

/// Reset process-wide scheduler shard policy to defaults. Intended for tests.
#[doc(hidden)]
/// Reset scheduler shard policy for test.
pub fn reset_scheduler_shard_policy_for_test() {
    let lock = SHARD_POLICY.get_or_init(|| std::sync::RwLock::new(SchedulerShardPolicy::default()));
    *lock
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = SchedulerShardPolicy::default();
}

fn stable_scheduler_shard_id(
    namespace: &str,
    app: &str,
    durable_id: &str,
    shard_count: i32,
) -> i32 {
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    hasher.update([0]);
    hasher.update(app.as_bytes());
    hasher.update([0]);
    hasher.update(durable_id.as_bytes());
    let digest = hasher.finalize();
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    i32::try_from(u64::from_be_bytes(prefix) % u64::try_from(shard_count.max(1)).unwrap_or(1))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::SchedulerShardPolicy;

    #[test]
    fn shard_policy_rejects_invalid_values() {
        assert!(SchedulerShardPolicy::new(0, 64).is_err());
        assert!(SchedulerShardPolicy::new(1, 0).is_err());
        assert!(SchedulerShardPolicy::new(1, 32_769).is_err());
    }

    #[test]
    fn shard_policy_maps_stably_with_configured_count() {
        let policy =
            SchedulerShardPolicy::new(3, 8).unwrap_or_else(|error| panic!("valid: {error}"));
        let first = policy.shard_id_for("default", "billing", "job-1");
        let second = policy.shard_id_for("default", "billing", "job-1");
        assert_eq!(first, second);
        assert!((0..8).contains(&first));
    }
}
