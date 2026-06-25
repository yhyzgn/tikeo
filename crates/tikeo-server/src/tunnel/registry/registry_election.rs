use std::{collections::HashMap, time::SystemTime};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tikeo_proto::worker::v1::{RegisterWorker, WorkerClusterElection};

use super::RegisteredWorker;

/// Worker-side master election outcome for a namespace/app/cluster/region domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerMasterState {
    /// Deterministic election domain.
    pub domain: String,
    /// Whether this worker is the elected master for the domain.
    pub is_master: bool,
    /// Current elected master worker id, when one exists.
    pub master_worker_id: Option<String>,
    /// Monotonic-ish term derived from domain membership generations.
    pub term: u64,
    /// Fencing token bound to domain, term, and elected master.
    pub fencing_token: Option<String>,
}

impl WorkerMasterState {
    /// Follower.
    pub(super) const fn follower(domain: String) -> Self {
        Self {
            domain,
            is_master: false,
            master_worker_id: None,
            term: 0,
            fencing_token: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkerElectionRegistration {
    /// Boolean state flag.
    pub(super) enabled: bool,
    pub(super) domain: String,
    pub(super) priority: u32,
}

/// Worker election registration.
pub(super) fn worker_election_registration(worker: &RegisterWorker) -> WorkerElectionRegistration {
    let election = worker.election.clone().unwrap_or_default();
    WorkerElectionRegistration {
        enabled: election.enabled,
        domain: normalized_election_domain(worker, &election),
        priority: election.priority,
    }
}

fn normalized_election_domain(worker: &RegisterWorker, election: &WorkerClusterElection) -> String {
    let configured = election.domain.trim();
    if configured.is_empty() {
        worker_domain(
            &worker.namespace,
            &worker.app,
            &worker.cluster,
            &worker.region,
        )
    } else {
        configured.to_owned()
    }
}

fn worker_domain(namespace: &str, app: &str, cluster: &str, region: &str) -> String {
    format!("{namespace}/{app}/{cluster}/{region}")
}

/// Recompute worker master states.
pub(super) fn recompute_worker_master_states(workers: &mut HashMap<String, RegisteredWorker>) {
    let now = SystemTime::now();
    let mut winners = HashMap::<String, (String, u64, String)>::new();
    for worker in workers.values() {
        if !worker.election.enabled || !worker.is_current() || worker.lease_expires_at <= now {
            continue;
        }
        let term = workers
            .values()
            .filter(|candidate| candidate.election.domain == worker.election.domain)
            .map(|candidate| candidate.generation)
            .max()
            .unwrap_or(worker.generation);
        let candidate = (
            worker.worker_id.clone(),
            term,
            worker_master_fencing_token(&worker.election.domain, term, &worker.worker_id),
        );
        let replace = winners
            .get(&worker.election.domain)
            .is_none_or(|(winner_id, _, _)| {
                let winner = workers.get(winner_id);
                winner.is_none_or(|winner| {
                    worker.election.priority < winner.election.priority
                        || (worker.election.priority == winner.election.priority
                            && worker.worker_id < winner.worker_id)
                })
            });
        if replace {
            winners.insert(worker.election.domain.clone(), candidate);
        }
    }

    for worker in workers.values_mut() {
        if !worker.election.enabled {
            worker.master = WorkerMasterState::follower(worker.election.domain.clone());
            continue;
        }
        if let Some((master_worker_id, term, fencing_token)) = winners.get(&worker.election.domain)
        {
            worker.master = WorkerMasterState {
                domain: worker.election.domain.clone(),
                is_master: master_worker_id == &worker.worker_id,
                master_worker_id: Some(master_worker_id.clone()),
                term: *term,
                fencing_token: Some(fencing_token.clone()),
            };
        } else {
            worker.master = WorkerMasterState::follower(worker.election.domain.clone());
        }
    }
}

fn worker_master_fencing_token(domain: &str, term: u64, worker_id: &str) -> String {
    let digest = Sha256::digest(format!("{domain}:{term}:{worker_id}").as_bytes());
    format!("wmf-{term}-{}", hex_prefix(&digest, 16))
}

fn hex_prefix(bytes: &[u8], len: usize) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .take(len)
        .map(|nibble| char::from_digit(u32::from(nibble), 16).unwrap_or('0'))
        .collect()
}
