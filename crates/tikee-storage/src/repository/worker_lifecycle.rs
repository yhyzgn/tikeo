use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect, Set, sea_query::OnConflict,
};
use sha2::{Digest, Sha256};

use crate::entities::{worker_logical_instance, worker_session, worker_session_event};

use super::util::{new_id, now_rfc3339, rfc3339_after_seconds};

const STATUS_ACTIVE: &str = "active";
const STATUS_ONLINE: &str = "online";
const STATUS_REPLACED: &str = "replaced";
const STATUS_OFFLINE: &str = "offline";
const STATUS_STOPPED: &str = "stopped";
const STATUS_INACTIVE: &str = "inactive";
const STATUS_DEGRADED: &str = "degraded";
const REASON_REPLACED: &str = "replaced_by_new_generation";
const REASON_LEASE_EXPIRED_UNKNOWN: &str = "lease_expired_unknown";
const REASON_GRACEFUL_SHUTDOWN: &str = "graceful_shutdown";
const REASON_TRANSPORT_ERROR: &str = "transport_error";

/// Input for creating a new ephemeral worker session under a logical worker instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterWorkerSession {
    /// Server-assigned worker session id.
    pub worker_id: String,
    /// Namespace scope name.
    pub namespace_name: String,
    /// Application scope name.
    pub app_name: String,
    /// Deployment cluster.
    pub cluster: String,
    /// Deployment region.
    pub region: String,
    /// Client stable instance hint.
    pub client_instance_id: String,
    /// Server-local connection id.
    pub connection_id: String,
    /// Plain fencing token; only its hash is persisted.
    pub fencing_token: String,
    /// Lease duration from now.
    pub lease_seconds: i64,
    /// Legacy free-form capabilities snapshot as JSON array.
    pub capabilities_json: String,
    /// Structured capabilities snapshot as JSON object.
    pub structured_capabilities_json: String,
    /// Worker label snapshot as JSON object.
    pub labels_json: String,
    /// Worker master/election state snapshot as JSON object.
    pub master_json: String,
}

/// Input for accepting a fenced worker heartbeat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerHeartbeat {
    /// Worker session id.
    pub worker_id: String,
    /// Worker generation claimed by the client.
    pub generation: i64,
    /// Plain fencing token assigned at registration.
    pub fencing_token: String,
    /// Monotonic heartbeat sequence.
    pub sequence: i64,
    /// Lease duration from now.
    pub lease_seconds: i64,
}

/// Worker session summary used by repositories and runtime recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerSessionSummary {
    /// Worker session id.
    pub worker_id: String,
    /// Logical instance id.
    pub logical_instance_id: String,
    /// Current logical worker id after registration/heartbeat.
    pub current_worker_id: Option<String>,
    /// Session generation.
    pub generation: i64,
    /// Session status.
    pub status: String,
    /// Optional status reason.
    pub status_reason: Option<String>,
    /// Optional status evidence.
    pub status_evidence: Option<String>,
    /// Lease expiry timestamp.
    pub lease_expires_at: String,
    /// Last heartbeat timestamp.
    pub last_heartbeat_at: String,
    /// Last heartbeat sequence.
    pub last_sequence: i64,
    /// Legacy free-form capabilities snapshot as JSON array.
    pub capabilities_json: String,
    /// Structured capabilities snapshot as JSON object.
    pub structured_capabilities_json: String,
    /// Worker label snapshot as JSON object.
    pub labels_json: String,
    /// Worker master/election state snapshot as JSON object.
    pub master_json: String,
    /// Replacement session id.
    pub replaced_by_worker_id: Option<String>,
}

