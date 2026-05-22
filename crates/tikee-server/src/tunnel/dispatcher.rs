//! Minimal pending-instance dispatcher for Worker Tunnel sessions.

use std::time::Duration;

use tikee_core::{
    InstanceStatus, ScriptExecutionPolicy, ScriptLanguage, ScriptPolicyError, ScriptStatus,
    WasmProcessorSpec,
};
use tikee_proto::worker::v1::{
    DispatchTask, ScriptProcessorBinding, TaskProcessorBinding, WasmProcessorBinding,
    task_processor_binding,
};
use tikee_storage::{
    AppendJobInstanceLog, JobInstanceAttemptRepository, JobInstanceRepository, JobRepository,
    ScriptRepository, ScriptSummary, ScriptVersionSummary, WorkflowRepository,
};
use tokio::time;
use tracing::{debug, warn};

use super::WorkerRegistry;
use crate::cluster::SharedClusterCoordinator;

const DISPATCH_INTERVAL: Duration = Duration::from_millis(500);
const DISPATCH_BATCH_SIZE: u64 = 16;
const DISPATCH_LEASE_SECONDS: i64 = 30;
const DISPATCHER_LEASE_OWNER: &str = "tikee-dispatcher";

fn dispatcher_fencing_token(node_id: &str, leader_fencing_token: Option<&str>) -> String {
    leader_fencing_token.map_or_else(
        || format!("standalone:{node_id}:{DISPATCHER_LEASE_OWNER}"),
        |token| format!("raft:{node_id}:{token}"),
    )
}

