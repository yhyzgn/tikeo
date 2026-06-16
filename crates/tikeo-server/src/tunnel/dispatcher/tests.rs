use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::cluster::{ClusterMode, ClusterRole, ClusterStatus, StaticCoordinator};
use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};
use tikeo_proto::worker::v1::{
    DispatchTask, RegisterWorker, ScriptRunnerCapability, SdkProcessorCapability,
    WorkerCapabilities, server_message, task_processor_binding,
};
use tikeo_storage::{
    AuditLogRepository, CreateJob, CreateJobInstance, JobInstanceAttemptRepository,
    JobInstanceRepository, JobRepository, JobRetryPolicy, NotificationChannelRepository,
    NotificationDeliveryAttemptRepository, NotificationMessageRepository,
    NotificationPolicyRepository, ScriptRepository, ScriptSummary, ScriptVersionSummary,
    WorkflowRepository, connect_and_migrate,
};
use tokio::sync::mpsc;

use super::{
    DispatchTaskBuild, JobExecutor, ScriptGovernanceFailure, WorkerRegistry, build_dispatch_task,
    complete_builtin_processor_outcome, dispatch_once, dispatch_once_if_owner,
    execute_file_cleanup_processor, execute_grpc_processor, execute_http_processor,
    execute_sql_processor, script_is_dispatchable, script_version_is_dispatchable,
};

fn sdk_capabilities(processor_name: &str) -> WorkerCapabilities {
    WorkerCapabilities {
        sdk_processors: vec![SdkProcessorCapability {
            name: processor_name.to_owned(),
        }],
        ..WorkerCapabilities::default()
    }
}

fn script_capabilities(language: &str) -> WorkerCapabilities {
    WorkerCapabilities {
        script_runners: vec![ScriptRunnerCapability {
            language: language.to_owned(),
            sandbox_backend: "auto".to_owned(),
        }],
        ..WorkerCapabilities::default()
    }
}

fn notification_center(jobs: &JobRepository) -> crate::notification::NotificationCenter {
    let db = jobs.db();
    crate::notification::NotificationCenter::new(
        NotificationChannelRepository::new(db.clone()),
        NotificationPolicyRepository::new(db.clone()),
        NotificationMessageRepository::new(db.clone()),
        NotificationDeliveryAttemptRepository::new(db.clone()),
        tikeo_storage::NotificationTemplateRepository::new(db),
        jobs.clone(),
    )
}

include!("tests/part_01.rs");
include!("tests/part_02.rs");