/// Persisted online worker view reconstructed from durable lifecycle tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedOnlineWorkerSummary {
    /// Worker session id.
    pub worker_id: String,
    /// Logical instance id.
    pub logical_instance_id: String,
    /// Client-provided stable instance hint.
    pub client_instance_id: Option<String>,
    /// Namespace scope.
    pub namespace_name: String,
    /// Application scope.
    pub app_name: String,
    /// Cluster name.
    pub cluster: String,
    /// Region name.
    pub region: String,
    /// Generation.
    pub generation: i64,
    /// Status.
    pub status: String,
    /// Optional status reason.
    pub status_reason: Option<String>,
    /// Lease expiry timestamp.
    pub lease_expires_at: String,
    /// Last heartbeat sequence.
    pub last_sequence: i64,
    /// Legacy free-form capabilities snapshot as JSON array.
    pub capabilities_json: String,
    /// Structured capabilities snapshot as JSON object.
    pub structured_capabilities_json: String,
    /// Worker label snapshot as JSON object.
    pub labels_json: String,
    /// Worker master/election state snapshot as JSON object.
    pub master_json: String,
    /// Replacement session id.
    pub replaced_by_worker_id: Option<String>,
}

/// Snapshot update for persisted worker presentation fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerSessionSnapshotUpdate {
    /// Worker session id.
    pub worker_id: String,
    /// Legacy free-form capabilities snapshot as JSON array.
    pub capabilities_json: String,
    /// Structured capabilities snapshot as JSON object.
    pub structured_capabilities_json: String,
    /// Worker label snapshot as JSON object.
    pub labels_json: String,
    /// Worker master/election state snapshot as JSON object.
    pub master_json: String,
}

/// Worker lifecycle event summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerSessionEventSummary {
    /// Event id.
    pub id: String,
    /// Worker session id.
    pub worker_id: String,
    /// Logical instance id.
    pub logical_instance_id: String,
    /// Event type.
    pub event_type: String,
    /// Optional reason code.
    pub reason: Option<String>,
    /// Optional JSON detail.
    pub detail_json: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
}

/// Repository for persistent worker logical instances, sessions, and events.
#[derive(Debug, Clone)]
pub struct WorkerLifecycleRepository {
    db: DatabaseConnection,
}

