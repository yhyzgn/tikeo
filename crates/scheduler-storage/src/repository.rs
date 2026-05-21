//! Repository APIs over scheduler metadata tables.
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn
)]

mod attempt;
mod audit;
mod auth;
mod instance;
mod job;
mod job_repo;
mod log;
mod script;
mod user;
mod util;
mod workflow;

pub use attempt::{
    CreateJobInstanceAttempt, JobInstanceAttemptRepository, JobInstanceAttemptSummary,
};
pub use audit::{AuditLogFilters, AuditLogRepository, AuditLogSummary, CreateAuditLog};
pub use auth::{
    AuthSessionRepository, AuthSessionSummary, CreateAuthSession, PermissionSummary, RbacRepository,
};
pub use instance::{CreateJobInstance, JobInstanceRepository, JobInstanceSummary};
pub use job::{CreateJob, JobSummary};
pub use job_repo::JobRepository;
pub use log::{AppendJobInstanceLog, JobInstanceLogRepository, JobInstanceLogSummary};
pub use script::{
    CreateScript, ScriptRepository, ScriptSummary, ScriptVersionRepository, ScriptVersionSummary,
    UpdateScript,
};
pub use user::{CreateUser, UpdateUser, UserRepository, UserSummary};
pub use workflow::{
    AdvanceWorkflowInput, AdvanceWorkflowResult, CompleteWorkflowShardInput,
    CompleteWorkflowShardResult, CreateWorkflow, DispatchQueueClaim, DispatchQueueSummary,
    InstanceEventSummary, MaterializeWorkflowNodeResult, QueueOverview, RecoverWorkflowNodeInput,
    RecoverWorkflowNodeResult, UpdateWorkflow, WorkflowDefinition, WorkflowEdgeSpec,
    WorkflowInstanceSummary, WorkflowJobResultOutcome, WorkflowNodeInstanceSummary,
    WorkflowNodeSpec, WorkflowRepository, WorkflowShardSummary, WorkflowSummary,
    WorkflowValidationResult, validate_workflow_definition,
};

