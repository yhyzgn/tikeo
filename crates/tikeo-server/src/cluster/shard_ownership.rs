//! Deterministic shard ownership decisions for future Raft-applied multi-active scheduling.
//!
//! This module is deliberately pure: it does not start multi-active scheduling by itself and it
//! does not rely on Redis, Dragonfly, SQL advisory locks, or any other external lock manager.
//! Runtime use must be fed by a Raft-applied assignment map with monotonically increasing epochs.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

/// One deterministic scheduler shard identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchedulerShardId(u32);

impl SchedulerShardId {
    /// Build a shard id if it is inside `0..shard_count`.
    #[must_use]
    /// New.
    pub const fn new(value: u32, shard_count: u32) -> Option<Self> {
        if shard_count > 0 && value < shard_count {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Raw shard id value.
    #[must_use]
    /// Value.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Durable scope used to map schedule/dispatch work to a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerShardKey {
    namespace: String,
    app: String,
    job_or_workflow_id: String,
}

impl SchedulerShardKey {
    /// Construct a key from durable task scope.
    #[must_use]
    /// New.
    pub fn new(
        namespace: impl Into<String>,
        app: impl Into<String>,
        job_or_workflow_id: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            app: app.into(),
            job_or_workflow_id: job_or_workflow_id.into(),
        }
    }

    /// Return the shard for this key.
    ///
    /// # Panics
    ///
    /// Panics when `shard_count` is zero. A zero-shard cluster is invalid configuration and must be
    /// rejected before a Raft assignment is applied.
    #[must_use]
    /// Shard id.
    pub fn shard_id(&self, shard_count: u32) -> SchedulerShardId {
        assert!(
            shard_count > 0,
            "scheduler shard_count must be greater than zero"
        );
        let mut hasher = Sha256::new();
        hasher.update(self.namespace.as_bytes());
        hasher.update([0]);
        hasher.update(self.app.as_bytes());
        hasher.update([0]);
        hasher.update(self.job_or_workflow_id.as_bytes());
        let digest = hasher.finalize();
        let mut prefix = [0_u8; 8];
        prefix.copy_from_slice(&digest[..8]);
        let value = u64::from_be_bytes(prefix) % u64::from(shard_count);
        SchedulerShardId(u32::try_from(value).unwrap_or(0))
    }
}

/// Raft-applied shard assignment command payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardAssignmentCommand {
    /// Monotonic shard map version used for hashing.
    pub shard_map_version: u64,
    /// Monotonic Raft-applied assignment epoch.
    pub epoch: u64,
    /// Number of scheduler shards in this assignment epoch.
    pub shard_count: u32,
    /// Owner node for each shard.
    pub owners: BTreeMap<SchedulerShardId, String>,
}

impl ShardAssignmentCommand {
    /// Build a balanced assignment for the provided owner set.
    ///
    /// # Panics
    ///
    /// Panics when `epoch` is zero, `shard_count` is zero, or `owner_node_ids` is empty. These are
    /// invalid Raft command inputs and callers must validate operator/API input first.
    #[must_use]
    /// Balanced.
    pub fn balanced(epoch: u64, shard_count: u32, owner_node_ids: &[impl AsRef<str>]) -> Self {
        Self::balanced_with_map_version(1, epoch, shard_count, owner_node_ids)
    }

    /// Build a balanced assignment for an explicit shard map version.
    ///
    /// # Panics
    ///
    /// Panics when `shard_map_version` or `epoch` is zero, `shard_count` is zero, or
    /// `owner_node_ids` is empty. These are invalid Raft command inputs and callers must validate
    /// operator/API input first.
    #[must_use]
    /// Balanced with map version.
    pub fn balanced_with_map_version(
        shard_map_version: u64,
        epoch: u64,
        shard_count: u32,
        owner_node_ids: &[impl AsRef<str>],
    ) -> Self {
        assert!(
            shard_map_version > 0,
            "shard assignment map version must be greater than zero"
        );
        assert!(
            epoch > 0,
            "shard assignment epoch must be greater than zero"
        );
        assert!(
            shard_count > 0,
            "shard assignment shard_count must be greater than zero"
        );
        assert!(
            !owner_node_ids.is_empty(),
            "shard assignment requires at least one owner node"
        );
        let mut owners = BTreeMap::new();
        for shard in 0..shard_count {
            let owner = owner_node_ids[(shard as usize) % owner_node_ids.len()]
                .as_ref()
                .to_owned();
            owners.insert(SchedulerShardId(shard), owner);
        }
        Self {
            shard_map_version,
            epoch,
            shard_count,
            owners,
        }
    }

    /// Return the owner node id for a shard.
    #[must_use]
    /// Owner for.
    pub fn owner_for(&self, shard: SchedulerShardId) -> Option<&str> {
        self.owners.get(&shard).map(String::as_str)
    }

