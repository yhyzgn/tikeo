use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct CreateNotificationChannel {
    /// Scope type value.
    pub scope_type: String,
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
    /// Worker pool value.
    pub worker_pool: Option<String>,
    /// Name value.
    pub name: String,
    /// Provider value.
    pub provider: String,
    /// Boolean state flag.
    pub enabled: bool,
    /// Serialized data value.
    pub config_json: String,
    /// Serialized data value.
    pub secret_refs_json: String,
    /// Serialized data value.
    pub safety_policy_json: Option<String>,
}

#[derive(Debug, Clone, Default)]
/// Public storage data type.
pub struct UpdateNotificationChannel {
    /// Scope type value.
    pub scope_type: Option<String>,
    /// Namespace value.
    pub namespace: Option<Option<String>>,
    /// App value.
    pub app: Option<Option<String>>,
    /// Worker pool value.
    pub worker_pool: Option<Option<String>>,
    /// Name value.
    pub name: Option<String>,
    /// Provider value.
    pub provider: Option<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
    /// Serialized data value.
    pub config_json: Option<String>,
    /// Serialized data value.
    pub secret_refs_json: Option<String>,
    /// Serialized data value.
    pub safety_policy_json: Option<Option<String>>,
    /// Updated by value.
    pub updated_by: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
/// Public storage data type.
pub struct NotificationChannelFilters {
    /// Scope type value.
    pub scope_type: Option<String>,
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
    /// Worker pool value.
    pub worker_pool: Option<String>,
    /// Provider value.
    pub provider: Option<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
/// Public storage data type.
pub struct NotificationChannelSummary {
    /// Identifier value.
    pub id: String,
    /// Scope type value.
    pub scope_type: String,
    /// Namespace value.
    pub namespace: Option<String>,
    /// App value.
    pub app: Option<String>,
    /// Worker pool value.
    pub worker_pool: Option<String>,
    /// Name value.
    pub name: String,
    /// Provider value.
    pub provider: String,
    /// Boolean state flag.
    pub enabled: bool,
    /// Serialized data value.
    pub config_json: String,
    #[serde(skip_serializing)]
    /// Serialized data value.
    pub secret_refs_json: String,
    /// Target redacted value.
    pub target_redacted: String,
    /// Serialized data value.
    pub safety_policy_json: Option<String>,
    /// Target configured value.
    pub target_configured: bool,
    /// Secret configured value.
    pub secret_configured: bool,
    /// Created by value.
    pub created_by: Option<String>,
    /// Updated by value.
    pub updated_by: Option<String>,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
/// Public storage data type.
pub struct NotificationChannelDeleteResult {
    /// Deleted value.
    pub deleted: bool,
    /// Referenced by policies value.
    pub referenced_by_policies: u64,
}

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct NotificationChannelDeliveryConfig {
    /// Identifier value.
    pub id: String,
    /// Provider value.
    pub provider: String,
    /// Boolean state flag.
    pub enabled: bool,
    /// Serialized data value.
    pub config_json: String,
    /// Serialized data value.
    pub secret_refs_json: String,
    /// Target redacted value.
    pub target_redacted: String,
    /// Serialized data value.
    pub safety_policy_json: Option<String>,
}

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct CreateNotificationPolicy {
    /// Owner type value.
    pub owner_type: String,
    /// Identifier value.
    pub owner_id: Option<String>,
    /// Name value.
    pub name: String,
    /// Event family value.
    pub event_family: String,
    /// Serialized data value.
    pub event_filter_json: String,
    /// Serialized data value.
    pub channel_refs_json: String,
    /// Template ref value.
    pub template_ref: Option<String>,
    /// Severity value.
    pub severity: String,
    /// Boolean state flag.
    pub enabled: bool,
    /// Dedupe seconds value.
    pub dedupe_seconds: i64,
}

#[derive(Debug, Clone, Default)]
/// Public storage data type.
pub struct UpdateNotificationPolicy {
    /// Owner type value.
    pub owner_type: Option<String>,
    /// Identifier value.
    pub owner_id: Option<Option<String>>,
    /// Name value.
    pub name: Option<String>,
    /// Event family value.
    pub event_family: Option<String>,
    /// Serialized data value.
    pub event_filter_json: Option<String>,
    /// Serialized data value.
    pub channel_refs_json: Option<String>,
    /// Template ref value.
    pub template_ref: Option<Option<String>>,
    /// Severity value.
    pub severity: Option<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
    /// Dedupe seconds value.
    pub dedupe_seconds: Option<i64>,
    /// Serialized data value.
    pub throttle_json: Option<Option<String>>,
    /// Serialized data value.
    pub quiet_hours_json: Option<Option<String>>,
    /// Serialized data value.
    pub escalation_json: Option<Option<String>>,
    /// Updated by value.
    pub updated_by: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
/// Public storage data type.
pub struct NotificationPolicyFilters {
    /// Owner type value.
    pub owner_type: Option<String>,
    /// Identifier value.
    pub owner_id: Option<String>,
    /// Event family value.
    pub event_family: Option<String>,
    /// Boolean state flag.
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
/// Public storage data type.
pub struct NotificationPolicySummary {
    /// Identifier value.
    pub id: String,
    /// Name value.
    pub name: String,
    /// Boolean state flag.
    pub enabled: bool,
    /// Owner type value.
    pub owner_type: String,
    /// Identifier value.
    pub owner_id: Option<String>,
    /// Event family value.
    pub event_family: String,
    /// Serialized data value.
    pub event_filter_json: String,
    /// Serialized data value.
    pub channel_refs_json: String,
    /// Template ref value.
    pub template_ref: Option<String>,
    /// Severity value.
    pub severity: String,
    /// Dedupe seconds value.
    pub dedupe_seconds: i64,
    /// Serialized data value.
    pub throttle_json: Option<String>,
    /// Serialized data value.
    pub quiet_hours_json: Option<String>,
    /// Serialized data value.
    pub escalation_json: Option<String>,
    /// Created by value.
    pub created_by: Option<String>,
    /// Updated by value.
    pub updated_by: Option<String>,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
/// Public storage data type.
pub struct NotificationPolicyValidationSummary {
    /// Identifier value.
    pub policy_id: String,
    /// Valid value.
    pub valid: bool,
    /// Channel count value.
    pub channel_count: u64,
    /// Missing channel ids value.
    pub missing_channel_ids: Vec<String>,
    /// Disabled channel ids value.
    pub disabled_channel_ids: Vec<String>,
    /// Issues value.
    pub issues: Vec<String>,
}

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct CreateNotificationMessage {
    /// Source type value.
    pub source_type: String,
    /// Identifier value.
    pub source_id: String,
    /// Identifier value.
    pub policy_id: String,
    /// Event type value.
    pub event_type: String,
    /// Resource type value.
    pub resource_type: String,
    /// Identifier value.
    pub resource_id: String,
    /// Severity value.
    pub severity: String,
    /// Subject value.
    pub subject: String,
    /// Body value.
    pub body: String,
    /// Serialized data value.
    pub payload_json: String,
    /// Dedupe key value.
    pub dedupe_key: String,
    /// Identifier value.
    pub trace_id: Option<String>,
    /// Status value.
    pub status: String,
}

#[derive(Debug, Clone, Default)]
/// Public storage data type.
pub struct NotificationMessageFilters {
    /// Source type value.
    pub source_type: Option<String>,
    /// Identifier value.
    pub source_id: Option<String>,
    /// Identifier value.
    pub policy_id: Option<String>,
    /// Event type value.
    pub event_type: Option<String>,
    /// Severity value.
    pub severity: Option<String>,
    /// Status value.
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
/// Public storage data type.
pub struct NotificationMessageSummary {
    /// Identifier value.
    pub id: String,
    /// Source type value.
    pub source_type: String,
    /// Identifier value.
    pub source_id: String,
    /// Identifier value.
    pub policy_id: String,
    /// Event type value.
    pub event_type: String,
    /// Resource type value.
    pub resource_type: String,
    /// Identifier value.
    pub resource_id: String,
    /// Severity value.
    pub severity: String,
    /// Subject value.
    pub subject: String,
    /// Body value.
    pub body: String,
    /// Serialized data value.
    pub payload_json: String,
    /// Dedupe key value.
    pub dedupe_key: String,
    /// Identifier value.
    pub trace_id: Option<String>,
    /// Status value.
    pub status: String,
    /// Timestamp value.
    pub created_at: String,
    /// Timestamp value.
    pub updated_at: String,
}

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct RecordNotificationDeliveryAttempt {
    /// Identifier value.
    pub message_id: String,
    /// Identifier value.
    pub policy_id: String,
    /// Identifier value.
    pub channel_id: String,
    /// Provider value.
    pub provider: String,
    /// Target redacted value.
    pub target_redacted: String,
    /// Attempt value.
    pub attempt: i32,
    /// Delivered value.
    pub delivered: bool,
    /// Status code value.
    pub status_code: Option<i32>,
    /// Error value.
    pub error: Option<String>,
    /// Retry state value.
    pub retry_state: String,
    /// Timestamp value.
    pub next_retry_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
/// Public storage data type.
pub struct NotificationDeliveryAttemptFilters {
    /// Identifier value.
    pub message_id: Option<String>,
    /// Identifier value.
    pub policy_id: Option<String>,
    /// Identifier value.
    pub channel_id: Option<String>,
    /// Provider value.
    pub provider: Option<String>,
    /// Retry state value.
    pub retry_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
/// Public storage data type.
pub struct NotificationDeliveryAttemptSummary {
    /// Identifier value.
    pub id: String,
    /// Identifier value.
    pub message_id: String,
    /// Identifier value.
    pub policy_id: String,
    /// Identifier value.
    pub channel_id: String,
    /// Provider value.
    pub provider: String,
    /// Target redacted value.
    pub target_redacted: String,
    /// Attempt value.
    pub attempt: i32,
    /// Delivered value.
    pub delivered: bool,
    /// Status code value.
    pub status_code: Option<i32>,
    /// Error value.
    pub error: Option<String>,
    /// Retry state value.
    pub retry_state: String,
    /// Timestamp value.
    pub next_retry_at: Option<String>,
    /// Timestamp value.
    pub created_at: String,
}

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct NotificationChannelRepository {
    pub(super) db: DatabaseConnection,
}

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct NotificationPolicyRepository {
    pub(super) db: DatabaseConnection,
}

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct NotificationMessageRepository {
    pub(super) db: DatabaseConnection,
}

#[derive(Debug, Clone)]
/// Public storage data type.
pub struct NotificationDeliveryAttemptRepository {
    pub(super) db: DatabaseConnection,
}
