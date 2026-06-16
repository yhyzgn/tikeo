use tikeo_core::InstanceStatus;
use tikeo_proto::worker::v1::{
    RegisterWorker, SubscribeTaskLogsRequest, TaskLog, WorkerMessage, server_message,
    worker_message,
};
use tikeo_storage::{
    AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
    JobInstanceRepository, JobRepository, NotificationChannelRepository,
    NotificationDeliveryAttemptRepository, NotificationMessageRepository,
    NotificationPolicyRepository, WorkflowRepository, connect_and_migrate,
};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use super::{
    TaskLogBroadcaster, WorkerMessageContext, WorkerRegistry, handle_task_result,
    handle_worker_message,
};

fn standalone_cluster() -> crate::cluster::SharedClusterCoordinator {
    crate::cluster::StandaloneCoordinator::shared("standalone")
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

#[tokio::test]
async fn register_message_updates_registry_and_acknowledges_worker() {
    let registry = WorkerRegistry::default();
    let instances = instances().await;
    let jobs = jobs().await;
    let logs = logs().await;
    let audit = audit().await;
    let (tx, mut rx) = mpsc::channel(1);

    let attempts = attempts().await;

    let workflows = workflows().await;
    let log_broadcaster = TaskLogBroadcaster::default();
    let notifications = notification_center(&jobs);
    let cluster = standalone_cluster();
    let context = WorkerMessageContext {
        registry: &registry,
        instances: &instances,
        jobs: &jobs,
        logs: &logs,
        attempts: &attempts,
        workflows: &workflows,
        audit: &audit,
        notifications: &notifications,
        log_broadcaster: &log_broadcaster,
        cluster: &cluster,
        tx: &tx,
        outbound: &tx,
    };

    handle_worker_message(
        &context,
        WorkerMessage {
            kind: Some(worker_message::Kind::Register(RegisterWorker {
                client_instance_id: "worker-1".to_owned(),
                app: "billing".to_owned(),
                namespace: "finance".to_owned(),
                cluster: "prod".to_owned(),
                region: "cn".to_owned(),
                capabilities: Vec::new(),
                structured_capabilities: None,
                election: None,
                labels: std::collections::HashMap::default(),
            })),
        },
    )
    .await
    .unwrap_or_else(|error| panic!("ack should send: {error}"));

    let ack = rx
        .recv()
        .await
        .unwrap_or_else(|| panic!("ack should exist"))
        .unwrap_or_else(|error| panic!("ack should be ok: {error}"));

    match ack.kind {
        Some(server_message::Kind::Registered(registered)) => {
            assert!(registered.worker_id.starts_with("wrk-"));
        }
        other => panic!("unexpected ack: {other:?}"),
    }

    let registered_id = registry
        .worker_ids()
        .await
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("registered worker id should exist"));
    assert!(registry.get(&registered_id).await.is_some());
}

#[tokio::test]
async fn register_message_is_rejected_on_raft_follower() {
    use crate::cluster::{ClusterMode, ClusterRole, ClusterStatus, StaticCoordinator};

    let registry = WorkerRegistry::default();
    let instances = instances().await;
    let jobs = jobs().await;
    let logs = logs().await;
    let audit = audit().await;
    let attempts = attempts().await;
    let workflows = workflows().await;
    let log_broadcaster = TaskLogBroadcaster::default();
    let notifications = notification_center(&jobs);
    let (tx, mut rx) = mpsc::channel(1);
    let follower = StaticCoordinator::shared(ClusterStatus {
        mode: ClusterMode::Raft,
        role: ClusterRole::Follower,
        node_id: "tikeo-server-1".to_owned(),
        nodes: 3,
        can_schedule: false,
        leader_fencing_token: None,
        detail: "test follower".to_owned(),
    });
    let context = WorkerMessageContext {
        registry: &registry,
        instances: &instances,
        jobs: &jobs,
        logs: &logs,
        attempts: &attempts,
        workflows: &workflows,
        audit: &audit,
        notifications: &notifications,
        log_broadcaster: &log_broadcaster,
        cluster: &follower,
        tx: &tx,
        outbound: &tx,
    };

    handle_worker_message(
        &context,
        WorkerMessage {
            kind: Some(worker_message::Kind::Register(RegisterWorker {
                client_instance_id: "worker-follower".to_owned(),
                app: "billing".to_owned(),
                namespace: "finance".to_owned(),
                cluster: "prod".to_owned(),
                region: "cn".to_owned(),
                capabilities: Vec::new(),
                structured_capabilities: None,
                election: None,
                labels: std::collections::HashMap::default(),
            })),
        },
    )
    .await
    .unwrap_or_else(|error| panic!("rejection should be sent: {error}"));

    let rejection = rx
        .recv()
        .await
        .unwrap_or_else(|| panic!("rejection should exist"))
        .expect_err("follower registration should be rejected");

    assert_eq!(rejection.code(), tonic::Code::FailedPrecondition);
    assert!(
        rejection
            .message()
            .contains("worker tunnel registration requires raft scheduling leader")
    );
    assert!(registry.worker_ids().await.is_empty());
}

