//! Minimal pending-instance dispatcher for Worker Tunnel sessions.

use std::net::IpAddr;
use std::path::{Path, PathBuf};
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
    AppendJobInstanceLog, AuditLogRepository, JobInstanceAttemptRepository, JobInstanceRepository,
    JobRepository, ScriptRepository, ScriptSummary, ScriptVersionSummary, WorkflowRepository,
};
use tokio::time;
use tracing::{debug, warn};

use super::{WorkerRegistry, capability::WorkerRequirement, governance};
use crate::cluster::SharedClusterCoordinator;

const DISPATCH_INTERVAL: Duration = Duration::from_millis(500);
const DISPATCH_BATCH_SIZE: u64 = 16;
const DISPATCH_LEASE_SECONDS: i64 = 30;
const DISPATCH_RETRY_BACKOFF_SECONDS: i64 = 2;
const DISPATCH_STALE_RUNNING_SECONDS: i64 = 60;
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
    audit: AuditLogRepository,
    registry: WorkerRegistry,
    cluster: SharedClusterCoordinator,
) {
    let mut ticker = time::interval(DISPATCH_INTERVAL);
    loop {
        ticker.tick().await;
        if let Err(error) = dispatch_once_if_owner(
            &jobs, &instances, &attempts, &workflows, &scripts, &logs, &audit, &registry, &cluster,
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
    audit: &AuditLogRepository,
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
        audit,
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
    audit: &AuditLogRepository,
    registry: &WorkerRegistry,
    fencing_token: &str,
) -> Result<(), tikee_storage::DbErr> {
    let recovered = workflows
        .requeue_stale_running_job_dispatches(DISPATCH_STALE_RUNNING_SECONDS)
        .await?;
    if recovered > 0 {
        warn!(recovered, "requeued stale running job dispatches");
    }
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
        audit,
        registry,
        fencing_token,
    )
    .await?;
    dispatch_broadcast_attempts(
        jobs, instances, attempts, workflows, scripts, logs, audit, registry,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_single_instances(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    logs: &tikee_storage::JobInstanceLogRepository,
    audit: &AuditLogRepository,
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
                .mark_dispatch_queue_failed(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            warn!(queue_id = %claim.item.id, %instance_id, "closed dispatch queue item for missing job instance");
            continue;
        };
        if instance.status != InstanceStatus::Pending {
            let _ = workflows
                .mark_dispatch_queue_done_by_instance(&instance.id)
                .await?;
            debug!(instance_id = %instance.id, status = %instance.status, "closed dispatch queue item for non-pending instance");
            continue;
        }
        if !instances.claim_pending_for_dispatch(&instance.id).await? {
            let _ = workflows
                .release_dispatch_queue_item(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            continue;
        }
        let Some(job) = jobs.get(&instance.job_id).await? else {
            let _ = workflows
                .mark_dispatch_queue_failed(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            instances
                .update_status(&instance.id, InstanceStatus::Failed)
                .await?;
            warn!(queue_id = %claim.item.id, instance_id = %instance.id, job_id = %instance.job_id, "closed dispatch queue item for missing job");
            continue;
        };

        let executor = resolve_job_executor(workflows, &instance.id, &job).await?;
        if let JobExecutor::Http { config } = &executor {
            let outcome = execute_http_processor(config).await;
            let status = if outcome.success {
                InstanceStatus::Succeeded
            } else {
                InstanceStatus::Failed
            };
            let _ = workflows
                .mark_dispatch_queue_done_by_instance(&instance.id)
                .await?;
            instances.update_status(&instance.id, status).await?;
            logs.append(AppendJobInstanceLog {
                instance_id: instance.id.clone(),
                worker_id: "builtin.http".to_owned(),
                level: if outcome.success {
                    "info".to_owned()
                } else {
                    "error".to_owned()
                },
                message: outcome.message.clone(),
                sequence: 1,
            })
            .await?;
            let _ = workflows
                .complete_job_node_from_result(&instance.id, status, Some(outcome.message))
                .await?;
            continue;
        }
        if let JobExecutor::Grpc { config } = &executor {
            let outcome = execute_grpc_processor(config).await;
            let status = if outcome.success {
                InstanceStatus::Succeeded
            } else {
                InstanceStatus::Failed
            };
            let _ = workflows
                .mark_dispatch_queue_done_by_instance(&instance.id)
                .await?;
            instances.update_status(&instance.id, status).await?;
            logs.append(AppendJobInstanceLog {
                instance_id: instance.id.clone(),
                worker_id: "builtin.grpc".to_owned(),
                level: if outcome.success { "info".to_owned() } else { "error".to_owned() },
                message: outcome.message.clone(),
                sequence: 1,
            })
            .await?;
            let _ = workflows
                .complete_job_node_from_result(&instance.id, status, Some(outcome.message))
                .await?;
            continue;
        }
        if let JobExecutor::Sql { config } = &executor {
            let outcome = execute_sql_processor(config).await;
            let status = if outcome.success {
                InstanceStatus::Succeeded
            } else {
                InstanceStatus::Failed
            };
            let _ = workflows
                .mark_dispatch_queue_done_by_instance(&instance.id)
                .await?;
            instances.update_status(&instance.id, status).await?;
            logs.append(AppendJobInstanceLog {
                instance_id: instance.id.clone(),
                worker_id: "builtin.sql".to_owned(),
                level: if outcome.success { "info".to_owned() } else { "error".to_owned() },
                message: outcome.message.clone(),
                sequence: 1,
            })
            .await?;
            let _ = workflows
                .complete_job_node_from_result(&instance.id, status, Some(outcome.message))
                .await?;
            continue;
        }

        if let JobExecutor::FileCleanup { config } = &executor {
            let outcome = execute_file_cleanup_processor(config).await;
            let status = if outcome.success {
                InstanceStatus::Succeeded
            } else {
                InstanceStatus::Failed
            };
            let _ = workflows
                .mark_dispatch_queue_done_by_instance(&instance.id)
                .await?;
            instances.update_status(&instance.id, status).await?;
            logs.append(AppendJobInstanceLog {
                instance_id: instance.id.clone(),
                worker_id: "builtin.file_cleanup".to_owned(),
                level: if outcome.success { "info".to_owned() } else { "error".to_owned() },
                message: outcome.message.clone(),
                sequence: 1,
            })
            .await?;
            let _ = workflows
                .complete_job_node_from_result(&instance.id, status, Some(outcome.message))
                .await?;
            continue;
        }
        let task = match build_dispatch_task(
            scripts,
            instance.id.clone(),
            instance.job_id.clone(),
            executor.clone(),
        )
        .await?
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                append_script_governance_log(logs, audit, &instance.id, &failure).await?;
                let _ = workflows
                    .mark_dispatch_queue_failed(&claim.item.id, DISPATCHER_LEASE_OWNER)
                    .await?;
                instances
                    .update_status(&instance.id, InstanceStatus::Failed)
                    .await?;
                continue;
            }
        };

        let requirement = required_task_requirement_for_executor(&task, &executor);
        let eligible_workers = registry
            .find_eligible_workers_with_requirement(&job.namespace, &job.app, requirement.as_ref())
            .await;
        if let Some(worker_id) = eligible_workers.first()
            && let Some(worker_id) = registry.dispatch_to_worker(worker_id, task).await
        {
            instances
                .update_status_if_current(
                    &instance.id,
                    InstanceStatus::Dispatching,
                    InstanceStatus::Running,
                )
                .await?;
            let _ = workflows
                .mark_dispatch_queue_running(&claim.item.id, DISPATCHER_LEASE_OWNER)
                .await?;
            debug!(%worker_id, instance_id = %instance.id, "dispatched instance to worker");
        } else {
            if let Some(requirement) = requirement.as_ref() {
                append_script_governance_log(
                    logs,
                    audit,
                    &instance.id,
                    &ScriptGovernanceFailure::NoEligibleWorkerCapability(
                        requirement.display_label(),
                    ),
                )
                .await?;
                let _ = workflows
                    .mark_dispatch_queue_failed(&claim.item.id, DISPATCHER_LEASE_OWNER)
                    .await?;
                instances
                    .update_status(&instance.id, InstanceStatus::Failed)
                    .await?;
                continue;
            }
            let _ = workflows
                .release_dispatch_queue_item_after(
                    &claim.item.id,
                    DISPATCHER_LEASE_OWNER,
                    DISPATCH_RETRY_BACKOFF_SECONDS,
                )
                .await?;
            instances
                .update_status(&instance.id, InstanceStatus::Pending)
                .await?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_broadcast_attempts(
    jobs: &JobRepository,
    instances: &JobInstanceRepository,
    attempts: &JobInstanceAttemptRepository,
    workflows: &WorkflowRepository,
    scripts: &ScriptRepository,
    logs: &tikee_storage::JobInstanceLogRepository,
    audit: &AuditLogRepository,
    registry: &WorkerRegistry,
) -> Result<(), tikee_storage::DbErr> {
    let pending = attempts.list_pending(DISPATCH_BATCH_SIZE).await?;

    for attempt in pending {
        let Some(instance) = instances.get(&attempt.instance_id).await? else {
            continue;
        };
        let executor = if let Some(job) = jobs.get(&instance.job_id).await? {
            resolve_job_executor(workflows, &instance.id, &job).await?
        } else {
            JobExecutor::SdkProcessor {
                processor_name: instance.job_id.clone(),
                processor_type: None,
            }
        };
        let task = match build_dispatch_task(
            scripts,
            attempt.instance_id.clone(),
            instance.job_id.clone(),
            executor.clone(),
        )
        .await?
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                append_script_governance_log(logs, audit, &attempt.instance_id, &failure).await?;
                attempts
                    .update_status(
                        &attempt.instance_id,
                        &attempt.worker_id,
                        InstanceStatus::Failed,
                    )
                    .await?;
                continue;
            }
        };

        let requirement = required_task_requirement_for_executor(&task, &executor);
        if !registry
            .worker_supports_requirement(&attempt.worker_id, requirement.as_ref())
            .await
        {
            if let Some(requirement) = requirement.as_ref() {
                append_script_governance_log(
                    logs,
                    audit,
                    &attempt.instance_id,
                    &ScriptGovernanceFailure::NoEligibleWorkerCapability(
                        requirement.display_label(),
                    ),
                )
                .await?;
                attempts
                    .update_status(
                        &attempt.instance_id,
                        &attempt.worker_id,
                        InstanceStatus::Failed,
                    )
                    .await?;
            }
            continue;
        }

        if let Some(worker_id) = registry.dispatch_to_worker(&attempt.worker_id, task).await {
            attempts
                .update_status_if_current(
                    &attempt.instance_id,
                    &attempt.worker_id,
                    InstanceStatus::Pending,
                    InstanceStatus::Running,
                )
                .await?;
            instances
                .update_status_if_current(
                    &attempt.instance_id,
                    InstanceStatus::Pending,
                    InstanceStatus::Running,
                )
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
                "script governance failed dispatch: no connected worker advertises required capability {capability}"
            ),
        }
    }
}

async fn append_script_governance_log(
    logs: &tikee_storage::JobInstanceLogRepository,
    audit: &AuditLogRepository,
    instance_id: &str,
    failure: &ScriptGovernanceFailure,
) -> Result<(), tikee_storage::DbErr> {
    let failure_class = failure.code();
    let message = failure.message();
    let payload = governance::script_governance_payload(failure_class, &message);
    let _ = logs
        .append(AppendJobInstanceLog {
            instance_id: instance_id.to_owned(),
            worker_id: "tikee-dispatcher".to_owned(),
            level: "warn".to_owned(),
            message: payload.to_string(),
            sequence: 0,
        })
        .await?;
    governance::materialize_script_governance_audit(
        audit,
        "tikee-dispatcher",
        instance_id,
        failure_class,
        &message,
    )
    .await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JobExecutor {
    SdkProcessor {
        processor_name: String,
        processor_type: Option<String>,
    },
    Script {
        script_id: String,
    },
    Http {
        config: serde_json::Value,
    },
    Grpc {
        config: serde_json::Value,
    },
    Sql {
        config: serde_json::Value,
    },
    FileCleanup {
        config: serde_json::Value,
    },
}

async fn build_dispatch_task(
    scripts: &ScriptRepository,
    instance_id: String,
    job_id: String,
    executor: JobExecutor,
) -> Result<DispatchTaskBuild, tikee_storage::DbErr> {
    let (processor_name, processor_binding) = match executor {
        JobExecutor::Script { script_id } => {
            let Some(script) = scripts.get(&script_id).await? else {
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
            if let Err(error) =
                validate_script_version_dispatchable(&version, script.release_grants.as_ref())
            {
                warn!(script_id = %script.id, version_number, language = %version.language, %error, "released script version policy is not dispatchable; dispatch remains pending");
                return Ok(DispatchTaskBuild::Rejected(
                    ScriptGovernanceFailure::PolicyRejected(error.to_string()),
                ));
            }

            (
                script.id.clone(),
                if language == ScriptLanguage::Wasm {
                    Some(Box::new(wasm_processor_binding(&script, &version)))
                } else {
                    Some(Box::new(script_processor_binding(&script, &version)))
                },
            )
        }
        JobExecutor::SdkProcessor { processor_name, .. } => (processor_name, None),
        JobExecutor::Http { .. } => ("builtin.http".to_owned(), None),
        JobExecutor::Grpc { .. } => ("builtin.grpc".to_owned(), None),
        JobExecutor::Sql { .. } => ("builtin.sql".to_owned(), None),
        JobExecutor::FileCleanup { .. } => ("builtin.file_cleanup".to_owned(), None),
    };

    Ok(DispatchTaskBuild::Built(DispatchTask {
        instance_id,
        job_id,
        payload: Vec::new(),
        processor_name,
        processor_binding,
        assignment_token: String::new(),
    }))
}

fn required_task_requirement_for_executor(
    task: &DispatchTask,
    executor: &JobExecutor,
) -> Option<WorkerRequirement> {
    match executor {
        JobExecutor::SdkProcessor {
            processor_name,
            processor_type: Some(processor_type),
        } if !processor_type.trim().is_empty() && processor_type != "sdk" => {
            Some(WorkerRequirement::PluginProcessor {
                processor_type: processor_type.trim().to_owned(),
                processor_name: processor_name.trim().to_owned(),
            })
        }
        JobExecutor::SdkProcessor { processor_name, .. } => Some(WorkerRequirement::SdkProcessor {
            name: processor_name.trim().to_owned(),
        }),
        JobExecutor::Script { .. } => required_task_requirement(task),
        JobExecutor::Http { .. }
        | JobExecutor::Grpc { .. }
        | JobExecutor::Sql { .. }
        | JobExecutor::FileCleanup { .. } => None,
    }
}

fn required_task_requirement(task: &DispatchTask) -> Option<WorkerRequirement> {
    let binding = task.processor_binding.as_ref()?;
    match binding.kind.as_ref()? {
        task_processor_binding::Kind::Wasm(_) => Some(WorkerRequirement::ScriptRunner {
            language: "wasm".to_owned(),
        }),
        task_processor_binding::Kind::Script(script) => Some(WorkerRequirement::ScriptRunner {
            language: script.language.trim().to_owned(),
        }),
    }
}

fn script_is_dispatchable(script: &ScriptSummary) -> bool {
    script.status == ScriptStatus::Approved.as_str()
        && parse_script_language(&script.language).is_some()
}

#[cfg(test)]
fn script_version_is_dispatchable(version: &ScriptVersionSummary) -> bool {
    validate_script_version_dispatchable(version, None).is_ok()
}

fn validate_script_version_dispatchable(
    version: &ScriptVersionSummary,
    release_grants: Option<&tikee_storage::ScriptReleaseGrantEvidenceSummary>,
) -> Result<(), ScriptDispatchValidationError> {
    match parse_script_language(&version.language) {
        Some(ScriptLanguage::Wasm) => script_version_to_wasm_spec(version)
            .validate()
            .map_err(|error| ScriptDispatchValidationError(error.to_string())),
        Some(
            ScriptLanguage::Shell
            | ScriptLanguage::Python
            | ScriptLanguage::Js
            | ScriptLanguage::Ts
            | ScriptLanguage::PowerShell
            | ScriptLanguage::Rhai,
        ) => validate_script_policy_for_dispatch(
            &script_policy(version.policy.clone()),
            release_grants,
        ),
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

fn validate_script_policy_for_dispatch(
    policy: &ScriptExecutionPolicy,
    release_grants: Option<&tikee_storage::ScriptReleaseGrantEvidenceSummary>,
) -> Result<(), ScriptDispatchValidationError> {
    if policy.resources.timeout_ms == 0 {
        return Err(ScriptDispatchValidationError(
            "script timeout must be greater than zero".to_owned(),
        ));
    }
    if policy.resources.max_memory_bytes == 0 {
        return Err(ScriptDispatchValidationError(
            "script memory limit must be greater than zero".to_owned(),
        ));
    }
    if policy.resources.max_output_bytes == 0 {
        return Err(ScriptDispatchValidationError(
            "script output limit must be greater than zero".to_owned(),
        ));
    }
    if release_grants.is_none() {
        policy
            .validate_default_deny()
            .map_err(ScriptDispatchValidationError::from)?;
    }
    Ok(())
}

fn script_processor_binding(
    script: &ScriptSummary,
    version: &ScriptVersionSummary,
) -> TaskProcessorBinding {
    let policy = script_policy(version.policy.clone());
    let release_grants = script.release_grants.as_ref();
    let language = parse_script_language(&version.language).map_or_else(
        || version.language.clone(),
        |language| language.as_str().to_owned(),
    );
    TaskProcessorBinding {
        kind: Some(task_processor_binding::Kind::Script(
            ScriptProcessorBinding {
                script_id: script.id.clone(),
                version: script.version.clone(),
                language,
                content: version.content.as_bytes().to_vec(),
                version_id: version.id.clone(),
                version_number: u64::try_from(version.version_number).unwrap_or_default(),
                content_sha256: version.content_sha256.clone(),
                timeout_ms: policy.resources.timeout_ms,
                max_memory_bytes: policy.resources.max_memory_bytes,
                max_output_bytes: policy.resources.max_output_bytes,
                allow_network: policy.network.enabled
                    || release_grants.is_some_and(|grants| !grants.url.is_empty()),
                allowed_env_vars: policy.env_vars,
                read_only_paths: release_grants
                    .map(|grants| grants.file_read.clone())
                    .unwrap_or(policy.filesystem.read_only_paths),
                writable_paths: release_grants
                    .map(|grants| grants.file_write.clone())
                    .unwrap_or(policy.filesystem.writable_paths),
                secret_refs: release_grants
                    .map(|grants| grants.secret.clone())
                    .unwrap_or(policy.secrets.refs),
                allowed_network_hosts: release_grants
                    .map(|grants| grants.url.clone())
                    .unwrap_or(policy.network.allowed_hosts),
                sandbox_backend: policy.sandbox.backend.as_str().to_owned(),
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

async fn resolve_job_executor(
    workflows: &WorkflowRepository,
    instance_id: &str,
    job: &tikee_storage::JobSummary,
) -> Result<JobExecutor, tikee_storage::DbErr> {
    if let Some(binding) = workflows.job_binding_for_instance(instance_id).await? {
        if binding.node_kind == "http" {
            return Ok(JobExecutor::Http {
                config: binding.config.unwrap_or_else(|| serde_json::json!({})),
            });
        }
        if binding.node_kind == "grpc" {
            return Ok(JobExecutor::Grpc {
                config: binding.config.unwrap_or_else(|| serde_json::json!({})),
            });
        }
        if binding.node_kind == "sql" {
            return Ok(JobExecutor::Sql {
                config: binding.config.unwrap_or_else(|| serde_json::json!({})),
            });
        }
        if binding.node_kind == "file_cleanup" {
            return Ok(JobExecutor::FileCleanup {
                config: binding.config.unwrap_or_else(|| serde_json::json!({})),
            });
        }
        if let Some(processor_name) = binding.processor_name {
            return Ok(JobExecutor::SdkProcessor {
                processor_name,
                processor_type: None,
            });
        }
    }
    if let Some(script_id) = job
        .script_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(JobExecutor::Script {
            script_id: script_id.to_owned(),
        });
    }
    Ok(JobExecutor::SdkProcessor {
        processor_name: job
            .processor_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&job.name)
            .to_owned(),
        processor_type: job.processor_type.clone(),
    })
}

#[derive(Debug, Clone)]
struct SqlProcessorOutcome {
    success: bool,
    message: String,
}

async fn execute_sql_processor(config: &serde_json::Value) -> SqlProcessorOutcome {
    let Some(database_url) = config
        .get("databaseUrl")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return SqlProcessorOutcome { success: false, message: "sql node requires config.databaseUrl".to_owned() };
    };
    let Some(sql) = config
        .get("sql")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return SqlProcessorOutcome { success: false, message: "sql node requires config.sql".to_owned() };
    };
    let allowed = string_array(config.get("allowedDatabaseUrls"));
    if allowed.is_empty() || !allowed.iter().any(|candidate| candidate == database_url) {
        return SqlProcessorOutcome { success: false, message: "sql databaseUrl is not in allowedDatabaseUrls".to_owned() };
    }
    let read_only = config
        .get("readOnly")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let dry_run = config
        .get("dryRun")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if read_only && !is_read_only_sql(sql) {
        return SqlProcessorOutcome { success: false, message: "sql readOnly mode only allows SELECT/EXPLAIN/WITH statements".to_owned() };
    }
    if dry_run {
        return SqlProcessorOutcome { success: true, message: "sql dry-run validated statement and datasource allowlist".to_owned() };
    }
    if !database_url.starts_with("sqlite:") {
        return SqlProcessorOutcome { success: false, message: "sql executor currently supports sqlite databaseUrl for direct execution".to_owned() };
    }
    let pool = match sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => return SqlProcessorOutcome { success: false, message: format!("sql connect failed: {error}") },
    };
    let result = if read_only {
        sqlx::query(sql).fetch_all(&pool).await.map(|rows| rows.len().to_string())
    } else {
        sqlx::query(sql).execute(&pool).await.map(|result| result.rows_affected().to_string())
    };
    match result {
        Ok(count) if read_only => SqlProcessorOutcome { success: true, message: format!("sql query returned {count} row(s)") },
        Ok(count) => SqlProcessorOutcome { success: true, message: format!("sql statement affected {count} row(s)") },
        Err(error) => SqlProcessorOutcome { success: false, message: format!("sql execution failed: {error}") },
    }
}

fn is_read_only_sql(sql: &str) -> bool {
    let normalized = sql
        .trim_start_matches(|ch: char| ch.is_whitespace() || ch == ';')
        .to_ascii_lowercase();
    normalized.starts_with("select ")
        || normalized.starts_with("select\n")
        || normalized == "select"
        || normalized.starts_with("with ")
        || normalized.starts_with("explain ")
}

#[derive(Debug, Clone)]
struct GrpcProcessorOutcome {
    success: bool,
    message: String,
}

async fn execute_grpc_processor(config: &serde_json::Value) -> GrpcProcessorOutcome {
    let Some(endpoint) = config
        .get("endpoint")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return GrpcProcessorOutcome { success: false, message: "grpc node requires config.endpoint".to_owned() };
    };
    let Some(service) = config
        .get("service")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return GrpcProcessorOutcome { success: false, message: "grpc node requires config.service".to_owned() };
    };
    let Some(method) = config
        .get("method")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return GrpcProcessorOutcome { success: false, message: "grpc node requires config.method".to_owned() };
    };
    let allowed_hosts = string_array(config.get("allowedHosts"));
    let url = match url::Url::parse(endpoint) {
        Ok(url) => url,
        Err(error) => return GrpcProcessorOutcome { success: false, message: format!("invalid grpc endpoint: {error}") },
    };
    if url.scheme() != "http" && url.scheme() != "https" {
        return GrpcProcessorOutcome { success: false, message: "grpc endpoint only allows http/https schemes".to_owned() };
    }
    let Some(host) = url.host_str() else {
        return GrpcProcessorOutcome { success: false, message: "grpc endpoint must include host".to_owned() };
    };
    if is_private_or_loopback_host(host) && !config.get("allowPrivateHost").and_then(serde_json::Value::as_bool).unwrap_or(false) {
        return GrpcProcessorOutcome { success: false, message: "grpc node rejects loopback/private IP hosts by default".to_owned() };
    }
    if !allowed_hosts.is_empty() && !allowed_hosts.iter().any(|allowed| host_matches(host, allowed)) {
        return GrpcProcessorOutcome { success: false, message: format!("grpc host {host} is not in allowedHosts") };
    }
    let path = format!("/{}/{}", service.trim_matches('/'), method.trim_matches('/'));
    let uri = match tonic::codegen::http::uri::PathAndQuery::from_maybe_shared(path.clone()) {
        Ok(uri) => uri,
        Err(error) => return GrpcProcessorOutcome { success: false, message: format!("invalid grpc method path {path}: {error}") },
    };
    let payload = config
        .get("payload")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let type_url = payload
        .get("typeUrl")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("type.googleapis.com/tikee.workflow.v1.JsonPayload")
        .to_owned();
    let value = payload
        .get("valueBase64")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value).ok())
        .unwrap_or_else(|| payload.get("json").map_or_else(Vec::new, |json| serde_json::to_vec(json).unwrap_or_default()));
    let any = prost_types::Any { type_url, value };
    let channel = match tonic::transport::Endpoint::from_shared(endpoint.to_owned()) {
        Ok(endpoint) => match endpoint.timeout(Duration::from_secs(15)).connect().await {
            Ok(channel) => channel,
            Err(error) => return GrpcProcessorOutcome { success: false, message: format!("grpc connect failed: {error}") },
        },
        Err(error) => return GrpcProcessorOutcome { success: false, message: format!("invalid grpc endpoint: {error}") },
    };
    let mut client = tonic::client::Grpc::new(channel);
    let mut request = tonic::Request::new(any);
    if let Some(metadata) = config.get("metadata").and_then(serde_json::Value::as_object) {
        for (key, value) in metadata {
            if let Some(value) = value.as_str()
                && let Ok(name) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes())
                && let Ok(parsed) = value.parse()
            {
                request.metadata_mut().insert(name, parsed);
            }
        }
    }
    match client
        .unary(request, uri, tonic_prost::ProstCodec::<prost_types::Any, prost_types::Any>::default())
        .await
    {
        Ok(_) => GrpcProcessorOutcome { success: true, message: format!("grpc {service}/{method} succeeded") },
        Err(status) => GrpcProcessorOutcome { success: false, message: format!("grpc {service}/{method} failed: {}", status.message()) },
    }
}