    /// Produce the fencing token a node must present while claiming work for a shard.
    #[must_use]
    /// Fencing token for.
    pub fn fencing_token_for(&self, shard: SchedulerShardId, node_id: &str) -> Option<String> {
        (self.owner_for(shard) == Some(node_id)).then(|| {
            format!(
                "raft-shard:v:{}:count:{}:epoch:{}:shard:{}:node:{}",
                self.shard_map_version,
                self.shard_count,
                self.epoch,
                shard.value(),
                node_id
            )
        })
    }
}

/// Local decision derived from the latest Raft-applied assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardOwnershipDecision {
    /// Shard evaluated for the current work item.
    pub shard: SchedulerShardId,
    /// Whether the local node owns this shard.
    pub owned: bool,
    /// Current epoch-scoped fencing token when owned.
    pub fencing_token: Option<String>,
}

/// Decide whether `local_node_id` may schedule/dispatch work for `key`.
#[must_use]
/// Decide ownership.
pub fn decide_ownership(
    assignment: &ShardAssignmentCommand,
    key: &SchedulerShardKey,
    local_node_id: &str,
) -> ShardOwnershipDecision {
    let shard = key.shard_id(assignment.shard_count);
    let fencing_token = assignment.fencing_token_for(shard, local_node_id);
    ShardOwnershipDecision {
        shard,
        owned: fencing_token.is_some(),
        fencing_token,
    }
}

/// Validate that a previously issued fencing token is still current for this assignment.
#[must_use]
/// Accepts fencing token.
pub fn accepts_fencing_token(
    assignment: &ShardAssignmentCommand,
    shard: SchedulerShardId,
    node_id: &str,
    token: &str,
) -> bool {
    assignment
        .fencing_token_for(shard, node_id)
        .is_some_and(|current| current == token)
}

#[cfg(test)]
mod tests {
    use super::{
        SchedulerShardKey, ShardAssignmentCommand, accepts_fencing_token, decide_ownership,
    };

    #[test]
    fn shard_key_maps_to_a_stable_bounded_shard() {
        let key = SchedulerShardKey::new("tenant-a", "billing", "job-42");

        let first = key.shard_id(16);
        let second = key.shard_id(16);

        assert_eq!(first, second);
        assert!(first.value() < 16);
    }

    #[test]
    fn balanced_assignment_spreads_shards_without_external_locks() {
        let assignment = ShardAssignmentCommand::balanced(7, 6, &["tikeo-0", "tikeo-1"]);

        let owners = (0..6)
            .map(|shard| {
                let shard = super::SchedulerShardId::new(shard, 6)
                    .unwrap_or_else(|| panic!("valid shard {shard}"));
                assignment
                    .owner_for(shard)
                    .unwrap_or_else(|| panic!("shard {} should have owner", shard.value()))
                    .to_owned()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            owners,
            [
                "tikeo-0", "tikeo-1", "tikeo-0", "tikeo-1", "tikeo-0", "tikeo-1"
            ]
        );
    }

    #[test]
    fn only_assigned_owner_receives_epoch_scoped_fencing_token() {
        let assignment =
            ShardAssignmentCommand::balanced_with_map_version(5, 3, 2, &["tikeo-0", "tikeo-1"]);
        let key = SchedulerShardKey::new("tenant-a", "orders", "job-east");
        let shard = key.shard_id(2);
        let owner = assignment
            .owner_for(shard)
            .unwrap_or_else(|| panic!("owner exists"));
        let non_owner = if owner == "tikeo-0" {
            "tikeo-1"
        } else {
            "tikeo-0"
        };

        let owner_decision = decide_ownership(&assignment, &key, owner);
        let non_owner_decision = decide_ownership(&assignment, &key, non_owner);

        assert!(owner_decision.owned);
        assert_eq!(owner_decision.shard, shard);
        assert_eq!(
            owner_decision.fencing_token.as_deref(),
            Some(
                format!(
                    "raft-shard:v:5:count:2:epoch:3:shard:{}:node:{owner}",
                    shard.value()
                )
                .as_str()
            )
        );
        assert!(!non_owner_decision.owned);
        assert_eq!(non_owner_decision.fencing_token, None);
    }

    #[test]
    fn failover_epoch_rejects_stale_fencing_token() {
        let shard = SchedulerShardKey::new("tenant-a", "orders", "job-1").shard_id(1);
        let before_failover = ShardAssignmentCommand::balanced(1, 1, &["tikeo-0"]);
        let stale_token = before_failover
            .fencing_token_for(shard, "tikeo-0")
            .unwrap_or_else(|| panic!("initial owner token exists"));
        let after_failover = ShardAssignmentCommand::balanced(2, 1, &["tikeo-1"]);
        let new_token = after_failover
            .fencing_token_for(shard, "tikeo-1")
            .unwrap_or_else(|| panic!("new owner token exists"));

        assert!(!accepts_fencing_token(
            &after_failover,
            shard,
            "tikeo-0",
            &stale_token
        ));
        assert!(accepts_fencing_token(
            &after_failover,
            shard,
            "tikeo-1",
            &new_token
        ));
    }
}