/// Run the minimal single-node dispatch loop forever.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    jobs: JobRepository,
    instances: JobInstanceRepository,
    attempts: JobInstanceAttemptRepository,
    workflows: WorkflowRepository,
    scripts: ScriptRepository,
    logs: tikee_storage::JobInstanceLogRepository,
    registry: WorkerRegistry,
    cluster: SharedClusterCoordinator,
) {
    let mut ticker = time::interval(DISPATCH_INTERVAL);
    loop {
        ticker.tick().await;
        if let Err(error) = dispatch_once_if_owner(
            &jobs, &instances, &attempts, &workflows, &scripts, &logs, &registry, &cluster,
        )
        .await
        {
            warn!(%error, "worker dispatch iteration failed");
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_once_if_owner(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    logs: &tikee_storage::JobInstanceLogRepository,
    registry: &WorkerRegistry,
    cluster: &SharedClusterCoordinator,
) -> Result<(), tikee_storage::DbErr> {
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
        logs,
        registry,
        &fencing_token,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_once(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    logs: &tikee_storage::JobInstanceLogRepository,
    registry: &WorkerRegistry,
    fencing_token: &str,
) -> Result<(), tikee_storage::DbErr> {
    let _expired = workflows.clear_expired_dispatch_queue_leases().await?;
    let _ = workflows
        .materialize_next_queued_node_with_fencing(
            DISPATCHER_LEASE_OWNER,
            DISPATCH_LEASE_SECONDS,
            fencing_token,
        )
        .await?;
    dispatch_single_instances(
        jobs,
        instances,
        workflows,
        scripts,
        logs,
        registry,
        fencing_token,
    )
    .await?;
    dispatch_broadcast_attempts(
        jobs, instances, attempts, workflows, scripts, logs, registry,
    )
    .await
}

async fn dispatch_single_instances(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    logs: &tikee_storage::JobInstanceLogRepository,
    registry: &WorkerRegistry,
    fencing_token: &str,
) -> Result<(), tikee_storage::DbErr> {
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
        let task = match build_dispatch_task(
            scripts,
            instance.id.clone(),
            instance.job_id.clone(),
            processor_name,
        )
        .await?
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                append_script_governance_log(logs, &instance.id, &failure).await?;
                let _ = workflows
                    .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                    .await?;
                instances
                    .update_status(&instance.id, InstanceStatus::Pending)
                    .await?;
                continue;
            }
        };

        let required_capability = required_task_capability(&task);
        let eligible_workers = registry
            .find_eligible_workers_with_capability(
                &job.namespace,
                &job.app,
                required_capability.as_deref(),
            )
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
            if let Some(capability) = required_capability.as_deref() {
                append_script_governance_log(
                    logs,
                    &instance.id,
                    &ScriptGovernanceFailure::NoEligibleWorkerCapability(capability.to_owned()),
                )
                .await?;
            }
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
    logs: &tikee_storage::JobInstanceLogRepository,
    registry: &WorkerRegistry,
) -> Result<(), tikee_storage::DbErr> {
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
        let task = match build_dispatch_task(
            scripts,
            attempt.instance_id.clone(),
            instance.job_id.clone(),
            processor_name,
        )
        .await?
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                append_script_governance_log(logs, &attempt.instance_id, &failure).await?;
                continue;
            }
        };

        let required_capability = required_task_capability(&task);
        if !registry
            .worker_supports_capability(&attempt.worker_id, required_capability.as_deref())
            .await
        {
            if let Some(capability) = required_capability.as_deref() {
                append_script_governance_log(
                    logs,
                    &attempt.instance_id,
                    &ScriptGovernanceFailure::NoEligibleWorkerCapability(capability.to_owned()),
                )
                .await?;
            }
            continue;
        }

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum DispatchTaskBuild {
    Built(DispatchTask),
    Rejected(ScriptGovernanceFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScriptGovernanceFailure {
    MissingScript,
    NotApproved,
    MissingReleasePointer,
    MissingReleasedVersion,
    UnsupportedLanguage,
    PolicyRejected(String),
    NoEligibleWorkerCapability(String),
}

impl ScriptGovernanceFailure {
    const fn code(&self) -> &'static str {
        match self {
            Self::MissingScript => "script_missing",
            Self::NotApproved => "script_not_approved",
            Self::MissingReleasePointer => "script_missing_release_pointer",
            Self::MissingReleasedVersion => "script_missing_released_version",
            Self::UnsupportedLanguage => "script_unsupported_language",
            Self::PolicyRejected(_) => "script_policy_rejected",
            Self::NoEligibleWorkerCapability(_) => "script_no_eligible_worker_capability",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::MissingScript => "script governance rejected dispatch: script definition is missing".to_owned(),
            Self::NotApproved => "script governance rejected dispatch: script is not approved".to_owned(),
            Self::MissingReleasePointer => {
                "script governance rejected dispatch: approved script has no released version pointer"
                    .to_owned()
            }
            Self::MissingReleasedVersion => {
                "script governance rejected dispatch: released script version is missing".to_owned()
            }
            Self::UnsupportedLanguage => {
                "script governance rejected dispatch: script language is unsupported".to_owned()
            }
            Self::PolicyRejected(reason) => {
                format!("script governance rejected dispatch: policy rejected ({reason})")
            }
            Self::NoEligibleWorkerCapability(capability) => format!(
                "script governance queued dispatch: no connected worker advertises required capability {capability}"
            ),
        }
    }
}

async fn append_script_governance_log(
    logs: &tikee_storage::JobInstanceLogRepository,
    instance_id: &str,
    failure: &ScriptGovernanceFailure,
) -> Result<(), tikee_storage::DbErr> {
    let payload = serde_json::json!({
        "event": "script_execution_governance",
        "failure_class": failure.code(),
        "message": failure.message(),
    });
    let _ = logs
        .append(AppendJobInstanceLog {
            instance_id: instance_id.to_owned(),
            worker_id: "tikee-dispatcher".to_owned(),
            level: "warn".to_owned(),
            message: payload.to_string(),
            sequence: 0,
        })
        .await?;
    Ok(())
}

async fn build_dispatch_task(
    scripts: &ScriptRepository,
    instance_id: String,
    job_id: String,
    processor_name: String,
) -> Result<DispatchTaskBuild, tikee_storage::DbErr> {
    let processor_binding = if let Some(script_id) = processor_name.strip_prefix("script:") {
        let Some(script) = scripts.get(script_id).await? else {
            warn!(%script_id, "script processor binding references missing script; dispatch remains pending");
            return Ok(DispatchTaskBuild::Rejected(
                ScriptGovernanceFailure::MissingScript,
            ));
        };
        if !script_is_dispatchable(&script) {
            warn!(script_id = %script.id, language = %script.language, status = %script.status, "script is not dispatchable; dispatch remains pending");
            return Ok(DispatchTaskBuild::Rejected(
                ScriptGovernanceFailure::NotApproved,
            ));
        }
        let Some(version_number) = script.released_version_number else {
            warn!(script_id = %script.id, "approved script has no released version pointer; dispatch remains pending");
            return Ok(DispatchTaskBuild::Rejected(
                ScriptGovernanceFailure::MissingReleasePointer,
            ));
        };
        let Some(version) = scripts
            .versions()
            .get_version_by_number(&script.id, version_number)
            .await?
        else {
            warn!(script_id = %script.id, version_number, "released script version is missing; dispatch remains pending");
            return Ok(DispatchTaskBuild::Rejected(
                ScriptGovernanceFailure::MissingReleasedVersion,
            ));
        };
        let Some(language) = parse_script_language(&version.language) else {
            warn!(script_id = %script.id, language = %version.language, "released script version has unsupported language; dispatch remains pending");
            return Ok(DispatchTaskBuild::Rejected(
                ScriptGovernanceFailure::UnsupportedLanguage,
            ));
        };
        if let Err(error) = validate_script_version_dispatchable(&version) {
            warn!(script_id = %script.id, version_number, language = %version.language, %error, "released script version policy is not dispatchable; dispatch remains pending");
            return Ok(DispatchTaskBuild::Rejected(
                ScriptGovernanceFailure::PolicyRejected(error.to_string()),
            ));
        }
        if language == ScriptLanguage::Wasm {
            Some(Box::new(wasm_processor_binding(&script, &version)))
        } else {
            Some(Box::new(script_processor_binding(&script, &version)))
        }
    } else {
        None
    };

    Ok(DispatchTaskBuild::Built(DispatchTask {
        instance_id,
        job_id,
        payload: Vec::new(),
        processor_name,
        processor_binding,
    }))
}

fn required_task_capability(task: &DispatchTask) -> Option<String> {
    let binding = task.processor_binding.as_ref()?;
    match binding.kind.as_ref()? {
        task_processor_binding::Kind::Wasm(_) => Some("script:wasm".to_owned()),
        task_processor_binding::Kind::Script(script) => Some(format!("script:{}", script.language)),
    }
}

fn script_is_dispatchable(script: &ScriptSummary) -> bool {
    script.status == ScriptStatus::Approved.as_str()
        && parse_script_language(&script.language).is_some()
}

#[cfg(test)]
fn script_version_is_dispatchable(version: &ScriptVersionSummary) -> bool {
    validate_script_version_dispatchable(version).is_ok()
}

fn validate_script_version_dispatchable(
    version: &ScriptVersionSummary,
) -> Result<(), ScriptDispatchValidationError> {
    match parse_script_language(&version.language) {
        Some(ScriptLanguage::Wasm) => script_version_to_wasm_spec(version)
            .validate()
            .map_err(|error| ScriptDispatchValidationError(error.to_string())),
        Some(
            ScriptLanguage::Shell
            | ScriptLanguage::Python
            | ScriptLanguage::Node
            | ScriptLanguage::PowerShell
            | ScriptLanguage::Rhai,
        ) => script_policy(version.policy.clone())
            .validate_default_deny()
            .map_err(ScriptDispatchValidationError::from),
        None => Err(ScriptDispatchValidationError(
            "script language is unsupported".to_owned(),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScriptDispatchValidationError(String);

impl From<ScriptPolicyError> for ScriptDispatchValidationError {
    fn from(value: ScriptPolicyError) -> Self {
        Self(value.to_string())
    }
}

impl std::fmt::Display for ScriptDispatchValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn parse_script_language(language: &str) -> Option<ScriptLanguage> {
    language.parse::<ScriptLanguage>().ok()
}

fn script_processor_binding(
    script: &ScriptSummary,
    version: &ScriptVersionSummary,
) -> TaskProcessorBinding {
    let policy = script_policy(version.policy.clone());
    TaskProcessorBinding {
        kind: Some(task_processor_binding::Kind::Script(
            ScriptProcessorBinding {
                script_id: script.id.clone(),
                version: script.version.clone(),
                language: version.language.clone(),
                content: version.content.as_bytes().to_vec(),
                version_id: version.id.clone(),
                version_number: u64::try_from(version.version_number).unwrap_or_default(),
                content_sha256: version.content_sha256.clone(),
                timeout_ms: policy.resources.timeout_ms,
                max_memory_bytes: policy.resources.max_memory_bytes,
                max_output_bytes: policy.resources.max_output_bytes,
                allow_network: policy.network.enabled,
                allowed_env_vars: policy.env_vars,
                read_only_paths: policy.filesystem.read_only_paths,
                writable_paths: policy.filesystem.writable_paths,
                secret_refs: policy.secrets.refs,
            },
        )),
    }
}

fn script_policy(value: serde_json::Value) -> ScriptExecutionPolicy {
    serde_json::from_value(value).unwrap_or_default()
}

fn wasm_processor_binding(
    script: &ScriptSummary,
    version: &ScriptVersionSummary,
) -> TaskProcessorBinding {
    let spec = script_version_to_wasm_spec(version);
    TaskProcessorBinding {
        kind: Some(task_processor_binding::Kind::Wasm(WasmProcessorBinding {
            script_id: script.id.clone(),
            version: script.version.clone(),
            module: version.content.as_bytes().to_vec(),
            runtime: spec.runtime.as_str().to_owned(),
            entrypoint: spec.entrypoint,
            timeout_ms: spec.resources.timeout_ms,
            max_memory_bytes: spec.resources.max_memory_bytes,
            fuel: spec.resources.fuel,
            allow_network: spec.capabilities.network,
            allowed_env_vars: spec.capabilities.env_vars,
            version_id: version.id.clone(),
            version_number: u64::try_from(version.version_number).unwrap_or_default(),
            module_sha256: version.content_sha256.clone(),
            module_signature: String::new(),
        })),
    }
}

fn script_version_to_wasm_spec(version: &ScriptVersionSummary) -> WasmProcessorSpec {
    let mut spec = WasmProcessorSpec::default();
    spec.resources.timeout_ms = version
        .timeout_seconds
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .map_or(spec.resources.timeout_ms, |seconds| {
            seconds.saturating_mul(1000)
        });
    spec.resources.max_memory_bytes = version
        .max_memory_bytes
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(spec.resources.max_memory_bytes);
    spec.capabilities.network = version.allow_network;
    spec.capabilities.env_vars = version.allowed_env_vars.clone().unwrap_or_default();
    spec
}

async fn resolve_processor_name(
    workflows: &WorkflowRepository,
    instance_id: &str,
    job: &tikee_storage::JobSummary,
) -> Result<String, tikee_storage::DbErr> {
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
    use tikee_core::{ExecutionMode, InstanceStatus, TriggerType};
    use tikee_proto::worker::v1::{RegisterWorker, server_message, task_processor_binding};
    use tikee_storage::{
        CreateJob, CreateJobInstance, JobInstanceAttemptRepository, JobInstanceRepository,
        JobRepository, ScriptRepository, ScriptSummary, ScriptVersionSummary, WorkflowRepository,
        connect_and_migrate,
    };
    use tokio::sync::mpsc;

    use super::{
        DispatchTaskBuild, ScriptGovernanceFailure, WorkerRegistry, build_dispatch_task,
        dispatch_once, dispatch_once_if_owner, script_is_dispatchable,
        script_version_is_dispatchable,
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
        let logs = tikee_storage::JobInstanceLogRepository::new(db.clone());
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
            &logs,
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
        let logs = tikee_storage::JobInstanceLogRepository::new(db.clone());
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
            &logs,
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
        let logs = tikee_storage::JobInstanceLogRepository::new(db.clone());
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
            &jobs, &instances, &attempts, &workflows, &scripts, &logs, &registry, &follower,
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
        let logs = tikee_storage::JobInstanceLogRepository::new(db.clone());
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
            .create_workflow(tikee_storage::CreateWorkflow {
                name: "processor override".to_owned(),
                created_by: "test".to_owned(),
                definition: tikee_storage::WorkflowDefinition {
                    nodes: vec![tikee_storage::WorkflowNodeSpec {
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
            &logs,
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
        let logs = tikee_storage::JobInstanceLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(tikee_storage::CreateScript {
                name: "wasm echo".to_owned(),
                language: "wasm".to_owned(),
                version: "1.0.0".to_owned(),
                content: "wasm-demo-module".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(5),
                max_memory_bytes: Some(1024 * 1024),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: None,
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));
        let version = scripts
            .versions()
            .get_version_by_number(&script.id, 1)
            .await
            .unwrap_or_else(|error| panic!("script version should load: {error}"))
            .unwrap_or_else(|| panic!("script version should exist"));
        let script = scripts
            .publish_version(&script.id, version.version_number)
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        assert_eq!(script.status, "approved");
        assert_eq!(
            script.released_version_id.as_deref(),
            Some(version.id.as_str())
        );
        assert_eq!(script.released_version_number, Some(version.version_number));

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
                    capabilities: vec!["script:wasm".to_owned()],
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
            &logs,
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
                        assert_eq!(wasm.version_id, version.id);
                        assert_eq!(
                            wasm.version_number,
                            u64::try_from(version.version_number).unwrap_or_else(|error| panic!(
                                "version number should convert: {error}"
                            ))
                        );
                        assert_eq!(wasm.module_sha256, version.content_sha256);
                        assert_eq!(wasm.module_signature, "");
                        assert_eq!(wasm.module, version.content.as_bytes());
                    }
                    other => panic!("unexpected binding: {other:?}"),
                }
            }
            other => panic!("unexpected server message: {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_includes_non_wasm_script_binding_only_for_released_safe_script() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(tikee_storage::CreateScript {
                name: "shell echo".to_owned(),
                language: "shell".to_owned(),
                version: "1.0.0".to_owned(),
                content: "printf ok".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(5),
                max_memory_bytes: Some(1024 * 1024),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: Some(r#"{"resources":{"timeout_ms":7000,"max_memory_bytes":33554432,"max_output_bytes":4096},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":["SAFE_ENV"]}"#.to_owned()),
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));
        let version = scripts
            .versions()
            .get_version_by_number(&script.id, 1)
            .await
            .unwrap_or_else(|error| panic!("script version should load: {error}"))
            .unwrap_or_else(|| panic!("script version should exist"));
        let script = scripts
            .publish_version(&script.id, version.version_number)
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));

        let task = match build_dispatch_task(
            &scripts,
            "instance-shell".to_owned(),
            "job-shell".to_owned(),
            format!("script:{}", script.id),
        )
        .await
        .unwrap_or_else(|error| panic!("task build should not error: {error}"))
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                panic!("released safe script should dispatch: {failure:?}")
            }
        };

        let binding = task
            .processor_binding
            .unwrap_or_else(|| panic!("script binding expected"));
        match binding.kind {
            Some(task_processor_binding::Kind::Script(script_binding)) => {
                assert_eq!(script_binding.script_id, script.id);
                assert_eq!(script_binding.language, "shell");
                assert_eq!(script_binding.content, version.content.as_bytes());
                assert_eq!(script_binding.version_id, version.id);
                assert_eq!(script_binding.content_sha256, version.content_sha256);
                assert_eq!(script_binding.timeout_ms, 7_000);
                assert_eq!(script_binding.max_memory_bytes, 33_554_432);
                assert_eq!(script_binding.max_output_bytes, 4_096);
                assert!(!script_binding.allow_network);
                assert_eq!(script_binding.allowed_env_vars, vec!["SAFE_ENV"]);
                assert!(script_binding.read_only_paths.is_empty());
                assert!(script_binding.writable_paths.is_empty());
                assert!(script_binding.secret_refs.is_empty());
            }
            other => panic!("unexpected binding: {other:?}"),
        }
    }

    #[tokio::test]
    async fn approved_wasm_script_without_release_pointer_fails_closed() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let scripts = ScriptRepository::new(db);
        let script = scripts
            .create_script(tikee_storage::CreateScript {
                name: "wasm unreleased".to_owned(),
                language: "wasm".to_owned(),
                version: "1.0.0".to_owned(),
                content: "module".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(1),
                max_memory_bytes: Some(1024),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: None,
            })
            .await
            .unwrap_or_else(|error| panic!("script should be created: {error}"));
        let approved = scripts
            .update_script(
                &script.id,
                tikee_storage::UpdateScript {
                    name: None,
                    language: None,
                    version: None,
                    content: None,
                    status: Some("approved".to_owned()),
                    timeout_seconds: None,
                    max_memory_bytes: None,
                    allow_network: None,
                    allowed_env_vars: None,
                    policy_json: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("script should update: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        assert_eq!(approved.status, "approved");
        assert_eq!(approved.released_version_number, None);

        let task = build_dispatch_task(
            &scripts,
            "instance-1".to_owned(),
            "job-1".to_owned(),
            format!("script:{}", script.id),
        )
        .await
        .unwrap_or_else(|error| panic!("task build should not error: {error}"));
        assert!(matches!(
            task,
            DispatchTaskBuild::Rejected(ScriptGovernanceFailure::MissingReleasePointer)
        ));
    }

    #[tokio::test]
    async fn wasm_script_dispatch_eligibility_requires_approval_and_safe_policy() {
        let mut script = ScriptSummary {
            id: "script_1".to_owned(),
            name: "demo".to_owned(),
            language: "wasm".to_owned(),
            version: "1.0.0".to_owned(),
            content: "module".to_owned(),
            content_sha256: "af67347816654d9b144b131e2c92b8b6f6ba3edecb7f1911ef6d8a81f8e08329"
                .to_owned(),
            status: "draft".to_owned(),
            released_version_id: None,
            released_version_number: None,
            timeout_seconds: Some(1),
            max_memory_bytes: Some(1024),
            allow_network: false,
            allowed_env_vars: None,
            policy: serde_json::json!({
                "resources": {"timeout_ms": 30_000, "max_memory_bytes": 64 * 1024 * 1024, "max_output_bytes": 1024 * 1024},
                "network": {"enabled": false, "allowed_hosts": []},
                "filesystem": {"read_only_paths": [], "writable_paths": []},
                "secrets": {"refs": []},
                "env_vars": []
            }),
            created_by: "tester".to_owned(),
            created_at: "now".to_owned(),
            updated_at: "now".to_owned(),
        };
        assert!(!script_is_dispatchable(&script));

        script.status = "approved".to_owned();
        assert!(script_is_dispatchable(&script));

        script.language = "unknown".to_owned();
        assert!(!script_is_dispatchable(&script));

        let mut version = ScriptVersionSummary {
            id: "version_1".to_owned(),
            script_id: script.id.clone(),
            version_number: 1,
            content: "module".to_owned(),
            content_sha256: script.content_sha256.clone(),
            language: "wasm".to_owned(),
            status: "draft".to_owned(),
            timeout_seconds: Some(1),
            max_memory_bytes: Some(1024),
            allow_network: false,
            allowed_env_vars: None,
            policy: script.policy,
            created_by: "tester".to_owned(),
            created_at: "now".to_owned(),
        };
        assert!(script_version_is_dispatchable(&version));

        version.allow_network = true;
        assert!(!script_version_is_dispatchable(&version));
    }
}