#[derive(Debug, Clone)]
struct FileCleanupOutcome {
    success: bool,
    message: String,
}

async fn execute_file_cleanup_processor(config: &serde_json::Value) -> FileCleanupOutcome {
    let dry_run = config
        .get("dryRun")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let allowed_roots = string_array(config.get("allowedRoots"))
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if allowed_roots.is_empty() {
        return FileCleanupOutcome {
            success: false,
            message: "file_cleanup requires non-empty config.allowedRoots".to_owned(),
        };
    }
    let mut paths = string_array(config.get("paths"));
    if let Some(path) = config.get("path").and_then(serde_json::Value::as_str) {
        paths.push(path.to_owned());
    }
    if paths.is_empty() {
        return FileCleanupOutcome {
            success: false,
            message: "file_cleanup requires config.paths".to_owned(),
        };
    }
    let recursive = config
        .get("recursive")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let mut cleaned = 0_usize;
    let mut planned = 0_usize;
    for raw in paths {
        let path = PathBuf::from(raw.trim());
        if !path.is_absolute() || path.components().any(|component| matches!(component, std::path::Component::ParentDir)) {
            return FileCleanupOutcome { success: false, message: format!("file_cleanup path must be clean absolute path: {}", path.display()) };
        }
        if !is_under_allowed_root(&path, &allowed_roots) {
            return FileCleanupOutcome { success: false, message: format!("file_cleanup path is outside allowedRoots: {}", path.display()) };
        }
        planned = planned.saturating_add(1);
        if dry_run {
            continue;
        }
        match tokio::fs::metadata(&path).await {
            Ok(metadata) if metadata.is_dir() && recursive => {
                if let Err(error) = tokio::fs::remove_dir_all(&path).await {
                    return FileCleanupOutcome { success: false, message: format!("file_cleanup failed to remove directory {}: {error}", path.display()) };
                }
                cleaned = cleaned.saturating_add(1);
            }
            Ok(metadata) if metadata.is_file() => {
                if let Err(error) = tokio::fs::remove_file(&path).await {
                    return FileCleanupOutcome { success: false, message: format!("file_cleanup failed to remove file {}: {error}", path.display()) };
                }
                cleaned = cleaned.saturating_add(1);
            }
            Ok(metadata) if metadata.is_dir() => {
                return FileCleanupOutcome { success: false, message: format!("file_cleanup refusing directory without recursive=true: {}", path.display()) };
            }
            Ok(_) => {
                return FileCleanupOutcome { success: false, message: format!("file_cleanup only supports regular files/directories: {}", path.display()) };
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return FileCleanupOutcome { success: false, message: format!("file_cleanup cannot inspect {}: {error}", path.display()) };
            }
        }
    }
    FileCleanupOutcome {
        success: true,
        message: if dry_run {
            format!("file_cleanup dry-run planned {planned} path(s)")
        } else {
            format!("file_cleanup removed {cleaned} of {planned} path(s)")
        },
    }
}

