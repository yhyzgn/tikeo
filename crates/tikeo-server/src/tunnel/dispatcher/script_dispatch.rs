use tikeo_core::{ScriptExecutionPolicy, ScriptLanguage, ScriptPolicyError, ScriptStatus};
use tikeo_proto::worker::v1::{DispatchTask, task_processor_binding};
use tikeo_storage::{
    AppendJobInstanceLog, AuditLogRepository, ScriptRepository, ScriptSummary, ScriptVersionSummary,
};
use tracing::warn;

use super::script_binding::{
    parse_script_language, script_policy, script_processor_binding, script_version_to_wasm_spec,
    wasm_processor_binding,
};
use crate::tunnel::{capability::WorkerRequirement, governance};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DispatchTaskBuild {
    Built(DispatchTask),
    Rejected(ScriptGovernanceFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ScriptGovernanceFailure {
    MissingScript,
    NotApproved,
    MissingReleasePointer,
    MissingReleasedVersion,
    UnsupportedLanguage,
    PolicyRejected(String),
    NoEligibleWorkerCapability(String),
}

impl ScriptGovernanceFailure {
    pub(super) const fn code(&self) -> &'static str {
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

    pub(super) fn message(&self) -> String {
        match self {
            Self::MissingScript => {
                "script governance rejected dispatch: script definition is missing".to_owned()
            }
            Self::NotApproved => {
                "script governance rejected dispatch: script is not approved".to_owned()
            }
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

pub(super) async fn append_script_governance_log(
    logs: &tikeo_storage::JobInstanceLogRepository,
    audit: &AuditLogRepository,
    instance_id: &str,
    failure: &ScriptGovernanceFailure,
) -> Result<(), tikeo_storage::DbErr> {
    let failure_class = failure.code();
    let message = failure.message();
    let payload = governance::script_governance_payload(failure_class, &message);
    let _ = logs
        .append(AppendJobInstanceLog {
            instance_id: instance_id.to_owned(),
            worker_id: "tikeo-dispatcher".to_owned(),
            level: "warn".to_owned(),
            message: payload.to_string(),
            sequence: 0,
        })
        .await?;
    governance::materialize_script_governance_audit(
        audit,
        "tikeo-dispatcher",
        instance_id,
        failure_class,
        &message,
    )
    .await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum JobExecutor {
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

pub(super) async fn build_dispatch_task(
    scripts: &ScriptRepository,
    instance_id: String,
    job_id: String,
    executor: JobExecutor,
) -> Result<DispatchTaskBuild, tikeo_storage::DbErr> {
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

pub(super) fn required_task_requirement_for_executor(
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
        JobExecutor::SdkProcessor { processor_name, .. } => {
            Some(WorkerRequirement::NormalProcessor {
                name: processor_name.trim().to_owned(),
            })
        }
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

pub(super) fn script_is_dispatchable(script: &ScriptSummary) -> bool {
    script.status == ScriptStatus::Approved.as_str()
        && parse_script_language(&script.language).is_some()
}

#[cfg(test)]
pub(super) fn script_version_is_dispatchable(version: &ScriptVersionSummary) -> bool {
    validate_script_version_dispatchable(version, None).is_ok()
}

fn validate_script_version_dispatchable(
    version: &ScriptVersionSummary,
    release_grants: Option<&tikeo_storage::ScriptReleaseGrantEvidenceSummary>,
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
            | ScriptLanguage::Php
            | ScriptLanguage::Groovy
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

fn validate_script_policy_for_dispatch(
    policy: &ScriptExecutionPolicy,
    release_grants: Option<&tikeo_storage::ScriptReleaseGrantEvidenceSummary>,
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
