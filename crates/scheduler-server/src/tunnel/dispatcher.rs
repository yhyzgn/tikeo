//! Minimal pending-instance dispatcher for Worker Tunnel sessions.

use std::time::Duration;

use scheduler_core::{
    InstanceStatus, ScriptLanguage, ScriptStatus, WasmCapabilities, WasmProcessorSpec,
};
use scheduler_proto::worker::v1::{
    DispatchTask, TaskProcessorBinding, WasmProcessorBinding, task_processor_binding,
};
use scheduler_storage::{
    JobInstanceAttemptRepository, JobInstanceRepository, JobRepository, ScriptRepository,
    ScriptSummary, WorkflowRepository,
};
use tokio::time;
use tracing::{debug, warn};

use super::WorkerRegistry;
use crate::cluster::SharedClusterCoordinator;

const DISPATCH_INTERVAL: Duration = Duration::from_millis(500);
const DISPATCH_BATCH_SIZE: u64 = 16;
const DISPATCH_LEASE_SECONDS: i64 = 30;
const DISPATCHER_LEASE_OWNER: &str = "scheduler-dispatcher";

fn dispatcher_fencing_token(node_id: &str, leader_fencing_token: Option<&str>) -> String {
    leader_fencing_token.map_or_else(
        || format!("standalone:{node_id}:{DISPATCHER_LEASE_OWNER}"),
        |token| format!("raft:{node_id}:{token}"),
    )
}

/// Run the minimal single-node dispatch loop forever.
pub async fn run(
    jobs: JobRepository,
    instances: JobInstanceRepository,
    attempts: JobInstanceAttemptRepository,
    workflows: WorkflowRepository,
    scripts: ScriptRepository,
    registry: WorkerRegistry,
    cluster: SharedClusterCoordinator,
) {
    let mut ticker = time::interval(DISPATCH_INTERVAL);
    loop {
        ticker.tick().await;
        if let Err(error) = dispatch_once_if_owner(
            &jobs, &instances, &attempts, &workflows, &scripts, &registry, &cluster,
        )
        .await
        {
            warn!(%error, "worker dispatch iteration failed");
        }
    }
}

async fn dispatch_once_if_owner(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    registry: &WorkerRegistry,
    cluster: &SharedClusterCoordinator,
) -> Result<(), scheduler_storage::DbErr> {
    let status = cluster.status().await;
    if !status.can_schedule {
        debug!(role = status.role.as_str(), node_id = %status.node_id, "skip worker dispatch without cluster ownership");
        return Ok(());
    }
    let fencing_token =
        dispatcher_fencing_token(&status.node_id, status.leader_fencing_token.as_deref());
    dispatch_once(
        jobs,
        instances,
        attempts,
        workflows,
        scripts,
        registry,
        &fencing_token,
    )
    .await
}

async fn dispatch_once(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    registry: &WorkerRegistry,
    fencing_token: &str,
) -> Result<(), scheduler_storage::DbErr> {
    let _expired = workflows.clear_expired_dispatch_queue_leases().await?;
    let _ = workflows
        .materialize_next_queued_node_with_fencing(
            DISPATCHER_LEASE_OWNER,
            DISPATCH_LEASE_SECONDS,
            fencing_token,
        )
        .await?;
    dispatch_single_instances(jobs, instances, workflows, scripts, registry, fencing_token).await?;
    dispatch_broadcast_attempts(jobs, instances, attempts, workflows, scripts, registry).await
}