fn is_under_allowed_root(path: &Path, allowed_roots: &[PathBuf]) -> bool {
    allowed_roots.iter().any(|root| path.starts_with(root))
}

#[derive(Debug, Clone)]
struct HttpProcessorOutcome {
    success: bool,
    message: String,
}

async fn execute_http_processor(config: &serde_json::Value) -> HttpProcessorOutcome {
    let Some(url) = config
        .get("url")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return HttpProcessorOutcome {
            success: false,
            message: "http node requires config.url".to_owned(),
        };
    };
    let method = config
        .get("method")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("GET")
        .to_ascii_uppercase();
    let allowed_hosts = string_array(config.get("allowedHosts"));
    let parsed = match url::Url::parse(url) {
        Ok(parsed) => parsed,
        Err(error) => {
            return HttpProcessorOutcome {
                success: false,
                message: format!("invalid http url: {error}"),
            };
        }
    };
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return HttpProcessorOutcome {
            success: false,
            message: "http node only allows http/https urls".to_owned(),
        };
    }
    let Some(host) = parsed.host_str() else {
        return HttpProcessorOutcome {
            success: false,
            message: "http node url must include host".to_owned(),
        };
    };
    if is_private_or_loopback_host(host) {
        return HttpProcessorOutcome {
            success: false,
            message: "http node rejects loopback/private IP hosts by default".to_owned(),
        };
    }
    if !allowed_hosts.is_empty()
        && !allowed_hosts
            .iter()
            .any(|allowed| host_matches(host, allowed))
    {
        return HttpProcessorOutcome {
            success: false,
            message: format!("http host {host} is not in allowedHosts"),
        };
    }
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return HttpProcessorOutcome {
                success: false,
                message: format!("http client build failed: {error}"),
            };
        }
    };
    let req_method = method.parse().unwrap_or(reqwest::Method::GET);
    let mut request = client.request(req_method, parsed);
    if let Some(body) = config.get("body") {
        request = request.json(body);
    }
    match request.send().await {
        Ok(response) => {
            let status = response.status();
            HttpProcessorOutcome {
                success: status.is_success(),
                message: format!("http {} {url} -> {}", method, status.as_u16()),
            }
        }
        Err(error) => HttpProcessorOutcome {
            success: false,
            message: format!("http request failed: {error}"),
        },
    }
}