impl WorkerLifecycleRepository {
    /// Create a repository using the provided database connection.
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Register a fresh session, replacing active older generations for the same logical key.
    pub async fn register_session(
        &self,
        input: RegisterWorkerSession,
    ) -> Result<WorkerSessionSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let logical = self.upsert_logical_instance(&input, &now).await?;
        let generation = logical.current_generation.saturating_add(1);
        self.replace_current_session(&logical, &input.worker_id, &now)
            .await?;
        let session = self
            .upsert_session(&input, &logical.id, generation, &now)
            .await?;
        self.promote_logical_current(logical, &input.worker_id, generation, &now)
            .await?;
        self.record_event(
            &input.worker_id,
            &session.logical_instance_id,
            "session_registered",
            None,
            None,
            &now,
        )
        .await?;
        Ok(WorkerSessionSummary::from_model(
            session,
            Some(input.worker_id),
        ))
    }

    /// Renew a session lease only when generation and fencing token still match the current session.
    pub async fn heartbeat(
        &self,
        input: WorkerHeartbeat,
    ) -> Result<Option<WorkerSessionSummary>, sea_orm::DbErr> {
        let Some(session) = self.get_session_model(&input.worker_id).await? else {
            return Ok(None);
        };
        if !session_accepts_heartbeat(&session, &input) {
            self.record_event(
                &session.worker_id,
                &session.logical_instance_id,
                "stale_worker_message",
                Some("heartbeat_fenced"),
                None,
                &now_rfc3339(),
            )
            .await?;
            return Ok(None);
        }
        let Some(logical) =
            worker_logical_instance::Entity::find_by_id(&session.logical_instance_id)
                .one(&self.db)
                .await?
        else {
            return Ok(None);
        };
        if logical.current_worker_id.as_deref() != Some(session.worker_id.as_str())
            || logical.current_generation != session.generation
        {
            return Ok(None);
        }

        let now = now_rfc3339();
        let mut active = session.into_active_model();
        active.last_heartbeat_at = Set(now.clone());
        active.lease_expires_at = Set(rfc3339_after_seconds(input.lease_seconds));
        active.last_sequence = Set(input.sequence);
        active.updated_at = Set(now.clone());
        let updated = active.update(&self.db).await?;

        let mut logical_active = logical.into_active_model();
        logical_active.last_seen_at = Set(now.clone());
        logical_active.updated_at = Set(now);
        logical_active.update(&self.db).await?;

        Ok(Some(WorkerSessionSummary::from_model(
            updated,
            Some(input.worker_id),
        )))
    }

    /// Mark a current fenced session as gracefully stopped.
    pub async fn graceful_unregister(
        &self,
        worker_id: &str,
        generation: i64,
        fencing_token: &str,
    ) -> Result<Option<WorkerSessionSummary>, sea_orm::DbErr> {
        let Some(session) = self.get_session_model(worker_id).await? else {
            return Ok(None);
        };
        let heartbeat = WorkerHeartbeat {
            worker_id: worker_id.to_owned(),
            generation,
            fencing_token: fencing_token.to_owned(),
            sequence: session.last_sequence,
            lease_seconds: 0,
        };
        if !session_accepts_heartbeat(&session, &heartbeat) {
            self.record_event(
                &session.worker_id,
                &session.logical_instance_id,
                "stale_worker_message",
                Some("unregister_fenced"),
                None,
                &now_rfc3339(),
            )
            .await?;
            return Ok(None);
        }
        let Some(logical) =
            worker_logical_instance::Entity::find_by_id(&session.logical_instance_id)
                .one(&self.db)
                .await?
        else {
            return Ok(None);
        };
        if logical.current_worker_id.as_deref() != Some(session.worker_id.as_str())
            || logical.current_generation != session.generation
        {
            return Ok(None);
        }

        let now = now_rfc3339();
        let logical_instance_id = session.logical_instance_id.clone();
        let updated = self.mark_session_gracefully_stopped(session, &now).await?;
        self.mark_logical_inactive_if_current(&logical_instance_id, worker_id, &now)
            .await?;
        self.record_event(
            worker_id,
            &logical_instance_id,
            "graceful_shutdown",
            Some(REASON_GRACEFUL_SHUTDOWN),
            None,
            &now,
        )
        .await?;
        Ok(Some(WorkerSessionSummary::from_model(updated, None)))
    }

    /// Mark a server-observed stream close/error as offline with high-confidence transport evidence.
    pub async fn mark_transport_error(
        &self,
        worker_id: &str,
        evidence: &str,
    ) -> Result<Option<WorkerSessionSummary>, sea_orm::DbErr> {
        let Some(session) = self.get_session_model(worker_id).await? else {
            return Ok(None);
        };
        if session.status != STATUS_ONLINE {
            return Ok(None);
        }
        let now = now_rfc3339();
        let logical_instance_id = session.logical_instance_id.clone();
        let updated = self
            .mark_session_transport_error(session, evidence, &now)
            .await?;
        self.mark_logical_degraded_if_current(&logical_instance_id, worker_id, &now)
            .await?;
        self.record_event(
            worker_id,
            &logical_instance_id,
            "transport_error",
            Some(REASON_TRANSPORT_ERROR),
            Some(&format!(
                "{{\"evidence\":{}}}",
                serde_json::to_string(evidence).unwrap_or_else(|_| "\"stream closed\"".to_owned())
            )),
            &now,
        )
        .await?;
        Ok(Some(WorkerSessionSummary::from_model(updated, None)))
    }

    /// Mark expired online sessions as offline with evidence-limited unknown lease expiry reason.
    pub async fn mark_expired_online_sessions(
        &self,
        limit: u64,
    ) -> Result<Vec<String>, sea_orm::DbErr> {
        let now = now_rfc3339();
        let sessions = worker_session::Entity::find()
            .filter(worker_session::Column::Status.eq(STATUS_ONLINE.to_owned()))
            .filter(worker_session::Column::LeaseExpiresAt.lt(now.clone()))
            .order_by_asc(worker_session::Column::LeaseExpiresAt)
            .limit(limit)
            .all(&self.db)
            .await?;

        let mut expired_worker_ids = Vec::with_capacity(sessions.len());
        for session in sessions {
            let worker_id = session.worker_id.clone();
            let logical_instance_id = session.logical_instance_id.clone();
            self.mark_session_lease_expired(session, &now).await?;
            self.mark_logical_degraded_if_current(&logical_instance_id, &worker_id, &now)
                .await?;
            self.record_event(
                &worker_id,
                &logical_instance_id,
                "lease_expired",
                Some(REASON_LEASE_EXPIRED_UNKNOWN),
                None,
                &now,
            )
            .await?;
            expired_worker_ids.push(worker_id);
        }
        Ok(expired_worker_ids)
    }

    /// Load a persisted session by worker id.
    pub async fn get_session(
        &self,
        worker_id: &str,
    ) -> Result<Option<WorkerSessionSummary>, sea_orm::DbErr> {
        Ok(self
            .get_session_model(worker_id)
            .await?
            .map(|session| WorkerSessionSummary::from_model(session, None)))
    }

    /// List persisted online sessions whose lease has not expired, joined with logical instance metadata.
    pub async fn list_online_workers(
        &self,
        limit: u64,
    ) -> Result<Vec<PersistedOnlineWorkerSummary>, sea_orm::DbErr> {
        let now = now_rfc3339();
        let sessions = worker_session::Entity::find()
            .filter(worker_session::Column::Status.eq(STATUS_ONLINE.to_owned()))
            .filter(worker_session::Column::LeaseExpiresAt.gt(now))
            .order_by_desc(worker_session::Column::UpdatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;

        let mut workers = Vec::with_capacity(sessions.len());
        for session in sessions {
            let Some(logical) =
                worker_logical_instance::Entity::find_by_id(&session.logical_instance_id)
                    .one(&self.db)
                    .await?
            else {
                continue;
            };
            if logical.current_worker_id.as_deref() != Some(session.worker_id.as_str())
                || logical.current_generation != session.generation
            {
                continue;
            }
            workers.push(PersistedOnlineWorkerSummary {
                worker_id: session.worker_id,
                logical_instance_id: session.logical_instance_id,
                client_instance_id: non_empty(logical.client_instance_id),
                namespace_name: logical.namespace_name,
                app_name: logical.app_name,
                cluster: logical.cluster,
                region: logical.region,
                generation: session.generation,
                status: session.status,
                status_reason: session.status_reason,
                lease_expires_at: session.lease_expires_at,
                last_sequence: session.last_sequence,
                capabilities_json: session.capabilities_json,
                structured_capabilities_json: session.structured_capabilities_json,
                labels_json: session.labels_json,
                master_json: session.master_json,
                replaced_by_worker_id: session.replaced_by_worker_id,
            });
        }
        Ok(workers)
    }

    /// Update persisted worker capability/label/master snapshots for currently known online sessions.
    pub async fn update_session_snapshots(
        &self,
        snapshots: Vec<WorkerSessionSnapshotUpdate>,
    ) -> Result<(), sea_orm::DbErr> {
        for snapshot in snapshots {
            let Some(session) = self.get_session_model(&snapshot.worker_id).await? else {
                continue;
            };
            if session.status != STATUS_ONLINE {
                continue;
            }
            let mut active = session.into_active_model();
            active.capabilities_json = Set(snapshot.capabilities_json);
            active.structured_capabilities_json = Set(snapshot.structured_capabilities_json);
            active.labels_json = Set(snapshot.labels_json);
            active.master_json = Set(snapshot.master_json);
            active.updated_at = Set(now_rfc3339());
            active.update(&self.db).await?;
        }
        Ok(())
    }

    /// List persisted worker sessions ordered by most recently updated first.
    pub async fn list_sessions(
        &self,
        limit: u64,
    ) -> Result<Vec<WorkerSessionSummary>, sea_orm::DbErr> {
        let sessions = worker_session::Entity::find()
            .order_by_desc(worker_session::Column::UpdatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;
        Ok(sessions
            .into_iter()
            .map(|session| WorkerSessionSummary::from_model(session, None))
            .collect())
    }

    /// List recent lifecycle events across all worker sessions.
    pub async fn list_recent_events(
        &self,
        limit: u64,
    ) -> Result<Vec<WorkerSessionEventSummary>, sea_orm::DbErr> {
        let events = worker_session_event::Entity::find()
            .order_by_desc(worker_session_event::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;
        Ok(events
            .into_iter()
            .map(WorkerSessionEventSummary::from_model)
            .collect())
    }

    /// List lifecycle events for one worker session.
    pub async fn list_session_events(
        &self,
        worker_id: &str,
    ) -> Result<Vec<WorkerSessionEventSummary>, sea_orm::DbErr> {
        let events = worker_session_event::Entity::find()
            .filter(worker_session_event::Column::WorkerId.eq(worker_id.to_owned()))
            .order_by_asc(worker_session_event::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(events
            .into_iter()
            .map(WorkerSessionEventSummary::from_model)
            .collect())
    }

    async fn upsert_logical_instance(
        &self,
        input: &RegisterWorkerSession,
        now: &str,
    ) -> Result<worker_logical_instance::Model, sea_orm::DbErr> {
        if let Some(existing) = self.find_logical_instance(input).await? {
            return Ok(existing);
        }
        worker_logical_instance::ActiveModel {
            id: Set(new_id("worker-logical")),
            namespace_name: Set(input.namespace_name.clone()),
            app_name: Set(input.app_name.clone()),
            cluster: Set(input.cluster.clone()),
            region: Set(input.region.clone()),
            client_instance_id: Set(input.client_instance_id.clone()),
            current_worker_id: Set(None),
            current_generation: Set(0),
            status: Set(STATUS_ACTIVE.to_owned()),
            last_seen_at: Set(now.to_owned()),
            created_at: Set(now.to_owned()),
            updated_at: Set(now.to_owned()),
        }
        .insert(&self.db)
        .await
    }

    async fn find_logical_instance(
        &self,
        input: &RegisterWorkerSession,
    ) -> Result<Option<worker_logical_instance::Model>, sea_orm::DbErr> {
        worker_logical_instance::Entity::find()
            .filter(worker_logical_instance::Column::NamespaceName.eq(input.namespace_name.clone()))
            .filter(worker_logical_instance::Column::AppName.eq(input.app_name.clone()))
            .filter(worker_logical_instance::Column::Cluster.eq(input.cluster.clone()))
            .filter(worker_logical_instance::Column::Region.eq(input.region.clone()))
            .filter(
                worker_logical_instance::Column::ClientInstanceId
                    .eq(input.client_instance_id.clone()),
            )
            .one(&self.db)
            .await
    }

    async fn replace_current_session(
        &self,
        logical: &worker_logical_instance::Model,
        replacement_worker_id: &str,
        now: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let Some(current_worker_id) = logical.current_worker_id.as_deref() else {
            return Ok(());
        };
        let Some(current) = self.get_session_model(current_worker_id).await? else {
            return Ok(());
        };
        if current.status != STATUS_ONLINE {
            return Ok(());
        }
        let mut active = current.into_active_model();
        active.status = Set(STATUS_REPLACED.to_owned());
        active.status_reason = Set(Some(REASON_REPLACED.to_owned()));
        active.status_evidence = Set(Some(
            "same logical instance registered a newer generation".to_owned(),
        ));
        active.replaced_by_worker_id = Set(Some(replacement_worker_id.to_owned()));
        active.disconnected_at = Set(Some(now.to_owned()));
        active.updated_at = Set(now.to_owned());
        let replaced = active.update(&self.db).await?;
        self.record_event(
            &replaced.worker_id,
            &replaced.logical_instance_id,
            "session_replaced",
            Some(REASON_REPLACED),
            Some(&format!(
                "{{\"replaced_by_worker_id\":\"{replacement_worker_id}\"}}"
            )),
            now,
        )
        .await
    }

    async fn upsert_session(
        &self,
        input: &RegisterWorkerSession,
        logical_instance_id: &str,
        generation: i64,
        now: &str,
    ) -> Result<worker_session::Model, sea_orm::DbErr> {
        let model = worker_session::ActiveModel {
            worker_id: Set(input.worker_id.clone()),
            logical_instance_id: Set(logical_instance_id.to_owned()),
            connection_id: Set(input.connection_id.clone()),
            generation: Set(generation),
            fencing_token_hash: Set(hash_token(&input.fencing_token)),
            status: Set(STATUS_ONLINE.to_owned()),
            status_reason: Set(None),
            status_evidence: Set(None),
            lease_expires_at: Set(rfc3339_after_seconds(input.lease_seconds)),
            last_heartbeat_at: Set(now.to_owned()),
            last_sequence: Set(0),
            connected_at: Set(now.to_owned()),
            disconnected_at: Set(None),
            replaced_by_worker_id: Set(None),
            drain_requested_at: Set(None),
            capabilities_json: Set(input.capabilities_json.clone()),
            structured_capabilities_json: Set(input.structured_capabilities_json.clone()),
            labels_json: Set(input.labels_json.clone()),
            master_json: Set(input.master_json.clone()),
            created_at: Set(now.to_owned()),
            updated_at: Set(now.to_owned()),
        };
        worker_session::Entity::insert(model)
            .on_conflict(
                OnConflict::column(worker_session::Column::WorkerId)
                    .update_columns([
                        worker_session::Column::LogicalInstanceId,
                        worker_session::Column::ConnectionId,
                        worker_session::Column::Generation,
                        worker_session::Column::FencingTokenHash,
                        worker_session::Column::Status,
                        worker_session::Column::StatusReason,
                        worker_session::Column::StatusEvidence,
                        worker_session::Column::LeaseExpiresAt,
                        worker_session::Column::LastHeartbeatAt,
                        worker_session::Column::LastSequence,
                        worker_session::Column::ConnectedAt,
                        worker_session::Column::DisconnectedAt,
                        worker_session::Column::ReplacedByWorkerId,
                        worker_session::Column::DrainRequestedAt,
                        worker_session::Column::CapabilitiesJson,
                        worker_session::Column::StructuredCapabilitiesJson,
                        worker_session::Column::LabelsJson,
                        worker_session::Column::MasterJson,
                        worker_session::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await?;
        self.get_session_model(&input.worker_id)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(input.worker_id.clone()))
    }

    async fn promote_logical_current(
        &self,
        logical: worker_logical_instance::Model,
        worker_id: &str,
        generation: i64,
        now: &str,
    ) -> Result<worker_logical_instance::Model, sea_orm::DbErr> {
        let mut active = logical.into_active_model();
        active.current_worker_id = Set(Some(worker_id.to_owned()));
        active.current_generation = Set(generation);
        active.status = Set(STATUS_ACTIVE.to_owned());
        active.last_seen_at = Set(now.to_owned());
        active.updated_at = Set(now.to_owned());
        active.update(&self.db).await
    }

    async fn mark_session_gracefully_stopped(
        &self,
        session: worker_session::Model,
        now: &str,
    ) -> Result<worker_session::Model, sea_orm::DbErr> {
        let mut active = session.into_active_model();
        active.status = Set(STATUS_STOPPED.to_owned());
        active.status_reason = Set(Some(REASON_GRACEFUL_SHUTDOWN.to_owned()));
        active.status_evidence = Set(Some(
            "worker sent graceful unregister before closing tunnel".to_owned(),
        ));
        active.disconnected_at = Set(Some(now.to_owned()));
        active.updated_at = Set(now.to_owned());
        active.update(&self.db).await
    }

    async fn mark_logical_inactive_if_current(
        &self,
        logical_instance_id: &str,
        worker_id: &str,
        now: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let Some(logical) = worker_logical_instance::Entity::find_by_id(logical_instance_id)
            .one(&self.db)
            .await?
        else {
            return Ok(());
        };
        if logical.current_worker_id.as_deref() != Some(worker_id) {
            return Ok(());
        }
        let mut active = logical.into_active_model();
        active.status = Set(STATUS_INACTIVE.to_owned());
        active.updated_at = Set(now.to_owned());
        active.update(&self.db).await?;
        Ok(())
    }

    async fn mark_session_transport_error(
        &self,
        session: worker_session::Model,
        evidence: &str,
        now: &str,
    ) -> Result<worker_session::Model, sea_orm::DbErr> {
        let mut active = session.into_active_model();
        active.status = Set(STATUS_OFFLINE.to_owned());
        active.status_reason = Set(Some(REASON_TRANSPORT_ERROR.to_owned()));
        active.status_evidence = Set(Some(evidence.to_owned()));
        active.disconnected_at = Set(Some(now.to_owned()));
        active.updated_at = Set(now.to_owned());
        active.update(&self.db).await
    }

    async fn mark_session_lease_expired(
        &self,
        session: worker_session::Model,
        now: &str,
    ) -> Result<worker_session::Model, sea_orm::DbErr> {
        let mut active = session.into_active_model();
        active.status = Set(STATUS_OFFLINE.to_owned());
        active.status_reason = Set(Some(REASON_LEASE_EXPIRED_UNKNOWN.to_owned()));
        active.status_evidence = Set(Some(
            "lease expired without graceful shutdown, replacement, or transport close evidence"
                .to_owned(),
        ));
        active.disconnected_at = Set(Some(now.to_owned()));
        active.updated_at = Set(now.to_owned());
        active.update(&self.db).await
    }

    async fn mark_logical_degraded_if_current(
        &self,
        logical_instance_id: &str,
        worker_id: &str,
        now: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let Some(logical) = worker_logical_instance::Entity::find_by_id(logical_instance_id)
            .one(&self.db)
            .await?
        else {
            return Ok(());
        };
        if logical.current_worker_id.as_deref() != Some(worker_id) {
            return Ok(());
        }
        let mut active = logical.into_active_model();
        active.status = Set(STATUS_DEGRADED.to_owned());
        active.updated_at = Set(now.to_owned());
        active.update(&self.db).await?;
        Ok(())
    }

    async fn get_session_model(
        &self,
        worker_id: &str,
    ) -> Result<Option<worker_session::Model>, sea_orm::DbErr> {
        worker_session::Entity::find_by_id(worker_id.to_owned())
            .one(&self.db)
            .await
    }

    async fn record_event(
        &self,
        worker_id: &str,
        logical_instance_id: &str,
        event_type: &str,
        reason: Option<&str>,
        detail_json: Option<&str>,
        now: &str,
    ) -> Result<(), sea_orm::DbErr> {
        worker_session_event::ActiveModel {
            id: Set(new_id("worker-event")),
            worker_id: Set(worker_id.to_owned()),
            logical_instance_id: Set(logical_instance_id.to_owned()),
            event_type: Set(event_type.to_owned()),
            reason: Set(reason.map(str::to_owned)),
            detail_json: Set(detail_json.map(str::to_owned)),
            created_at: Set(now.to_owned()),
        }
        .insert(&self.db)
        .await?;
        Ok(())
    }
}

impl WorkerSessionSummary {
    fn from_model(model: worker_session::Model, current_worker_id: Option<String>) -> Self {
        Self {
            worker_id: model.worker_id,
            logical_instance_id: model.logical_instance_id,
            current_worker_id,
            generation: model.generation,
            status: model.status,
            status_reason: model.status_reason,
            status_evidence: model.status_evidence,
            lease_expires_at: model.lease_expires_at,
            last_heartbeat_at: model.last_heartbeat_at,
            last_sequence: model.last_sequence,
            capabilities_json: model.capabilities_json,
            structured_capabilities_json: model.structured_capabilities_json,
            labels_json: model.labels_json,
            master_json: model.master_json,
            replaced_by_worker_id: model.replaced_by_worker_id,
        }
    }
}

impl WorkerSessionEventSummary {
    fn from_model(model: worker_session_event::Model) -> Self {
        Self {
            id: model.id,
            worker_id: model.worker_id,
            logical_instance_id: model.logical_instance_id,
            event_type: model.event_type,
            reason: model.reason,
            detail_json: model.detail_json,
            created_at: model.created_at,
        }
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn session_accepts_heartbeat(session: &worker_session::Model, input: &WorkerHeartbeat) -> bool {
    session.status == STATUS_ONLINE
        && session.generation == input.generation
        && session.fencing_token_hash == hash_token(&input.fencing_token)
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    format!("{digest:x}")
}