async fn dispatch_single_instances(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    registry: &WorkerRegistry,
    fencing_token: &str,
) -> Result<(), scheduler_storage::DbErr> {
    for _ in 0..DISPATCH_BATCH_SIZE {
        let Some(claim) = workflows
            .claim_next_job_queue_item_with_fencing(
                DISPATCHER_LEASE_OWNER,
                DISPATCH_LEASE_SECONDS,
                fencing_token,
            )
            .await?
        else {
            break;
        };
        let Some(instance_id) = claim.item.job_instance_id.clone() else {
            continue;
        };
        let Some(instance) = instances.get(&instance_id).await? else {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            continue;
        };
        if !instances.claim_pending_for_dispatch(&instance.id).await? {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            continue;
        }
        let Some(job) = jobs.get(&instance.job_id).await? else {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            instances
                .update_status(&instance.id, InstanceStatus::Pending)
                .await?;
            continue;
        };

        let processor_name = resolve_processor_name(workflows, &instance.id, &job).await?;
        let task = build_dispatch_task(
            scripts,
            instance.id.clone(),
            instance.job_id.clone(),
            processor_name,
        )
        .await?;

        let eligible_workers = registry
            .find_eligible_workers(&job.namespace, &job.app)
            .await;
        if let Some(worker_id) = eligible_workers.first()
            && let Some(worker_id) = registry.dispatch_to_worker(worker_id, task).await
        {
            instances
                .update_status(&instance.id, InstanceStatus::Running)
                .await?;
            let _ = workflows
                .mark_dispatch_queue_running(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            debug!(%worker_id, instance_id = %instance.id, "dispatched instance to worker");
        } else {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            instances
                .update_status(&instance.id, InstanceStatus::Pending)
                .await?;
        }
    }

    Ok(())
}

async fn dispatch_broadcast_attempts(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    registry: &WorkerRegistry,
) -> Result<(), scheduler_storage::DbErr> {
    let pending = attempts.list_pending(DISPATCH_BATCH_SIZE).await?;

    for attempt in pending {
        let Some(instance) = instances.get(&attempt.instance_id).await? else {
            continue;
        };
        let processor_name = if let Some(job) = jobs.get(&instance.job_id).await? {
            resolve_processor_name(workflows, &instance.id, &job).await?
        } else {
            instance.job_id.clone()
        };
        let task = build_dispatch_task(
            scripts,
            attempt.instance_id.clone(),
            instance.job_id.clone(),
            processor_name,
        )
        .await?;

        if let Some(worker_id) = registry.dispatch_to_worker(&attempt.worker_id, task).await {
            attempts
                .update_status(
                    &attempt.instance_id,
                    &attempt.worker_id,
                    InstanceStatus::Running,
                )
                .await?;
            instances
                .update_status(&attempt.instance_id, InstanceStatus::Running)
                .await?;
            debug!(%worker_id, instance_id = %attempt.instance_id, "dispatched broadcast attempt to worker");
        }
    }

    Ok(())
}

async fn build_dispatch_task(
    scripts: &ScriptRepository,
    instance_id: String,
    job_id: String,
    processor_name: String,
) -> Result<DispatchTask, scheduler_storage::DbErr> {
    let processor_binding = if let Some(script_id) = processor_name.strip_prefix("script:") {
        match scripts.get(script_id).await? {
            Some(script) if wasm_script_is_dispatchable(&script) => {
                Some(Box::new(wasm_processor_binding(&script)))
            }
            _ => None,
        }
    } else {
        None
    };

    Ok(DispatchTask {
        instance_id,
        job_id,
        payload: Vec::new(),
        processor_name,
        processor_binding,
    })
}

fn wasm_script_is_dispatchable(script: &ScriptSummary) -> bool {
    script.language == ScriptLanguage::Wasm.as_str()
        && script.status == ScriptStatus::Approved.as_str()
        && script_to_wasm_spec(script).validate().is_ok()
}

fn wasm_processor_binding(script: &ScriptSummary) -> TaskProcessorBinding {
    let spec = script_to_wasm_spec(script);
    TaskProcessorBinding {
        kind: Some(task_processor_binding::Kind::Wasm(WasmProcessorBinding {
            script_id: script.id.clone(),
            version: script.version.clone(),
            module: script.content.as_bytes().to_vec(),
            runtime: spec.runtime.as_str().to_owned(),
            entrypoint: spec.entrypoint,
            timeout_ms: spec.resources.timeout_ms,
            max_memory_bytes: spec.resources.max_memory_bytes,
            fuel: spec.resources.fuel,
            allow_network: spec.capabilities.network,
            allowed_env_vars: spec.capabilities.env_vars,
        })),
    }
}

fn script_to_wasm_spec(script: &ScriptSummary) -> WasmProcessorSpec {
    let mut spec = WasmProcessorSpec::default();
    spec.resources.timeout_ms = script
        .timeout_seconds
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .map_or(spec.resources.timeout_ms, |seconds| {
            seconds.saturating_mul(1000)
        });
    spec.resources.max_memory_bytes = script
        .max_memory_bytes
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(spec.resources.max_memory_bytes);
    spec.capabilities = WasmCapabilities {
        network: script.allow_network,
        preopened_dirs: Vec::new(),
        env_vars: script.allowed_env_vars.clone().unwrap_or_default(),
    };
    spec
}

async fn resolve_processor_name(
    workflows: &WorkflowRepository,
    instance_id: &str,
    job: &scheduler_storage::JobSummary,
) -> Result<String, scheduler_storage::DbErr> {
    if let Some(processor_name) = workflows
        .processor_name_for_job_instance(instance_id)
        .await?
    {
        return Ok(processor_name);
    }
    Ok(job
        .processor_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&job.id)
        .to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::cluster::{ClusterMode, ClusterRole, ClusterStatus, StaticCoordinator};
    use scheduler_core::{ExecutionMode, InstanceStatus, TriggerType};
    use scheduler_proto::worker::v1::{RegisterWorker, server_message, task_processor_binding};
    use scheduler_storage::{
        CreateJob, CreateJobInstance, JobInstanceAttemptRepository, JobInstanceRepository,
        JobRepository, ScriptRepository, ScriptSummary, WorkflowRepository, connect_and_migrate,
        entities::script,
    };
    use sea_orm::{ActiveModelTrait, Set};
    use tokio::sync::mpsc;

    use super::{
        WorkerRegistry, dispatch_once, dispatch_once_if_owner, wasm_script_is_dispatchable,
    };

    #[tokio::test]
    async fn dispatch_once_sends_pending_instance_to_registered_worker() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: Some("billing.manual".to_owned()),
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
        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &workflows,
            &scripts,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, "billing.manual");
            }
            other => panic!("unexpected server message: {other:?}"),
        }

        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }

    #[tokio::test]
    async fn dispatch_once_filters_by_namespace_and_app() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: None,
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
        let registry = WorkerRegistry::default();
        let (tx1, mut rx1) = mpsc::channel(1);
        let (tx2, _rx2) = mpsc::channel(1);

        // This worker should match
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                },
                tx1,
            )
            .await;

        // This worker should NOT match
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-2".to_owned(),
                    app: "analytics".to_owned(), // Different app
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                },
                tx2,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &workflows,
            &scripts,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx1
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker-1 should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, job.id);
            }
            other => panic!("unexpected server message: {other:?}"),
        }

        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Running);
    }

    #[tokio::test]
    async fn follower_dispatch_does_not_claim_queue_items() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "follower-dispatch".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: None,
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        instances
            .create_pending(CreateJobInstance {
                job_id: job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));
        let registry = WorkerRegistry::default();
        let follower = StaticCoordinator::shared(ClusterStatus {
            mode: ClusterMode::Raft,
            role: ClusterRole::Follower,
            node_id: "node-b".to_owned(),
            nodes: 3,
            can_schedule: false,
            leader_fencing_token: None,
            detail: "test follower".to_owned(),
        });

        dispatch_once_if_owner(
            &jobs, &instances, &attempts, &workflows, &scripts, &registry, &follower,
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch gate should run: {error}"));

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        assert_eq!(overview.pending, 1);
        assert_eq!(overview.running, 0);
        assert!(overview.items[0].lease_owner.is_none());
    }

    #[tokio::test]
    async fn dispatch_once_prefers_workflow_node_processor_name() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: Some("job.default".to_owned()),
                enabled: true,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(scheduler_storage::CreateWorkflow {
                name: "processor override".to_owned(),
                created_by: "test".to_owned(),
                definition: scheduler_storage::WorkflowDefinition {
                    nodes: vec![scheduler_storage::WorkflowNodeSpec {
                        key: "job-a".to_owned(),
                        name: Some("Job A".to_owned()),
                        kind: Some("job".to_owned()),
                        job_id: Some(job.id.clone()),
                        processor_name: Some("workflow.override".to_owned()),
                        child_workflow_id: None,
                        map_items: None,
                        config: None,
                    }],
                    edges: Vec::new(),
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"));
        workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("workflow node should materialize: {error}"));

        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &workflows,
            &scripts,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.processor_name, "workflow.override");
            }
            other => panic!("unexpected server message: {other:?}"),
        }
    }
    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn dispatch_includes_wasm_binding_only_for_approved_policy_safe_script() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let scripts = ScriptRepository::new(db.clone());
        let script = ScriptSummary {
            id: "script_wasm_approved".to_owned(),
            name: "wasm echo".to_owned(),
            language: "wasm".to_owned(),
            version: "1.0.0".to_owned(),
            content: "wasm-demo-module".to_owned(),
            status: "approved".to_owned(),
            timeout_seconds: Some(5),
            max_memory_bytes: Some(1024 * 1024),
            allow_network: false,
            allowed_env_vars: None,
            created_by: "tester".to_owned(),
            created_at: "now".to_owned(),
            updated_at: "now".to_owned(),
        };
        script::ActiveModel {
            id: Set(script.id.clone()),
            name: Set(script.name.clone()),
            language: Set(script.language.clone()),
            version: Set(script.version.clone()),
            content: Set(script.content.clone()),
            status: Set(script.status.clone()),
            timeout_seconds: Set(script.timeout_seconds),
            max_memory_bytes: Set(script.max_memory_bytes),
            allow_network: Set(script.allow_network),
            allowed_env_vars: Set(None),
            created_by: Set(script.created_by.clone()),
            created_at: Set(script.created_at.clone()),
            updated_at: Set(script.updated_at.clone()),
        }
        .insert(&db)
        .await
        .unwrap_or_else(|error| panic!("script should be inserted: {error}"));
        let job = jobs
            .create_job(CreateJob {
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "wasm job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                processor_name: Some(format!("script:{}", script.id)),
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
        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "worker-1".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    labels: HashMap::default(),
                },
                tx,
            )
            .await;

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &workflows,
            &scripts,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, instance.id);
                assert_eq!(task.processor_name, format!("script:{}", script.id));
                let binding = task
                    .processor_binding
                    .unwrap_or_else(|| panic!("wasm binding expected"));
                match binding.kind {
                    Some(task_processor_binding::Kind::Wasm(wasm)) => {
                        assert_eq!(wasm.script_id, script.id);
                        assert_eq!(wasm.runtime, "wasmtime");
                        assert_eq!(wasm.entrypoint, "_start");
                        assert_eq!(wasm.timeout_ms, 5_000);
                        assert_eq!(wasm.max_memory_bytes, 1024 * 1024);
                        assert!(!wasm.allow_network);
                        assert!(wasm.allowed_env_vars.is_empty());
                    }
                    other => panic!("unexpected binding: {other:?}"),
                }
            }
            other => panic!("unexpected server message: {other:?}"),
        }
    }

    #[tokio::test]
    async fn wasm_script_dispatch_eligibility_requires_approval_and_safe_policy() {
        let mut script = ScriptSummary {
            id: "script_1".to_owned(),
            name: "demo".to_owned(),
            language: "wasm".to_owned(),
            version: "1.0.0".to_owned(),
            content: "module".to_owned(),
            status: "draft".to_owned(),
            timeout_seconds: Some(1),
            max_memory_bytes: Some(1024),
            allow_network: false,
            allowed_env_vars: None,
            created_by: "tester".to_owned(),
            created_at: "now".to_owned(),
            updated_at: "now".to_owned(),
        };
        assert!(!wasm_script_is_dispatchable(&script));

        script.status = "approved".to_owned();
        assert!(wasm_script_is_dispatchable(&script));

        script.allow_network = true;
        assert!(!wasm_script_is_dispatchable(&script));
    }
}
