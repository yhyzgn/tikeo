use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
pub(super) enum Workflows {
    Table,
    Id,
    Name,
    Definition,
    Status,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum WorkflowNodes {
    Table,
    Id,
    WorkflowId,
    NodeKey,
    Name,
    Kind,
    JobId,
    ProcessorName,
    Config,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum WorkflowEdges {
    Table,
    Id,
    WorkflowId,
    FromNodeKey,
    ToNodeKey,
    Condition,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum WorkflowInstances {
    Table,
    Id,
    WorkflowId,
    Status,
    TriggerType,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum WorkflowNodeInstances {
    Table,
    Id,
    WorkflowInstanceId,
    NodeKey,
    Status,
    JobInstanceId,
    ChildWorkflowInstanceId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum WorkflowShards {
    Table,
    Id,
    WorkflowInstanceId,
    WorkflowNodeInstanceId,
    NodeKey,
    ShardIndex,
    Status,
    Input,
    Output,
    Checkpoint,
    RetryCount,
    JobInstanceId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum DispatchQueue {
    Table,
    Id,
    JobInstanceId,
    WorkflowNodeInstanceId,
    Priority,
    RunAfter,
    Status,
    Attempt,
    LeaseOwner,
    LeaseUntil,
    FencingToken,
    WorkerSelector,
    Namespace,
    App,
    WorkerPool,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum InstanceEvents {
    Table,
    Id,
    InstanceId,
    InstanceType,
    EventType,
    Message,
    Payload,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum RaftMetadata {
    Table,
    Id,
    ClusterId,
    NodeId,
    CurrentTerm,
    VotedFor,
    CommitIndex,
    AppliedIndex,
    LeaderFencingToken,
    ConfState,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum RaftMembers {
    Table,
    Id,
    NodeId,
    Endpoint,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum RaftLogEntries {
    Table,
    Id,
    ClusterId,
    NodeId,
    LogIndex,
    Term,
    EntryType,
    Data,
    Context,
    SyncStatus,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum RaftSnapshots {
    Table,
    Id,
    ClusterId,
    NodeId,
    SnapshotIndex,
    Term,
    ConfState,
    Data,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum RaftAppliedCommands {
    Table,
    Id,
    ClusterId,
    NodeId,
    LogIndex,
    Term,
    CommandId,
    CommandType,
    Payload,
    Status,
    Message,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum RaftMembershipProposals {
    Table,
    Id,
    ClusterId,
    ProposalId,
    Action,
    NodeId,
    Endpoint,
    Status,
    Message,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum ScheduleCursors {
    Table,
    Id,
    JobId,
    TriggerType,
    FireAt,
    InstanceId,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum SdkApiKeys {
    Table,
    Id,
    Name,
    KeyHash,
    KeyPrefix,
    Namespace,
    App,
    ServiceAccountId,
    ServiceAccountName,
    Scopes,
    Status,
    ExpiresAt,
    LastUsedAt,
    CreatedBy,
    RevokedBy,
    RotatedFrom,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum Secrets {
    Table,
    Id,
    Namespace,
    App,
    Name,
    ValueRef,
    Status,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum AuthSessions {
    Table,
    Id,
    UserId,
    TokenHash,
    DeviceId,
    DeviceName,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum OidcAuthStates {
    Table,
    Id,
    StateHash,
    RedirectUri,
    ExpiresAt,
    ConsumedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum OidcIdentities {
    Table,
    Id,
    Issuer,
    Subject,
    Username,
    Namespace,
    App,
    WorkerPool,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum Plugins {
    Table,
    Id,
    Name,
    Kind,
    ProcessorTypesJson,
    AlertChannelTypesJson,
    Enabled,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum Scripts {
    Table,
    Id,
    Name,
    Language,
    Version,
    Content,
    Status,
    ReleasedVersionId,
    ReleasedVersionNumber,
    ReleaseApprovalTicket,
    ReleaseSignature,
    ReleaseSignatureVerifiedAt,
    ReleaseSignatureVerifiedBy,
    ReleaseGrantsJson,
    ReleaseGrantsVerifiedAt,
    ReleaseGrantsVerifiedBy,
    TimeoutSeconds,
    MaxMemoryBytes,
    AllowNetwork,
    AllowedEnvVars,
    PolicyJson,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum ScriptVersions {
    Table,
    Id,
    ScriptId,
    VersionNumber,
    Content,
    ContentSha256,
    Language,
    Status,
    TimeoutSeconds,
    MaxMemoryBytes,
    AllowNetwork,
    AllowedEnvVars,
    PolicyJson,
    CreatedBy,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum AlertRules {
    Table,
    Id,
    Name,
    Severity,
    ConditionJson,
    ChannelsJson,
    Enabled,
    DedupeSeconds,
    SilencedUntil,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum AlertEvents {
    Table,
    Id,
    RuleId,
    RuleName,
    Severity,
    Status,
    EventType,
    ResourceType,
    ResourceId,
    FailureClass,
    Message,
    DedupeKey,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum AlertDeliveryAttempts {
    Table,
    Id,
    EventId,
    RuleId,
    Provider,
    Target,
    Delivered,
    StatusCode,
    Error,
    Attempt,
    RetryState,
    NextRetryAt,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum AuditLogs {
    Table,
    Id,
    Actor,
    Action,
    ResourceType,
    ResourceId,
    Detail,
    Before,
    After,
    TraceId,
    Result,
    FailureReason,
    IpAddress,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum Roles {
    Table,
    Id,
    Name,
    Description,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum Permissions {
    Table,
    Id,
    Resource,
    Action,
    Description,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum RolePermissions {
    Table,
    Id,
    RoleId,
    PermissionId,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum ServiceAccounts {
    Table,
    Id,
    Name,
    Description,
    Namespace,
    App,
    WorkerPool,
    Status,
    CreatedBy,
    UpdatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum Users {
    Table,
    Id,
    Username,
    Email,
    Password,
    Role,
    BootstrapAdmin,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum Namespaces {
    Table,
    Id,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum Apps {
    Table,
    Id,
    NamespaceId,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum WorkerPools {
    Table,
    Id,
    NamespaceId,
    AppId,
    Name,
    MaxQueueDepth,
    MaxConcurrency,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum WorkerLogicalInstances {
    Table,
    Id,
    NamespaceName,
    AppName,
    Cluster,
    Region,
    ClientInstanceId,
    CurrentWorkerId,
    CurrentGeneration,
    Status,
    LastSeenAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum WorkerSessions {
    Table,
    WorkerId,
    LogicalInstanceId,
    ConnectionId,
    Generation,
    FencingTokenHash,
    Status,
    StatusReason,
    StatusEvidence,
    LeaseExpiresAt,
    LastHeartbeatAt,
    LastSequence,
    ConnectedAt,
    DisconnectedAt,
    ReplacedByWorkerId,
    DrainRequestedAt,
    CapabilitiesJson,
    StructuredCapabilitiesJson,
    LabelsJson,
    MasterJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum WorkerSessionEvents {
    Table,
    Id,
    WorkerId,
    LogicalInstanceId,
    EventType,
    Reason,
    DetailJson,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum Jobs {
    Table,
    Id,
    NamespaceId,
    AppId,
    Name,
    ScheduleType,
    ScheduleExpr,
    MisfirePolicy,
    ScheduleStartAt,
    ScheduleEndAt,
    ScheduleCalendarJson,
    ProcessorName,
    ProcessorType,
    ScriptId,
    Enabled,
    CanaryJobId,
    CanaryPercent,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum JobVersions {
    Table,
    Id,
    JobId,
    VersionNumber,
    Name,
    ScheduleType,
    ScheduleExpr,
    MisfirePolicy,
    ScheduleStartAt,
    ScheduleEndAt,
    ScheduleCalendarJson,
    ProcessorName,
    ProcessorType,
    ScriptId,
    Enabled,
    CreatedBy,
    ChangeReason,
    RolledBackFromVersion,
    CreatedAt,
}

#[derive(DeriveIden)]
pub(super) enum JobInstances {
    Table,
    Id,
    JobId,
    Status,
    TriggerType,
    ExecutionMode,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum JobInstanceAttempts {
    Table,
    Id,
    InstanceId,
    WorkerId,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(super) enum JobInstanceLogs {
    Table,
    Id,
    InstanceId,
    WorkerId,
    Level,
    Message,
    Sequence,
    CreatedAt,
}