#[cfg(test)]
mod tests {
    use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, Set, Statement};
    use sea_orm_migration::MigratorTrait;

    use scheduler_core::{ExecutionMode, InstanceStatus, TriggerType};

    use crate::{
        entities::auth_session,
        migration::Migrator,
        repository::{AppendJobInstanceLog, CreateJob, CreateJobInstance},
    };

    use super::JobRepository;

    #[tokio::test]
    async fn migration_creates_metadata_tables() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let result = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT name FROM sqlite_master WHERE type='table' AND name='jobs'".to_owned(),
            ))
            .await
            .unwrap_or_else(|error| panic!("sqlite_master query should run: {error}"));

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn repository_creates_and_lists_jobs() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let repository = JobRepository::new(db);

        let created = repository
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "nightly".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));

        let jobs = repository
            .list_jobs()
            .await
            .unwrap_or_else(|error| panic!("jobs should list: {error}"));
        let scheduled = repository
            .list_enabled_scheduled_jobs()
            .await
            .unwrap_or_else(|error| panic!("scheduled jobs should list: {error}"));

        assert_eq!(created.name, "nightly");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].app, "billing");
        assert!(jobs[0].enabled);
        assert!(scheduled.is_empty());
    }

    #[tokio::test]
    async fn repository_creates_and_lists_job_instances() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = super::JobInstanceRepository::new(db);

        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));

        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));

        let listed = instances
            .list_by_job(&job.id)
            .await
            .unwrap_or_else(|error| panic!("instances should list: {error}"));

        assert_eq!(instance.status, InstanceStatus::Pending);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].trigger_type, TriggerType::Api);

        let pending = instances
            .list_pending_single(10)
            .await
            .unwrap_or_else(|error| panic!("pending instances should list: {error}"));
        assert_eq!(pending.len(), 1);

        let updated = instances
            .update_status(&instance.id, InstanceStatus::Running)
            .await
            .unwrap_or_else(|error| panic!("instance status should update: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }

    #[tokio::test]
    async fn repository_appends_and_lists_job_instance_logs() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = super::JobInstanceRepository::new(db.clone());
        let logs = super::JobInstanceLogRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));

        logs.append(AppendJobInstanceLog {
            instance_id: instance.id.clone(),
            worker_id: "worker-1".to_owned(),
            level: "info".to_owned(),
            message: "hello".to_owned(),
            sequence: 1,
        })
        .await
        .unwrap_or_else(|error| panic!("log should append: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));

        let listed = logs
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("logs should list: {error}"));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].message, "hello");
    }

    #[tokio::test]
    async fn auth_session_repository_deletes_expired_rows() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let users = super::UserRepository::new(db.clone());
        let admin = users
            .get_by_username("scheduler_init")
            .await
            .unwrap_or_else(|error| panic!("admin lookup should work: {error}"))
            .unwrap_or_else(|| panic!("seeded admin should exist"));
        let sessions = super::AuthSessionRepository::new(db.clone());
        auth_session::ActiveModel {
            id: Set("expired-session".to_owned()),
            user_id: Set(admin.id),
            token_hash: Set("expired-token-hash".to_owned()),
            device_id: Set(None),
            device_name: Set(None),
            expires_at: Set("1970-01-01T00:00:00Z".to_owned()),
            created_at: Set("1970-01-01T00:00:00Z".to_owned()),
            updated_at: Set("1970-01-01T00:00:00Z".to_owned()),
        }
        .insert(&db)
        .await
        .unwrap_or_else(|error| panic!("expired session should insert: {error}"));

        let deleted = sessions
            .delete_expired()
            .await
            .unwrap_or_else(|error| panic!("expired session should delete: {error}"));
        assert_eq!(deleted, 1);
        let loaded = sessions
            .get_by_token_hash("expired-token-hash")
            .await
            .unwrap_or_else(|error| panic!("session lookup should work: {error}"));
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn user_repository_crud_operations() {
        let db = Database::connect("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|error| panic!("migration should run: {error}"));

        let users = super::UserRepository::new(db);

        // Seeding checked
        let admin = users
            .get_by_username("scheduler_init")
            .await
            .unwrap_or_else(|error| panic!("should load admin user: {error}"));
        let admin = admin.unwrap_or_else(|| panic!("seeded admin should exist"));
        assert_eq!(admin.role, "admin");

        // Create user
        let user = users
            .create_user(super::CreateUser {
                username: "operator-1".to_owned(),
                password: "$2b$10$operatorhash".to_owned(),
                role: "operator".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("should create user: {error}"));
        assert_eq!(user.username, "operator-1");
        assert_eq!(user.role, "operator");

        // List users
        let listed = users
            .list_users()
            .await
            .unwrap_or_else(|error| panic!("should list users: {error}"));
        assert_eq!(listed.len(), 2); // admin + operator-1

        // Update user
        let updated = users
            .update_user(
                &user.id,
                super::UpdateUser {
                    password: None,
                    role: Some("viewer".to_owned()),
                },
            )
            .await
            .unwrap_or_else(|error| panic!("should update user: {error}"))
            .unwrap_or_else(|| panic!("user should exist"));
        assert_eq!(updated.role, "viewer");

        // Delete user
        let deleted = users
            .delete_user(&user.id)
            .await
            .unwrap_or_else(|error| panic!("should delete user: {error}"));
        assert!(deleted);

        // List users again
        let listed_again = users
            .list_users()
            .await
            .unwrap_or_else(|error| panic!("should list users: {error}"));
        assert_eq!(listed_again.len(), 1); // just admin
    }
    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_job_result_auto_advances_next_node() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db.clone());
        let first_job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "first".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("first job should be created: {error}"));
        let second_job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "second".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("second job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "auto-advance".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "first".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(first_job.id),
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                        super::WorkflowNodeSpec {
                            key: "second".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(second_job.id),
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "first".to_owned(),
                        to: "second".to_owned(),
                        condition: Some("on_success".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("node should materialize: {error}"))
            .unwrap_or_else(|| panic!("queued node should exist"));
        assert_eq!(materialized.queue_item.status, "done");
        assert_eq!(materialized.queue_item.lease_owner, None);
        let job_claim = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("job queue should claim: {error}"))
            .unwrap_or_else(|| panic!("job queue item should exist"));
        assert_eq!(job_claim.item.attempt, 1);
        let job_instance_id = materialized
            .node
            .job_instance_id
            .clone()
            .unwrap_or_else(|| panic!("job node should create job instance"));

        let running_marked = workflows
            .mark_dispatch_queue_running(&job_claim.item.id, "server-a")
            .await
            .unwrap_or_else(|error| panic!("job queue should mark running: {error}"));
        assert!(running_marked);

        let outcome = workflows
            .complete_job_node_from_result(&job_instance_id, InstanceStatus::Succeeded, None)
            .await
            .unwrap_or_else(|error| panic!("workflow should advance from job result: {error}"))
            .unwrap_or_else(|| panic!("job should be linked to workflow node"));

        assert_eq!(outcome.node_key, "first");
        assert_eq!(outcome.status, "succeeded");
        assert_eq!(outcome.queued_nodes, vec!["second".to_owned()]);
        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.status, "running");
        assert_eq!(refreshed.nodes[0].status, "succeeded");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn dispatch_queue_claim_sets_and_releases_lease() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "claimable".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "claim-flow".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "start".to_owned(),
                        name: None,
                        kind: Some("job".to_owned()),
                        job_id: Some(job.id),
                        child_workflow_id: None,
                        map_items: None,
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let _instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let claim = workflows
            .claim_next_dispatch_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("queue should claim: {error}"))
            .unwrap_or_else(|| panic!("queue item should be claimable"));
        assert_eq!(claim.lease_owner, "server-a");
        assert_eq!(claim.item.lease_owner.as_deref(), Some("server-a"));
        assert_eq!(claim.item.attempt, 1);
        assert!(claim.item.workflow_node_instance_id.is_some());

        let cleared = workflows
            .clear_expired_dispatch_queue_leases()
            .await
            .unwrap_or_else(|error| panic!("expired lease cleanup should run: {error}"));
        assert_eq!(cleared, 0);

        let second_claim = workflows
            .claim_dispatch_queue_item(&claim.item.id, "server-b", 30)
            .await
            .unwrap_or_else(|error| panic!("second claim should not error: {error}"));
        assert!(second_claim.is_none());
        assert!(
            workflows
                .release_dispatch_queue_item(&claim.item.id, "server-a")
                .await
                .unwrap_or_else(|error| panic!("release should succeed: {error}"))
        );
        let reclaimed = workflows
            .claim_dispatch_queue_item(&claim.item.id, "server-b", 30)
            .await
            .unwrap_or_else(|error| panic!("reclaim should succeed: {error}"))
            .unwrap_or_else(|| panic!("released item should be claimable"));
        assert_eq!(reclaimed.lease_owner, "server-b");
        assert_eq!(reclaimed.item.attempt, 2);
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_shards_complete_and_advance_successor() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let reduce_job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "reduce".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "shards".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "map".to_owned(),
                            name: None,
                            kind: Some("map".to_owned()),
                            job_id: None,
                            child_workflow_id: None,
                            map_items: Some(vec![
                                serde_json::json!({"n": 1}),
                                serde_json::json!({"n": 2}),
                            ]),
                            config: None,
                        },
                        super::WorkflowNodeSpec {
                            key: "reduce".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(reduce_job.id),
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "map".to_owned(),
                        to: "reduce".to_owned(),
                        condition: Some("on_success".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("map should materialize: {error}"))
            .unwrap_or_else(|| panic!("map queue should exist"));
        assert_eq!(materialized.shards.len(), 2);
        assert!(
            materialized
                .shards
                .iter()
                .all(|shard| shard.job_instance_id.is_some())
        );

        let first = workflows
            .complete_workflow_shard(
                &materialized.shards[0].id,
                super::CompleteWorkflowShardInput {
                    status: "succeeded".to_owned(),
                    output: Some(serde_json::json!({"ok": 1})),
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("first shard should complete: {error}"))
            .unwrap_or_else(|| panic!("first shard should exist"));
        assert!(!first.node_completed);
        assert!(first.advance.is_none());

        let second = workflows
            .complete_workflow_shard(
                &materialized.shards[1].id,
                super::CompleteWorkflowShardInput {
                    status: "succeeded".to_owned(),
                    output: Some(serde_json::json!({"ok": 2})),
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("second shard should complete: {error}"))
            .unwrap_or_else(|| panic!("second shard should exist"));
        assert!(second.node_completed);
        assert_eq!(second.node_status.as_deref(), Some("succeeded"));
        assert_eq!(
            second
                .advance
                .as_ref()
                .map(|advance| advance.queued_nodes.as_slice()),
            Some(&["reduce".to_owned()][..])
        );

        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "succeeded");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn child_workflow_completion_advances_parent_node() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let child_job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "child-job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let child = workflows
            .create_workflow(super::CreateWorkflow {
                name: "child".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "child-task".to_owned(),
                        name: None,
                        kind: Some("job".to_owned()),
                        job_id: Some(child_job.id),
                        child_workflow_id: None,
                        map_items: None,
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("child workflow should be created: {error}"));
        let parent = workflows
            .create_workflow(super::CreateWorkflow {
                name: "parent".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "child".to_owned(),
                        name: None,
                        kind: Some("sub_workflow".to_owned()),
                        job_id: None,
                        child_workflow_id: Some(child.id),
                        map_items: None,
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("parent workflow should be created: {error}"));
        let parent_instance = workflows
            .run_workflow(&parent.id, "api")
            .await
            .unwrap_or_else(|error| panic!("parent should run: {error}"))
            .unwrap_or_else(|| panic!("parent should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("sub workflow should materialize: {error}"))
            .unwrap_or_else(|| panic!("sub workflow queue should exist"));
        let child_instance_id = materialized
            .node
            .child_workflow_instance_id
            .clone()
            .unwrap_or_else(|| panic!("child instance id should exist"));

        let advanced = workflows
            .advance_workflow(
                &child_instance_id,
                super::AdvanceWorkflowInput {
                    node_key: "child-task".to_owned(),
                    status: "succeeded".to_owned(),
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("child should advance: {error}"))
            .unwrap_or_else(|| panic!("child should exist"));
        assert!(advanced.completed);

        let refreshed = workflows
            .get_workflow_instance(&parent_instance.id)
            .await
            .unwrap_or_else(|error| panic!("parent should load: {error}"))
            .unwrap_or_else(|| panic!("parent should exist"));
        assert_eq!(refreshed.status, "succeeded");
        assert_eq!(refreshed.nodes[0].status, "succeeded");
    }
}
