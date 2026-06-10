use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    QueryFilter, Set, Statement,
};
use sea_orm_migration::MigratorTrait;

use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};

use crate::{
    entities::{auth_session, dispatch_queue, job_instance},
    migration::Migrator,
    repository::{
        AppendJobInstanceLog, CreateJob, CreateJobInstance, CreateScript, RaftRepository,
        RecordRaftAppliedCommand, ScopeRepository, ScriptRepository, UpdateJob, UpdateScript,
        UpdateWorkerPoolQuota, UpsertRaftLogEntry, UpsertRaftMember, UpsertRaftMetadata,
        UpsertRaftSnapshot, VerifiedScriptReleaseGrants, VerifiedScriptReleaseSignature,
    },
};

use super::{JobInstanceRepository, JobRepository};

async fn seed_worker_pool_quota(
    db: &DatabaseConnection,
    namespace: &str,
    app: &str,
    pool: &str,
    max_queue_depth: i32,
    max_concurrency: i32,
) {
    let scopes = ScopeRepository::new(db.clone());
    let worker_pool = scopes
        .create_worker_pool(namespace, app, pool)
        .await
        .unwrap_or_else(|error| panic!("worker pool should create: {error}"));
    scopes
        .update_worker_pool_quota(
            &worker_pool.id,
            UpdateWorkerPoolQuota {
                max_queue_depth,
                max_concurrency,
            },
        )
        .await
        .unwrap_or_else(|error| panic!("worker pool quota should update: {error}"))
        .unwrap_or_else(|| panic!("worker pool should exist"));
}

async fn insert_scoped_job_queue_item(
    db: &DatabaseConnection,
    namespace: &str,
    app: &str,
    pool: &str,
    status: &str,
) -> String {
    let now = super::util::now_rfc3339();
    let instance_id = super::util::new_id("inst");
    job_instance::ActiveModel {
        id: Set(instance_id.clone()),
        job_id: Set(super::util::new_id("job")),
        status: Set(if status == "running" {
            "running".to_owned()
        } else {
            "pending".to_owned()
        }),
        trigger_type: Set("api".to_owned()),
        execution_mode: Set("single".to_owned()),
        result_worker_id: Set(None),
        result_success: Set(None),
        result_message: Set(None),
        result_completed_at: Set(None),
        created_at: Set(now.clone()),
        updated_at: Set(now.clone()),
    }
    .insert(db)
    .await
    .unwrap_or_else(|error| panic!("job instance should insert: {error}"));
    let queue_id = super::util::new_id("dq");
    dispatch_queue::ActiveModel {
        id: Set(queue_id.clone()),
        job_instance_id: Set(Some(instance_id)),
        workflow_node_instance_id: Set(None),
        priority: Set(0),
        run_after: Set(now.clone()),
        status: Set(status.to_owned()),
        attempt: Set(0),
        lease_owner: Set(None),
        lease_until: Set(None),
        fencing_token: Set(None),
        worker_selector: Set(None),
        namespace: Set(Some(namespace.to_owned())),
        app: Set(Some(app.to_owned())),
        worker_pool: Set(Some(pool.to_owned())),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .unwrap_or_else(|error| panic!("dispatch queue item should insert: {error}"));
    queue_id
}

include!("tests/part_01.rs");
include!("tests/part_02.rs");
include!("tests/part_03.rs");