fn string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn host_matches(host: &str, allowed: &str) -> bool {
    host.eq_ignore_ascii_case(allowed)
        || allowed
            .strip_prefix("*.")
            .is_some_and(|suffix| host.ends_with(suffix))
}

fn is_private_or_loopback_host(host: &str) -> bool {
    host.parse::<IpAddr>().is_ok_and(|ip| {
        ip.is_loopback()
            || ip.is_unspecified()
            || match ip {
                IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
                IpAddr::V6(v6) => v6.is_unique_local() || v6.is_unicast_link_local(),
            }
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::cluster::{ClusterMode, ClusterRole, ClusterStatus, StaticCoordinator};
    use tikee_core::{ExecutionMode, InstanceStatus, TriggerType};
    use tikee_proto::worker::v1::{
        RegisterWorker, ScriptRunnerCapability, SdkProcessorCapability, WorkerCapabilities,
        server_message, task_processor_binding,
    };
    use tikee_storage::{
        AuditLogRepository, CreateJob, CreateJobInstance, JobInstanceAttemptRepository,
        JobInstanceRepository, JobRepository, ScriptRepository, ScriptSummary,
        ScriptVersionSummary, WorkflowRepository, connect_and_migrate,
    };
    use tokio::sync::mpsc;

    use super::{
        DispatchTaskBuild, JobExecutor, ScriptGovernanceFailure, WorkerRegistry,
        build_dispatch_task, dispatch_once, dispatch_once_if_owner, execute_file_cleanup_processor,
        execute_grpc_processor, execute_sql_processor, script_is_dispatchable,
        script_version_is_dispatchable,
    };

    fn sdk_capabilities(processor_name: &str) -> Option<WorkerCapabilities> {
        Some(WorkerCapabilities {
            sdk_processors: vec![SdkProcessorCapability {
                name: processor_name.to_owned(),
            }],
            ..WorkerCapabilities::default()
        })
    }

    fn script_capabilities(language: &str) -> Option<WorkerCapabilities> {
        Some(WorkerCapabilities {
            script_runners: vec![ScriptRunnerCapability {
                language: language.to_owned(),
                sandbox_backend: "auto".to_owned(),
            }],
            ..WorkerCapabilities::default()
        })
    }

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
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("billing.manual".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
                    structured_capabilities: sdk_capabilities("billing.manual"),
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
            &audit,
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
    async fn dispatch_once_backoffs_unmatched_queue_item_without_starving_later_work() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let logs = tikee_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);

        let blocked_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "offline".to_owned(),
                name: "blocked".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("demo.blocked".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
            })
            .await
            .unwrap_or_else(|error| panic!("blocked job should be created: {error}"));
        let blocked_instance = instances
            .create_pending(CreateJobInstance {
                job_id: blocked_job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("blocked instance should be created: {error}"))
            .unwrap_or_else(|| panic!("blocked job should exist"));

        let valid_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("demo.echo".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
            })
            .await
            .unwrap_or_else(|error| panic!("valid job should be created: {error}"));
        let valid_instance = instances
            .create_pending(CreateJobInstance {
                job_id: valid_job.id.clone(),
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("valid instance should be created: {error}"))
            .unwrap_or_else(|| panic!("valid job should exist"));

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
                    structured_capabilities: sdk_capabilities("demo.echo"),
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
            &audit,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("matched worker should receive later valid dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                assert_eq!(task.instance_id, valid_instance.id);
                assert_eq!(task.processor_name, "demo.echo");
            }
            other => panic!("unexpected server message: {other:?}"),
        }

        let blocked = instances
            .get(&blocked_instance.id)
            .await
            .unwrap_or_else(|error| panic!("blocked instance should load: {error}"))
            .unwrap_or_else(|| panic!("blocked instance should exist"));
        assert_eq!(blocked.status, InstanceStatus::Pending);
        let valid = instances
            .get(&valid_instance.id)
            .await
            .unwrap_or_else(|error| panic!("valid instance should load: {error}"))
            .unwrap_or_else(|| panic!("valid instance should exist"));
        assert_eq!(valid.status, InstanceStatus::Running);

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        let blocked_queue = overview
            .items
            .iter()
            .find(|item| item.job_instance_id.as_deref() == Some(blocked_instance.id.as_str()))
            .unwrap_or_else(|| panic!("blocked queue item should exist"));
        assert_eq!(blocked_queue.status, "pending");
        assert!(blocked_queue.run_after > blocked_queue.created_at);
    }

    #[tokio::test]
    async fn dispatch_once_fails_script_instance_when_no_script_worker_capability_exists() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let logs = tikee_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
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
            .publish_version(&script.id, version.version_number, None, None)
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "shell job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: Some(script.id),
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &workflows,
            &scripts,
            &logs,
            &audit,
            &WorkerRegistry::default(),
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let updated = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should load: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(updated.status, InstanceStatus::Failed);
        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        let queue_item = overview
            .items
            .iter()
            .find(|item| item.job_instance_id.as_deref() == Some(instance.id.as_str()))
            .unwrap_or_else(|| panic!("queue item should exist"));
        assert_eq!(queue_item.status, "failed");
        let instance_logs = logs
            .list_by_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("logs should load: {error}"));
        assert!(
            instance_logs
                .iter()
                .any(|log| { log.message.contains("script_no_eligible_worker_capability") })
        );
    }

    #[tokio::test]
    async fn dispatch_script_uses_unified_script_worker_capability() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let logs = tikee_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(tikee_storage::CreateScript {
                name: "python example".to_owned(),
                language: "python".to_owned(),
                version: "1.0.0".to_owned(),
                content: "print('ok')".to_owned(),
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
            .publish_version(&script.id, version.version_number, None, None)
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "python job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: Some(script.id.clone()),
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
        let registry = WorkerRegistry::default();
        let (tx, mut rx) = mpsc::channel(1);
        registry
            .register(
                RegisterWorker {
                    client_instance_id: "script-worker".to_owned(),
                    app: "billing".to_owned(),
                    namespace: "default".to_owned(),
                    cluster: "local".to_owned(),
                    region: "local".to_owned(),
                    capabilities: Vec::new(),
                    structured_capabilities: script_capabilities("python"),
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
            &audit,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let message = rx
            .recv()
            .await
            .unwrap_or_else(|| panic!("script worker should receive dispatch"))
            .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"));
        match message.kind {
            Some(server_message::Kind::DispatchTask(task)) => {
                let binding = task
                    .processor_binding
                    .unwrap_or_else(|| panic!("script binding expected"));
                match binding.kind {
                    Some(task_processor_binding::Kind::Script(script_binding)) => {
                        assert_eq!(script_binding.script_id, script.id);
                        assert_eq!(script_binding.language, "python");
                    }
                    other => panic!("unexpected binding: {other:?}"),
                }
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
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
                    structured_capabilities: sdk_capabilities("manual"),
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
                    structured_capabilities: sdk_capabilities("manual"),
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
            &audit,
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
                assert_eq!(task.processor_name, job.name);
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
    async fn dispatch_once_closes_terminal_instance_queue_item() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let attempts = JobInstanceAttemptRepository::new(db.clone());
        let workflows = WorkflowRepository::new(db.clone());
        let logs = tikee_storage::JobInstanceLogRepository::new(db.clone());
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "already-done".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
        instances
            .update_status(&instance.id, InstanceStatus::Succeeded)
            .await
            .unwrap_or_else(|error| panic!("instance should be terminal: {error}"));
        let registry = WorkerRegistry::default();

        dispatch_once(
            &jobs,
            &instances,
            &attempts,
            &workflows,
            &scripts,
            &logs,
            &audit,
            &registry,
            "test-fence",
        )
        .await
        .unwrap_or_else(|error| panic!("dispatch should run: {error}"));

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue should load: {error}"));
        assert_eq!(overview.pending, 0);
        assert_eq!(overview.running, 0);
        assert_eq!(overview.done, 1);
        assert_eq!(
            overview.items[0].job_instance_id.as_deref(),
            Some(instance.id.as_str())
        );
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
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "follower-dispatch".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
            &jobs, &instances, &attempts, &workflows, &scripts, &logs, &audit, &registry, &follower,
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
        let audit = AuditLogRepository::new(db.clone());
        let scripts = ScriptRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "manual".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: Some("job.default".to_owned()),
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
                    structured_capabilities: sdk_capabilities("workflow.override"),
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
            &audit,
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
        let audit = AuditLogRepository::new(db.clone());
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
            .publish_version(&script.id, version.version_number, None, None)
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
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "wasm job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: Some(script.id.clone()),
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
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
                    structured_capabilities: script_capabilities("wasm"),
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
            &audit,
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
                assert_eq!(task.processor_name, script.id);
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
            .publish_version(&script.id, version.version_number, None, None)
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));

        let task = match build_dispatch_task(
            &scripts,
            "instance-shell".to_owned(),
            "job-shell".to_owned(),
            JobExecutor::Script {
                script_id: script.id.clone(),
            },
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
                assert_eq!(script_binding.sandbox_backend, "auto");
            }
            other => panic!("unexpected binding: {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_copies_verified_release_grants_into_script_binding() {
        let db = connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
        let scripts = ScriptRepository::new(db);

        let script = scripts
            .create_script(tikee_storage::CreateScript {
                name: "shell grants".to_owned(),
                language: "shell".to_owned(),
                version: "1.0.0".to_owned(),
                content: "printf ok".to_owned(),
                created_by: "tester".to_owned(),
                timeout_seconds: Some(5),
                max_memory_bytes: Some(1024 * 1024),
                allow_network: false,
                allowed_env_vars: None,
                policy_json: Some(r#"{"resources":{"timeout_ms":7000,"max_memory_bytes":33554432,"max_output_bytes":4096},"network":{"enabled":false,"allowed_hosts":["policy.example.invalid"]},"filesystem":{"read_only_paths":["/policy/read"],"writable_paths":["/policy/write"]},"secrets":{"refs":["secret:policy"]},"env_vars":["SAFE_ENV"]}"#.to_owned()),
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
            .publish_version(
                &script.id,
                version.version_number,
                None,
                Some(tikee_storage::VerifiedScriptReleaseGrants {
                    grants: tikee_core::ScriptReleaseGrantSet {
                        url: vec!["api.example.com".to_owned()],
                        file_read: vec!["/data/input".to_owned()],
                        file_write: vec!["/data/output".to_owned()],
                        secret: vec!["secret:db-readonly".to_owned()],
                    },
                    verified_by: "grant-verifier".to_owned(),
                }),
            )
            .await
            .unwrap_or_else(|error| panic!("script should publish: {error}"))
            .unwrap_or_else(|| panic!("script should exist"));

        let task = match build_dispatch_task(
            &scripts,
            "instance-shell".to_owned(),
            "job-shell".to_owned(),
            JobExecutor::Script {
                script_id: script.id.clone(),
            },
        )
        .await
        .unwrap_or_else(|error| panic!("task build should not error: {error}"))
        {
            DispatchTaskBuild::Built(task) => task,
            DispatchTaskBuild::Rejected(failure) => {
                panic!("released grant script should dispatch: {failure:?}")
            }
        };

        let binding = task
            .processor_binding
            .unwrap_or_else(|| panic!("script binding expected"));
        match binding.kind {
            Some(task_processor_binding::Kind::Script(script_binding)) => {
                assert!(script_binding.allow_network);
                assert_eq!(
                    script_binding.allowed_network_hosts,
                    vec!["api.example.com"]
                );
                assert_eq!(script_binding.read_only_paths, vec!["/data/input"]);
                assert_eq!(script_binding.writable_paths, vec!["/data/output"]);
                assert_eq!(script_binding.secret_refs, vec!["secret:db-readonly"]);
                assert_eq!(script_binding.allowed_env_vars, vec!["SAFE_ENV"]);
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
            JobExecutor::Script {
                script_id: script.id.clone(),
            },
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
            release_signature: None,
            release_grants: None,
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
    #[tokio::test]
    async fn file_cleanup_processor_defaults_to_dry_run_and_requires_allowed_roots() {
        let outcome = execute_file_cleanup_processor(&serde_json::json!({
            "paths": ["/tmp/tikee-cleanup-demo"]
        }))
        .await;
        assert!(!outcome.success);
        assert!(outcome.message.contains("allowedRoots"));

        let outcome = execute_file_cleanup_processor(&serde_json::json!({
            "paths": ["/tmp/tikee-cleanup-demo"],
            "allowedRoots": ["/tmp"]
        }))
        .await;
        assert!(outcome.success);
        assert!(outcome.message.contains("dry-run"));
    }

    #[tokio::test]
    async fn file_cleanup_processor_deletes_only_under_allowed_roots() {
        let temp_root = std::env::temp_dir().join(format!("tikee-cleanup-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_root)
            .await
            .unwrap_or_else(|error| panic!("temp root should be created: {error}"));
        let target = temp_root.join("stale.log");
        tokio::fs::write(&target, b"stale")
            .await
            .unwrap_or_else(|error| panic!("target file should be written: {error}"));

        let rejected = execute_file_cleanup_processor(&serde_json::json!({
            "paths": [target.display().to_string()],
            "allowedRoots": ["/var/lib/tikee"],
            "dryRun": false
        }))
        .await;
        assert!(!rejected.success);
        assert!(tokio::fs::metadata(&target).await.is_ok());

        let deleted = execute_file_cleanup_processor(&serde_json::json!({
            "paths": [target.display().to_string()],
            "allowedRoots": [temp_root.display().to_string()],
            "dryRun": false
        }))
        .await;
        assert!(deleted.success, "{}", deleted.message);
        assert!(tokio::fs::metadata(&target).await.is_err());
        let _ = tokio::fs::remove_dir_all(&temp_root).await;
    }

    #[tokio::test]
    async fn grpc_processor_fails_closed_without_required_fields_and_private_hosts() {
        let missing = execute_grpc_processor(&serde_json::json!({})).await;
        assert!(!missing.success);
        assert!(missing.message.contains("endpoint"));

        let private = execute_grpc_processor(&serde_json::json!({
            "endpoint": "http://127.0.0.1:50051",
            "service": "demo.Echo",
            "method": "Ping"
        }))
        .await;
        assert!(!private.success);
        assert!(private.message.contains("private"));
    }

    #[tokio::test]
    async fn sql_processor_enforces_allowlist_and_read_only_default() {
        let missing_allowlist = execute_sql_processor(&serde_json::json!({
            "databaseUrl": "sqlite::memory:",
            "sql": "SELECT 1"
        }))
        .await;
        assert!(!missing_allowlist.success);
        assert!(missing_allowlist.message.contains("allowedDatabaseUrls"));

        let write_rejected = execute_sql_processor(&serde_json::json!({
            "databaseUrl": "sqlite::memory:",
            "allowedDatabaseUrls": ["sqlite::memory:"],
            "sql": "DELETE FROM demo",
            "dryRun": false
        }))
        .await;
        assert!(!write_rejected.success);
        assert!(write_rejected.message.contains("readOnly"));

        let dry_run = execute_sql_processor(&serde_json::json!({
            "databaseUrl": "sqlite::memory:",
            "allowedDatabaseUrls": ["sqlite::memory:"],
            "sql": "SELECT 1"
        }))
        .await;
        assert!(dry_run.success);
        assert!(dry_run.message.contains("dry-run"));
    }

    #[tokio::test]
    async fn sql_processor_executes_sqlite_read_only_query() {
        let outcome = execute_sql_processor(&serde_json::json!({
            "databaseUrl": "sqlite::memory:",
            "allowedDatabaseUrls": ["sqlite::memory:"],
            "sql": "SELECT 1",
            "dryRun": false
        }))
        .await;
        assert!(outcome.success, "{}", outcome.message);
        assert!(outcome.message.contains("1 row"));
    }

}