#[tokio::test]
async fn task_result_with_wrong_assignment_token_is_rejected() {
    use tikeo_core::{ExecutionMode, TriggerType};
    use tikeo_proto::worker::v1::{RegisterWorker, TaskResult};
    use tikeo_storage::{CreateJob, CreateJobInstance, JobRepository};

    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let logs = JobInstanceLogRepository::new(db.clone());
    let attempts = JobInstanceAttemptRepository::new(db.clone());
    let workflows = WorkflowRepository::new(db.clone());
    let audit = AuditLogRepository::new(db.clone());
    let job = jobs
        .create_job(CreateJob {
            created_by: None,
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "assign-token".to_owned(),
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
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("job should create: {error}"));
    let instance = instances
        .create_pending(CreateJobInstance {
            job_id: job.id,
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Single,
        })
        .await
        .unwrap_or_else(|error| panic!("instance should create: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    let (outbound, _rx) = mpsc::channel(8);
    let registry = WorkerRegistry::default();
    let worker = registry
        .register(
            RegisterWorker {
                client_instance_id: "assign-worker".to_owned(),
                app: "billing".to_owned(),
                namespace: "default".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                capabilities: Vec::new(),
                structured_capabilities: None,
                election: None,
                labels: std::collections::HashMap::default(),
            },
            outbound,
        )
        .await;
    let (tx, _events) = mpsc::channel(8);
    let broadcaster = TaskLogBroadcaster::default();
    let notifications = notification_center(&jobs);
    let cluster = standalone_cluster();
    let context = WorkerMessageContext {
        registry: &registry,
        instances: &instances,
        jobs: &jobs,
        logs: &logs,
        attempts: &attempts,
        workflows: &workflows,
        audit: &audit,
        notifications: &notifications,
        log_broadcaster: &broadcaster,
        cluster: &cluster,
        tx: &tx,
        outbound: &tx,
    };

    handle_task_result(
        &context,
        TaskResult {
            worker_id: worker.worker_id,
            instance_id: instance.id.clone(),
            success: true,
            message: "ok".to_owned(),
            assignment_token: "wrong".to_owned(),
        },
    )
    .await;

    let unchanged = instances
        .get(&instance.id)
        .await
        .unwrap_or_else(|error| panic!("instance should load: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    assert_eq!(unchanged.status, InstanceStatus::Pending);
}

#[tokio::test]
async fn broadcast_task_result_persists_per_worker_attempt_result() {
    use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};
    use tikeo_proto::worker::v1::{DispatchTask, RegisterWorker, TaskResult, server_message};
    use tikeo_storage::{
        CreateJob, CreateJobInstance, CreateNotificationChannel, CreateNotificationPolicy,
        JobRepository, NotificationMessageFilters,
    };

    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let logs = JobInstanceLogRepository::new(db.clone());
    let attempts = JobInstanceAttemptRepository::new(db.clone());
    let workflows = WorkflowRepository::new(db.clone());
    let audit = AuditLogRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let delivery_attempts = NotificationDeliveryAttemptRepository::new(db);
    let job = jobs
        .create_job(CreateJob {
            created_by: None,
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "broadcast-result".to_owned(),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "fire_once".to_owned(),
            schedule_start_at: None,
            schedule_end_at: None,
            schedule_calendar_json: None,
            processor_name: Some("demo.broadcast".to_owned()),
            processor_type: None,
            script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("job should create: {error}"));
    let instance = instances
        .create_pending(CreateJobInstance {
            job_id: job.id.clone(),
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Broadcast,
        })
        .await
        .unwrap_or_else(|error| panic!("instance should create: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "job".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "broadcast status".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({"url":"https://hooks.example.com/broadcast-secret"})
                .to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));
    policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "job".to_owned(),
            owner_id: Some(job.id.clone()),
            name: "broadcast terminal".to_owned(),
            event_family: "job_instance".to_owned(),
            event_filter_json: serde_json::json!({"statuses":["succeeded"]}).to_string(),
            channel_refs_json: serde_json::json!([{"channelId": channel.id}]).to_string(),
            template_ref: None,
            severity: "info".to_owned(),
            enabled: true,
            dedupe_seconds: 300,
        })
        .await
        .unwrap_or_else(|error| panic!("policy should create: {error}"));

    let (outbound, mut rx) = mpsc::channel(8);
    let registry = WorkerRegistry::default();
    let worker = registry
        .register(
            RegisterWorker {
                client_instance_id: "broadcast-worker".to_owned(),
                app: "billing".to_owned(),
                namespace: "default".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                capabilities: Vec::new(),
                structured_capabilities: None,
                election: None,
                labels: std::collections::HashMap::default(),
            },
            outbound,
        )
        .await;
    attempts
        .create_pending_for_workers(&instance.id, std::slice::from_ref(&worker.worker_id))
        .await
        .unwrap_or_else(|error| panic!("attempt should create: {error}"));
    registry
        .dispatch_to_worker(
            &worker.worker_id,
            DispatchTask {
                instance_id: instance.id.clone(),
                job_id: job.id,
                payload: Vec::new(),
                processor_name: "demo.broadcast".to_owned(),
                processor_binding: None,
                assignment_token: String::new(),
            },
        )
        .await
        .unwrap_or_else(|| panic!("task should dispatch"));
    let token = match rx
        .recv()
        .await
        .unwrap_or_else(|| panic!("dispatch should arrive"))
        .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"))
        .kind
    {
        Some(server_message::Kind::DispatchTask(task)) => task.assignment_token,
        other => panic!("unexpected server message: {other:?}"),
    };
    persist_assignment_token_for_test(&attempts, &instance.id, &worker.worker_id, &token).await;
    let (tx, _events) = mpsc::channel(8);
    let broadcaster = TaskLogBroadcaster::default();
    let templates = tikeo_storage::NotificationTemplateRepository::new(channels.db());
    let notifications = crate::notification::NotificationCenter::new(
        channels,
        policies,
        messages.clone(),
        delivery_attempts,
        templates,
        jobs.clone(),
    );
    let cluster = standalone_cluster();
    let context = WorkerMessageContext {
        registry: &registry,
        instances: &instances,
        jobs: &jobs,
        logs: &logs,
        attempts: &attempts,
        workflows: &workflows,
        audit: &audit,
        notifications: &notifications,
        log_broadcaster: &broadcaster,
        cluster: &cluster,
        tx: &tx,
        outbound: &tx,
    };

    handle_task_result(
        &context,
        TaskResult {
            worker_id: worker.worker_id.clone(),
            instance_id: instance.id.clone(),
            success: true,
            message: "broadcast ok".to_owned(),
            assignment_token: token,
        },
    )
    .await;

    let persisted_attempts = attempts
        .list_by_instance(&instance.id)
        .await
        .unwrap_or_else(|error| panic!("attempts should load: {error}"));
    let result = persisted_attempts[0]
        .result
        .clone()
        .unwrap_or_else(|| panic!("attempt result should persist"));
    assert!(result.success);
    assert_eq!(result.worker_id, worker.worker_id);
    assert_eq!(result.message, "broadcast ok");

    let parent = instances
        .get(&instance.id)
        .await
        .unwrap_or_else(|error| panic!("instance should load: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    assert_eq!(parent.status, InstanceStatus::Succeeded);
    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("job_instance".to_owned()),
            source_id: Some(instance.id),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    assert!(
        timeline
            .iter()
            .any(|message| message.event_type == "job_instance.succeeded")
    );
}

#[tokio::test]
async fn failed_single_task_result_schedules_retry_and_logs_result() {
    use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};
    use tikeo_proto::worker::v1::{DispatchTask, RegisterWorker, TaskResult, server_message};
    use tikeo_storage::{CreateJob, CreateJobInstance, JobRepository, JobRetryPolicy};

    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let logs = JobInstanceLogRepository::new(db.clone());
    let attempts = JobInstanceAttemptRepository::new(db.clone());
    let workflows = WorkflowRepository::new(db.clone());
    let audit = AuditLogRepository::new(db);
    let job = jobs
        .create_job(CreateJob {
            created_by: None,
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "retry-runtime".to_owned(),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "fire_once".to_owned(),
            schedule_start_at: None,
            schedule_end_at: None,
            schedule_calendar_json: None,
            processor_name: Some("demo.retry".to_owned()),
            processor_type: None,
            script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            retry_policy: Some(JobRetryPolicy {
                enabled: true,
                max_attempts: 3,
                initial_delay_seconds: 5,
                backoff_multiplier: 2,
                max_delay_seconds: 60,
            }),
        })
        .await
        .unwrap_or_else(|error| panic!("job should create: {error}"));
    let instance = instances
        .create_pending(CreateJobInstance {
            job_id: job.id.clone(),
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Single,
        })
        .await
        .unwrap_or_else(|error| panic!("instance should create: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    let claim = workflows
        .claim_next_job_queue_item("server-a", 30)
        .await
        .unwrap_or_else(|error| panic!("queue should claim: {error}"))
        .unwrap_or_else(|| panic!("queue item should exist"));
    workflows
        .mark_dispatch_queue_running(&claim.item.id, "server-a")
        .await
        .unwrap_or_else(|error| panic!("queue should run: {error}"));
    instances
        .update_status(&instance.id, InstanceStatus::Running)
        .await
        .unwrap_or_else(|error| panic!("instance should run: {error}"));

    let (outbound, mut rx) = mpsc::channel(8);
    let registry = WorkerRegistry::default();
    let worker = registry
        .register(
            RegisterWorker {
                client_instance_id: "retry-worker".to_owned(),
                app: "billing".to_owned(),
                namespace: "default".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                capabilities: Vec::new(),
                structured_capabilities: None,
                election: None,
                labels: std::collections::HashMap::default(),
            },
            outbound,
        )
        .await;
    registry
        .dispatch_to_worker(
            &worker.worker_id,
            DispatchTask {
                instance_id: instance.id.clone(),
                job_id: job.id,
                payload: Vec::new(),
                processor_name: "demo.retry".to_owned(),
                processor_binding: None,
                assignment_token: String::new(),
            },
        )
        .await
        .unwrap_or_else(|| panic!("task should dispatch"));
    let token = match rx
        .recv()
        .await
        .unwrap_or_else(|| panic!("dispatch should arrive"))
        .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"))
        .kind
    {
        Some(server_message::Kind::DispatchTask(task)) => task.assignment_token,
        other => panic!("unexpected server message: {other:?}"),
    };
    persist_assignment_token_for_test(&attempts, &instance.id, &worker.worker_id, &token).await;
    let (tx, _events) = mpsc::channel(8);
    let broadcaster = TaskLogBroadcaster::default();
    let notifications = notification_center(&jobs);
    let cluster = standalone_cluster();
    let context = WorkerMessageContext {
        registry: &registry,
        instances: &instances,
        jobs: &jobs,
        logs: &logs,
        attempts: &attempts,
        workflows: &workflows,
        audit: &audit,
        notifications: &notifications,
        log_broadcaster: &broadcaster,
        cluster: &cluster,
        tx: &tx,
        outbound: &tx,
    };

    handle_task_result(
        &context,
        TaskResult {
            worker_id: worker.worker_id.clone(),
            instance_id: instance.id.clone(),
            success: false,
            message: "runtime failed with exit 2".to_owned(),
            assignment_token: token,
        },
    )
    .await;

    let requeued_instance = instances
        .get(&instance.id)
        .await
        .unwrap_or_else(|error| panic!("instance should load: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    assert_eq!(requeued_instance.status, InstanceStatus::Retrying);
    let result = requeued_instance
        .result
        .unwrap_or_else(|| panic!("result should persist"));
    assert!(!result.success);
    assert_eq!(result.message, "runtime failed with exit 2");

    let requeued = workflows
        .claim_next_job_queue_item("server-b", 30)
        .await
        .unwrap_or_else(|error| panic!("queue should load: {error}"));
    assert!(requeued.is_none(), "retry should wait for backoff");
    let persisted_logs = logs
        .list_by_instance(&instance.id)
        .await
        .unwrap_or_else(|error| panic!("logs should load: {error}"));
    assert!(
        persisted_logs
            .iter()
            .any(|log| log.message.contains("retry scheduled"))
    );
    assert!(
        persisted_logs
            .iter()
            .any(|log| log.message.contains("runtime failed with exit 2"))
    );
}

#[tokio::test]
async fn failed_single_task_result_emits_job_notification_policy() {
    use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};
    use tikeo_proto::worker::v1::{DispatchTask, RegisterWorker, TaskResult, server_message};
    use tikeo_storage::{
        CreateJob, CreateJobInstance, CreateNotificationChannel, CreateNotificationPolicy,
        JobRetryPolicy, NotificationDeliveryAttemptFilters, NotificationMessageFilters,
    };

    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let logs = JobInstanceLogRepository::new(db.clone());
    let attempts = JobInstanceAttemptRepository::new(db.clone());
    let workflows = WorkflowRepository::new(db.clone());
    let audit = AuditLogRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let delivery_attempts = NotificationDeliveryAttemptRepository::new(db.clone());
    let job = jobs
        .create_job(CreateJob {
            created_by: None,
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "notify-runtime".to_owned(),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "fire_once".to_owned(),
            schedule_start_at: None,
            schedule_end_at: None,
            schedule_calendar_json: None,
            processor_name: Some("demo.notify".to_owned()),
            processor_type: None,
            script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            retry_policy: Some(JobRetryPolicy {
                enabled: true,
                max_attempts: 2,
                initial_delay_seconds: 5,
                backoff_multiplier: 2,
                max_delay_seconds: 60,
            }),
        })
        .await
        .unwrap_or_else(|error| panic!("job should create: {error}"));
    let instance = instances
        .create_pending(CreateJobInstance {
            job_id: job.id.clone(),
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Single,
        })
        .await
        .unwrap_or_else(|error| panic!("instance should create: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "ops".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json:
                serde_json::json!({"url":"https://hooks.example.com/services/top-secret-token"})
                    .to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));
    policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "job".to_owned(),
            owner_id: Some(job.id.clone()),
            name: "notify failures".to_owned(),
            event_family: "job_instance".to_owned(),
            event_filter_json: serde_json::json!({"statuses":["failed","retry_exhausted"]})
                .to_string(),
            channel_refs_json: serde_json::json!([{"channelId": channel.id}]).to_string(),
            template_ref: None,
            severity: "critical".to_owned(),
            enabled: true,
            dedupe_seconds: 300,
        })
        .await
        .unwrap_or_else(|error| panic!("policy should create: {error}"));
    let first_claim = workflows
        .claim_next_job_queue_item("server-pre", 30)
        .await
        .unwrap_or_else(|error| panic!("queue should claim first attempt: {error}"))
        .unwrap_or_else(|| panic!("first queue item should exist"));
    workflows
        .mark_dispatch_queue_running(&first_claim.item.id, "server-pre")
        .await
        .unwrap_or_else(|error| panic!("first queue should run: {error}"));
    workflows
        .requeue_dispatch_queue_for_retry(&instance.id, 0)
        .await
        .unwrap_or_else(|error| panic!("first failure should requeue: {error}"))
        .unwrap_or_else(|| panic!("first failure should produce pending retry"));

    let claim = workflows
        .claim_next_job_queue_item("server-a", 30)
        .await
        .unwrap_or_else(|error| panic!("queue should claim: {error}"))
        .unwrap_or_else(|| panic!("queue item should exist"));
    assert_eq!(claim.item.attempt, 2);
    workflows
        .mark_dispatch_queue_running(&claim.item.id, "server-a")
        .await
        .unwrap_or_else(|error| panic!("queue should run: {error}"));
    instances
        .update_status(&instance.id, InstanceStatus::Running)
        .await
        .unwrap_or_else(|error| panic!("instance should run: {error}"));

    let (outbound, mut rx) = mpsc::channel(8);
    let registry = WorkerRegistry::default();
    let worker = registry
        .register(
            RegisterWorker {
                client_instance_id: "notify-worker".to_owned(),
                app: "billing".to_owned(),
                namespace: "default".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                capabilities: Vec::new(),
                structured_capabilities: None,
                election: None,
                labels: std::collections::HashMap::default(),
            },
            outbound,
        )
        .await;
    registry
        .dispatch_to_worker(
            &worker.worker_id,
            DispatchTask {
                instance_id: instance.id.clone(),
                job_id: job.id.clone(),
                payload: Vec::new(),
                processor_name: "demo.notify".to_owned(),
                processor_binding: None,
                assignment_token: String::new(),
            },
        )
        .await
        .unwrap_or_else(|| panic!("task should dispatch"));
    let token = match rx
        .recv()
        .await
        .unwrap_or_else(|| panic!("dispatch should arrive"))
        .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"))
        .kind
    {
        Some(server_message::Kind::DispatchTask(task)) => task.assignment_token,
        other => panic!("unexpected server message: {other:?}"),
    };
    persist_assignment_token_for_test(&attempts, &instance.id, &worker.worker_id, &token).await;
    let (tx, _events) = mpsc::channel(8);
    let broadcaster = TaskLogBroadcaster::default();
    let templates = tikeo_storage::NotificationTemplateRepository::new(channels.db());
    let notifications = crate::notification::NotificationCenter::new(
        channels,
        policies,
        messages.clone(),
        delivery_attempts.clone(),
        templates,
        jobs.clone(),
    );
    let cluster = standalone_cluster();
    let context = WorkerMessageContext {
        registry: &registry,
        instances: &instances,
        jobs: &jobs,
        logs: &logs,
        attempts: &attempts,
        workflows: &workflows,
        audit: &audit,
        notifications: &notifications,
        log_broadcaster: &broadcaster,
        cluster: &cluster,
        tx: &tx,
        outbound: &tx,
    };

    handle_task_result(
        &context,
        TaskResult {
            worker_id: worker.worker_id,
            instance_id: instance.id.clone(),
            success: false,
            message: "exit 2".to_owned(),
            assignment_token: token,
        },
    )
    .await;

    let failed = instances
        .get(&instance.id)
        .await
        .unwrap_or_else(|error| panic!("instance should load: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    assert_eq!(failed.status, InstanceStatus::Failed);
    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("job_instance".to_owned()),
            source_id: Some(instance.id),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    let event_types = timeline
        .iter()
        .map(|message| message.event_type.as_str())
        .collect::<Vec<_>>();
    assert!(!event_types.contains(&"job_instance.failed"));
    assert!(event_types.contains(&"job_instance.retry_exhausted"));
    assert!(
        !serde_json::to_string(&timeline)
            .unwrap_or_default()
            .contains("top-secret-token")
    );
    let attempts = delivery_attempts
        .list_attempts(NotificationDeliveryAttemptFilters::default())
        .await
        .unwrap_or_else(|error| panic!("delivery attempts should list: {error}"));
    assert_eq!(attempts.len(), 1);
    assert!(
        attempts
            .iter()
            .all(|attempt| attempt.retry_state == "retry_pending")
    );
    assert!(
        attempts
            .iter()
            .all(|attempt| attempt.target_redacted == "https://hooks.example.com/...")
    );
}

#[tokio::test]
async fn non_retrying_failed_task_result_emits_failed_notification_policy() {
    use tikeo_core::{ExecutionMode, InstanceStatus, TriggerType};
    use tikeo_proto::worker::v1::{DispatchTask, RegisterWorker, TaskResult, server_message};
    use tikeo_storage::{
        CreateJob, CreateJobInstance, CreateNotificationChannel, CreateNotificationPolicy,
        JobRetryPolicy, NotificationMessageFilters,
    };

    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let logs = JobInstanceLogRepository::new(db.clone());
    let attempts = JobInstanceAttemptRepository::new(db.clone());
    let workflows = WorkflowRepository::new(db.clone());
    let audit = AuditLogRepository::new(db.clone());
    let channels = NotificationChannelRepository::new(db.clone());
    let policies = NotificationPolicyRepository::new(db.clone());
    let messages = NotificationMessageRepository::new(db.clone());
    let delivery_attempts = NotificationDeliveryAttemptRepository::new(db.clone());
    let job = jobs
        .create_job(CreateJob {
            created_by: None,
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "notify-no-retry".to_owned(),
            schedule_type: "api".to_owned(),
            schedule_expr: None,
            misfire_policy: "fire_once".to_owned(),
            schedule_start_at: None,
            schedule_end_at: None,
            schedule_calendar_json: None,
            processor_name: Some("demo.no-retry".to_owned()),
            processor_type: None,
            script_id: None,
            enabled: true,
            canary_job_id: None,
            canary_percent: 0,
            retry_policy: Some(JobRetryPolicy {
                enabled: true,
                max_attempts: 1,
                initial_delay_seconds: 5,
                backoff_multiplier: 2,
                max_delay_seconds: 60,
            }),
        })
        .await
        .unwrap_or_else(|error| panic!("job should create: {error}"));
    let instance = instances
        .create_pending(CreateJobInstance {
            job_id: job.id.clone(),
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Single,
        })
        .await
        .unwrap_or_else(|error| panic!("instance should create: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    let channel = channels
        .create_channel(CreateNotificationChannel {
            scope_type: "app".to_owned(),
            namespace: Some("default".to_owned()),
            app: Some("billing".to_owned()),
            worker_pool: None,
            name: "ops".to_owned(),
            provider: "webhook".to_owned(),
            enabled: true,
            config_json: serde_json::json!({"url":"https://hooks.example.com/services/no-retry"})
                .to_string(),
            secret_refs_json: "{}".to_owned(),
            safety_policy_json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("channel should create: {error}"));
    policies
        .create_policy(CreateNotificationPolicy {
            owner_type: "job".to_owned(),
            owner_id: Some(job.id.clone()),
            name: "notify only failed".to_owned(),
            event_family: "job_instance".to_owned(),
            event_filter_json: serde_json::json!({"statuses":["failed"]}).to_string(),
            channel_refs_json: serde_json::json!([{"channelId": channel.id}]).to_string(),
            template_ref: None,
            severity: "critical".to_owned(),
            enabled: true,
            dedupe_seconds: 300,
        })
        .await
        .unwrap_or_else(|error| panic!("policy should create: {error}"));
    let claim = workflows
        .claim_next_job_queue_item("server-a", 30)
        .await
        .unwrap_or_else(|error| panic!("queue should claim: {error}"))
        .unwrap_or_else(|| panic!("queue item should exist"));
    workflows
        .mark_dispatch_queue_running(&claim.item.id, "server-a")
        .await
        .unwrap_or_else(|error| panic!("queue should run: {error}"));
    instances
        .update_status(&instance.id, InstanceStatus::Running)
        .await
        .unwrap_or_else(|error| panic!("instance should run: {error}"));

    let (outbound, mut rx) = mpsc::channel(8);
    let registry = WorkerRegistry::default();
    let worker = registry
        .register(
            RegisterWorker {
                client_instance_id: "notify-no-retry-worker".to_owned(),
                app: "billing".to_owned(),
                namespace: "default".to_owned(),
                cluster: "local".to_owned(),
                region: "local".to_owned(),
                capabilities: Vec::new(),
                structured_capabilities: None,
                election: None,
                labels: std::collections::HashMap::default(),
            },
            outbound,
        )
        .await;
    registry
        .dispatch_to_worker(
            &worker.worker_id,
            DispatchTask {
                instance_id: instance.id.clone(),
                job_id: job.id.clone(),
                payload: Vec::new(),
                processor_name: "demo.no-retry".to_owned(),
                processor_binding: None,
                assignment_token: String::new(),
            },
        )
        .await
        .unwrap_or_else(|| panic!("task should dispatch"));
    let token = match rx
        .recv()
        .await
        .unwrap_or_else(|| panic!("dispatch should arrive"))
        .unwrap_or_else(|error| panic!("dispatch should be ok: {error}"))
        .kind
    {
        Some(server_message::Kind::DispatchTask(task)) => task.assignment_token,
        other => panic!("unexpected server message: {other:?}"),
    };
    persist_assignment_token_for_test(&attempts, &instance.id, &worker.worker_id, &token).await;
    let (tx, _events) = mpsc::channel(8);
    let broadcaster = TaskLogBroadcaster::default();
    let templates = tikeo_storage::NotificationTemplateRepository::new(channels.db());
    let notifications = crate::notification::NotificationCenter::new(
        channels,
        policies,
        messages.clone(),
        delivery_attempts,
        templates,
        jobs.clone(),
    );
    let cluster = standalone_cluster();
    let context = WorkerMessageContext {
        registry: &registry,
        instances: &instances,
        jobs: &jobs,
        logs: &logs,
        attempts: &attempts,
        workflows: &workflows,
        audit: &audit,
        notifications: &notifications,
        log_broadcaster: &broadcaster,
        cluster: &cluster,
        tx: &tx,
        outbound: &tx,
    };

    handle_task_result(
        &context,
        TaskResult {
            worker_id: worker.worker_id,
            instance_id: instance.id.clone(),
            success: false,
            message: "exit 2".to_owned(),
            assignment_token: token,
        },
    )
    .await;

    let failed = instances
        .get(&instance.id)
        .await
        .unwrap_or_else(|error| panic!("instance should load: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    assert_eq!(failed.status, InstanceStatus::Failed);
    let timeline = messages
        .list_messages(NotificationMessageFilters {
            source_type: Some("job_instance".to_owned()),
            source_id: Some(instance.id),
            ..Default::default()
        })
        .await
        .unwrap_or_else(|error| panic!("messages should list: {error}"));
    let event_types = timeline
        .iter()
        .map(|message| message.event_type.as_str())
        .collect::<Vec<_>>();
    assert!(event_types.contains(&"job_instance.failed"));
    assert!(!event_types.contains(&"job_instance.retry_exhausted"));
}

#[tokio::test]
async fn subscribe_task_logs_replays_existing_and_streams_live_logs() {
    use tikeo_core::{ExecutionMode, TriggerType};
    use tikeo_proto::worker::v1::worker_tunnel_service_server::WorkerTunnelService;
    use tikeo_storage::{CreateJob, CreateJobInstance, JobRepository};
    use tonic::Request;

    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    let jobs = JobRepository::new(db.clone());
    let instances = JobInstanceRepository::new(db.clone());
    let logs = JobInstanceLogRepository::new(db.clone());
    let attempts = JobInstanceAttemptRepository::new(db.clone());
    let workflows = WorkflowRepository::new(db);
    let job = jobs
        .create_job(CreateJob {
            created_by: None,
            namespace: "default".to_owned(),
            app: "billing".to_owned(),
            name: "log-stream".to_owned(),
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
            retry_policy: None,
        })
        .await
        .unwrap_or_else(|error| panic!("job should create: {error}"));
    let instance = instances
        .create_pending(CreateJobInstance {
            job_id: job.id,
            trigger_type: TriggerType::Api,
            execution_mode: ExecutionMode::Single,
        })
        .await
        .unwrap_or_else(|error| panic!("instance should create: {error}"))
        .unwrap_or_else(|| panic!("instance should exist"));
    logs.append(tikeo_storage::AppendJobInstanceLog {
        instance_id: instance.id.clone(),
        worker_id: "wrk-existing".to_owned(),
        level: "INFO".to_owned(),
        message: "existing".to_owned(),
        sequence: 1,
    })
    .await
    .unwrap_or_else(|error| panic!("existing log should append: {error}"));

    let audit = audit().await;
    let broadcaster = TaskLogBroadcaster::default();
    let service = super::WorkerTunnel::new(crate::tunnel::WorkerTunnelRuntime::new(
        crate::tunnel::WorkerTunnelRuntimeParts {
            registry: WorkerRegistry::default(),
            instances,
            jobs,
            logs,
            attempts,
            workflows,
            audit,
            notifications: None,
            log_broadcaster: broadcaster.clone(),
            cluster: standalone_cluster(),
        },
    ));
    let response = service
        .subscribe_task_logs(Request::new(SubscribeTaskLogsRequest {
            instance_id: instance.id.clone(),
            after_sequence: 0,
            replay_existing: true,
        }))
        .await
        .unwrap_or_else(|error| panic!("subscription should start: {error}"));
    let mut stream = response.into_inner();
    let replayed = stream
        .next()
        .await
        .unwrap_or_else(|| panic!("replayed log should exist"))
        .unwrap_or_else(|error| panic!("replay should stream: {error}"));
    assert_eq!(replayed.message, "existing");

    broadcaster.publish(TaskLog {
        worker_id: "wrk-live".to_owned(),
        instance_id: instance.id,
        level: "INFO".to_owned(),
        message: "live".to_owned(),
        sequence: 2,
        assignment_token: String::new(),
    });
    let live = stream
        .next()
        .await
        .unwrap_or_else(|| panic!("live log should exist"))
        .unwrap_or_else(|error| panic!("live log should stream: {error}"));
    assert_eq!(live.message, "live");
}

async fn persist_assignment_token_for_test(
    attempts: &JobInstanceAttemptRepository,
    instance_id: &str,
    worker_id: &str,
    token: &str,
) {
    let _ = attempts
        .create_pending_for_workers(instance_id, &[worker_id.to_owned()])
        .await
        .unwrap_or_else(|error| panic!("attempt should exist for assignment token: {error}"));
    assert!(
        attempts
            .record_assignment_token(instance_id, worker_id, token)
            .await
            .unwrap_or_else(|error| panic!("assignment token should persist: {error}")),
        "assignment token should be recorded for test dispatch"
    );
}

async fn jobs() -> JobRepository {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    JobRepository::new(db)
}

async fn instances() -> JobInstanceRepository {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    JobInstanceRepository::new(db)
}

async fn attempts() -> JobInstanceAttemptRepository {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    JobInstanceAttemptRepository::new(db)
}

async fn workflows() -> WorkflowRepository {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    WorkflowRepository::new(db)
}

async fn logs() -> JobInstanceLogRepository {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    JobInstanceLogRepository::new(db)
}

async fn audit() -> AuditLogRepository {
    let db = connect_and_migrate("sqlite::memory:")
        .await
        .unwrap_or_else(|error| panic!("test storage should initialize: {error}"));
    AuditLogRepository::new(db)
}
