//! Repository APIs over tikeo metadata tables.
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    missing_docs
)]

mod alert;
mod attempt;
mod audit;
mod auth;
mod calendar;
mod instance;
mod job;
mod job_repo;
mod job_version;
mod log;
mod notification;
mod notification_template;
mod oidc;
mod oidc_identity;
mod plugin;
mod raft;
mod rbac;
mod schedule_cursor;
mod scope;
mod script;
mod sdk_api_key;
mod secret;
mod service_account;
mod user;
pub mod util;
mod worker_dispatch_outbox;
mod worker_lifecycle;
mod workflow;

pub use alert::{
    AlertDeliveryAttemptFilters, AlertDeliveryAttemptSummary, AlertEventFilters, AlertEventSummary,
    AlertRepository, AlertRuleSummary, CreateAlertRule, RecordAlertDeliveryAttempt,
};
pub use attempt::{
    CreateJobInstanceAttempt, JobInstanceAttemptRepository, JobInstanceAttemptSummary,
};
pub use audit::{
    AuditLogFilters, AuditLogPageSummary, AuditLogRepository, AuditLogSummary, CreateAuditLog,
};
pub use auth::{AuthSessionRepository, AuthSessionSummary, CreateAuthSession, PermissionSummary};
pub use calendar::{CalendarRepository, CalendarSummary, CalendarWindowSummary, UpsertCalendar};
pub use instance::{
    CreateJobInstance, JobDurationHistory, JobInstanceRepository, JobInstanceResult,
    JobInstanceSummary,
};
pub use job::{CreateJob, JobRetryPolicy, JobSummary, UpdateJob};
pub use job_repo::JobRepository;
pub use job_version::{JobVersionRepository, JobVersionSummary};
pub use log::{AppendJobInstanceLog, JobInstanceLogRepository, JobInstanceLogSummary};
pub use notification::{
    CreateNotificationChannel, CreateNotificationMessage, CreateNotificationPolicy,
    NotificationChannelDeleteResult, NotificationChannelDeliveryConfig, NotificationChannelFilters,
    NotificationChannelRepository, NotificationChannelSummary, NotificationDeliveryAttemptFilters,
    NotificationDeliveryAttemptRepository, NotificationDeliveryAttemptSummary,
    NotificationMessageFilters, NotificationMessageRepository, NotificationMessageSummary,
    NotificationPolicyFilters, NotificationPolicyRepository, NotificationPolicySummary,
    NotificationPolicyValidationSummary, RecordNotificationDeliveryAttempt,
    UpdateNotificationChannel, UpdateNotificationPolicy,
};
pub use notification_template::{
    CreateNotificationTemplate, NotificationTemplateFilters, NotificationTemplateRepository,
    NotificationTemplateSummary, UpdateNotificationTemplate,
};
pub use oidc::{CreateOidcAuthState, OidcAuthStateRepository, OidcAuthStateSummary};
pub use oidc_identity::{OidcIdentityRepository, OidcIdentitySummary, UpsertOidcIdentity};
pub use plugin::{
    CreatePlugin, PluginAlertChannelTypeSummary, PluginProcessorTypeSummary, PluginRepository,
    PluginSummary, UpdatePlugin,
};
pub use raft::{
    RaftAppliedCommandSummary, RaftLogEntrySummary, RaftMemberSummary,
    RaftMembershipProposalSummary, RaftMetadataSummary, RaftRepository, RaftSnapshotSummary,
    RecordRaftAppliedCommand, RecordRaftMembershipProposal, UpsertRaftLogEntry, UpsertRaftMember,
    UpsertRaftMetadata, UpsertRaftSnapshot,
};
pub use rbac::{CreateRole, PermissionCatalogItem, RbacRepository, RoleSummary, UpdateRole};
pub use schedule_cursor::ScheduleCursorRepository;
pub use scope::{
    AppSummary, NamespaceSummary, ScopeRepository, UpdateWorkerPoolQuota, WorkerPoolSummary,
};
pub use script::{
    CreateScript, ScriptReleaseGrantEvidenceSummary, ScriptReleaseSignatureSummary,
    ScriptRepository, ScriptSummary, ScriptVersionRepository, ScriptVersionSummary, UpdateScript,
    VerifiedScriptReleaseGrants, VerifiedScriptReleaseSignature,
};
pub use sdk_api_key::{CreateSdkApiKey, SdkApiKeyRepository, SdkApiKeySummary, UpdateSdkApiKey};
pub use secret::{CreateSecret, SecretRepository, SecretSummary};
pub use service_account::{
    CreateServiceAccount, ServiceAccountRepository, ServiceAccountSummary, UpdateServiceAccount,
};
pub use user::{CreateUser, UpdateUser, UserRepository, UserSummary};
pub use worker_dispatch_outbox::{
    CreateWorkerDispatchOutbox, WorkerDispatchOutboxRepository, WorkerDispatchOutboxSloSummary,
    WorkerDispatchOutboxSummary,
};
pub use worker_lifecycle::{
    PersistedOnlineWorkerSummary, RegisterWorkerSession, WorkerHeartbeat,
    WorkerLifecycleRepository, WorkerSessionEventSummary, WorkerSessionSnapshotUpdate,
    WorkerSessionSummary,
};
pub use workflow::{
    AdvanceWorkflowInput, AdvanceWorkflowResult, CompleteWorkflowShardInput,
    CompleteWorkflowShardResult, CreateWorkflow, DispatchQueueClaim, DispatchQueueSloSummary,
    DispatchQueueSummary, InstanceEventSummary, MaterializeWorkflowNodeResult, QueueOverview,
    RebalanceWorkflowShardsInput, RebalanceWorkflowShardsResult, RecoverWorkflowNodeInput,
    RecoverWorkflowNodeResult, UpdateWorkflow, WorkflowDefinition, WorkflowEdgeSpec,
    WorkflowInstanceSummary, WorkflowJobResultOutcome, WorkflowNodeInstanceSummary,
    WorkflowNodeSpec, WorkflowRepository, WorkflowShardSummary, WorkflowSloSummary,
    WorkflowSummary, WorkflowValidationResult, validate_workflow_definition,
};

#[cfg(test)]
mod tests;
