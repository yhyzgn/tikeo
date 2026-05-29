#![allow(missing_docs)]

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::collections::{BTreeMap, HashMap};

use crate::entities::{alert_delivery_attempt, alert_event, alert_rule};

use super::util::{new_id, now_rfc3339};

#[derive(Debug, Clone)]
pub struct RecordAlertDeliveryAttempt {
    pub event_id: String,
    pub rule_id: String,
    pub provider: String,
    pub target: String,
    pub delivered: bool,
    pub status_code: Option<i32>,
    pub error: Option<String>,
    pub attempt: i32,
    pub retry_state: String,
    pub next_retry_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AlertDeliveryAttemptSummary {
    pub id: String,
    pub event_id: String,
    pub rule_id: String,
    pub provider: String,
    pub target: String,
    pub delivered: bool,
    pub status_code: Option<i32>,
    pub error: Option<String>,
    pub attempt: i32,
    pub retry_state: String,
    pub next_retry_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct AlertDeliveryAttemptFilters {
    pub event_id: Option<String>,
    pub rule_id: Option<String>,
    pub provider: Option<String>,
    pub retry_state: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateAlertRule {
    pub name: String,
    pub severity: String,
    pub condition_json: String,
    pub channels_json: String,
    pub enabled: bool,
    pub dedupe_seconds: i64,
    pub silenced_until: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertRuleSummary {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub condition_json: String,
    pub channels_json: String,
    pub enabled: bool,
    pub dedupe_seconds: i64,
    pub silenced_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertEventSummary {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub status: String,
    pub event_type: String,
    pub resource_type: String,
    pub resource_id: String,
    pub failure_class: Option<String>,
    pub message: Option<String>,
    pub dedupe_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertNotificationSummary {
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub resource_type: String,
    pub resource_id: String,
    pub failure_class: Option<String>,
    pub latest_status: String,
    pub latest_event_type: String,
    pub latest_message: Option<String>,
    pub event_count: u64,
    pub firing_count: u64,
    pub suppressed_count: u64,
    pub silenced_count: u64,
    pub recovered_count: u64,
    pub first_seen: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AlertEventStatusCounts {
    pub total_events: u64,
    pub by_status: BTreeMap<String, u64>,
    pub script_failure_events: u64,
    pub by_failure_class: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default)]
pub struct AlertEventFilters {
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub failure_class: Option<String>,
    pub rule_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AlertRepository {
    db: DatabaseConnection,
}

impl AlertRepository {
    #[must_use]
    pub const fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[must_use]
    pub fn db(&self) -> DatabaseConnection {
        self.db.clone()
    }

    pub async fn record_delivery_attempt(
        &self,
        input: RecordAlertDeliveryAttempt,
    ) -> Result<AlertDeliveryAttemptSummary, sea_orm::DbErr> {
        let model = alert_delivery_attempt::ActiveModel {
            id: Set(new_id("alert-delivery")),
            event_id: Set(input.event_id),
            rule_id: Set(input.rule_id),
            provider: Set(input.provider),
            target: Set(input.target),
            delivered: Set(input.delivered),
            status_code: Set(input.status_code),
            error: Set(input.error),
            attempt: Set(input.attempt),
            retry_state: Set(input.retry_state),
            next_retry_at: Set(input.next_retry_at),
            created_at: Set(now_rfc3339()),
        }
        .insert(&self.db)
        .await?;
        Ok(AlertDeliveryAttemptSummary::from(model))
    }

    pub async fn list_delivery_attempts(
        &self,
        filters: AlertDeliveryAttemptFilters,
    ) -> Result<Vec<AlertDeliveryAttemptSummary>, sea_orm::DbErr> {
        let mut query = alert_delivery_attempt::Entity::find();
        if let Some(value) = filters.event_id {
            query = query.filter(alert_delivery_attempt::Column::EventId.eq(value));
        }
        if let Some(value) = filters.rule_id {
            query = query.filter(alert_delivery_attempt::Column::RuleId.eq(value));
        }
        if let Some(value) = filters.provider {
            query = query.filter(alert_delivery_attempt::Column::Provider.eq(value));
        }
        if let Some(value) = filters.retry_state {
            query = query.filter(alert_delivery_attempt::Column::RetryState.eq(value));
        }
        let rows = query
            .order_by_desc(alert_delivery_attempt::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(AlertDeliveryAttemptSummary::from)
            .collect())
    }

    pub async fn list_due_delivery_attempts(
        &self,
        limit: u64,
    ) -> Result<Vec<AlertDeliveryAttemptSummary>, sea_orm::DbErr> {
        let now = now_rfc3339();
        let rows = alert_delivery_attempt::Entity::find()
            .filter(alert_delivery_attempt::Column::RetryState.eq("retry_pending"))
            .filter(
                alert_delivery_attempt::Column::NextRetryAt
                    .is_null()
                    .or(alert_delivery_attempt::Column::NextRetryAt.lte(now)),
            )
            .order_by_asc(alert_delivery_attempt::Column::NextRetryAt)
            .order_by_asc(alert_delivery_attempt::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(AlertDeliveryAttemptSummary::from)
            .collect())
    }

    pub async fn mark_delivery_attempt_retry_state(
        &self,
        id: &str,
        retry_state: &str,
        error: Option<&str>,
        next_retry_at: Option<&str>,
    ) -> Result<Option<AlertDeliveryAttemptSummary>, sea_orm::DbErr> {
        let Some(row) = alert_delivery_attempt::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: alert_delivery_attempt::ActiveModel = row.into();
        active.retry_state = Set(retry_state.to_owned());
        if let Some(error) = error {
            active.error = Set(Some(error.to_owned()));
        }
        active.next_retry_at = Set(next_retry_at.map(ToOwned::to_owned));
        let updated = active.update(&self.db).await?;
        Ok(Some(AlertDeliveryAttemptSummary::from(updated)))
    }

    pub async fn create_rule(
        &self,
        input: CreateAlertRule,
    ) -> Result<AlertRuleSummary, sea_orm::DbErr> {
        let now = now_rfc3339();
        let model = alert_rule::ActiveModel {
            id: Set(new_id("alert-rule")),
            name: Set(input.name),
            severity: Set(input.severity),
            condition_json: Set(input.condition_json),
            channels_json: Set(input.channels_json),
            enabled: Set(input.enabled),
            dedupe_seconds: Set(input.dedupe_seconds),
            silenced_until: Set(input.silenced_until),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await?;
        Ok(AlertRuleSummary::from(model))
    }

    pub async fn get_rule(&self, id: &str) -> Result<Option<AlertRuleSummary>, sea_orm::DbErr> {
        alert_rule::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|model| model.map(AlertRuleSummary::from))
    }

    pub async fn list_rules(&self) -> Result<Vec<AlertRuleSummary>, sea_orm::DbErr> {
        let rows = alert_rule::Entity::find()
            .order_by_desc(alert_rule::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(AlertRuleSummary::from).collect())
    }

    pub async fn list_events(
        &self,
        filters: AlertEventFilters,
    ) -> Result<Vec<AlertEventSummary>, sea_orm::DbErr> {
        let mut query = alert_event::Entity::find();
        if let Some(value) = filters.resource_type {
            query = query.filter(alert_event::Column::ResourceType.eq(value));
        }
        if let Some(value) = filters.resource_id {
            query = query.filter(alert_event::Column::ResourceId.eq(value));
        }
        if let Some(value) = filters.failure_class {
            query = query.filter(alert_event::Column::FailureClass.eq(Some(value)));
        }
        if let Some(value) = filters.rule_id {
            query = query.filter(alert_event::Column::RuleId.eq(value));
        }
        if let Some(value) = filters.status {
            query = query.filter(alert_event::Column::Status.eq(value));
        }
        let rows = query
            .order_by_desc(alert_event::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(AlertEventSummary::from).collect())
    }

    pub async fn list_event_summaries(
        &self,
        filters: AlertEventFilters,
    ) -> Result<Vec<AlertNotificationSummary>, sea_orm::DbErr> {
        let rows = self.list_events(filters).await?;
        let mut summaries: HashMap<
            (String, String, String, Option<String>),
            AlertNotificationSummary,
        > = HashMap::new();

        for event in rows {
            let key = (
                event.rule_id.clone(),
                event.resource_type.clone(),
                event.resource_id.clone(),
                event.failure_class.clone(),
            );
            let summary = summaries
                .entry(key)
                .or_insert_with(|| AlertNotificationSummary {
                    rule_id: event.rule_id.clone(),
                    rule_name: event.rule_name.clone(),
                    severity: event.severity.clone(),
                    resource_type: event.resource_type.clone(),
                    resource_id: event.resource_id.clone(),
                    failure_class: event.failure_class.clone(),
                    latest_status: event.status.clone(),
                    latest_event_type: event.event_type.clone(),
                    latest_message: event.message.clone(),
                    event_count: 0,
                    firing_count: 0,
                    suppressed_count: 0,
                    silenced_count: 0,
                    recovered_count: 0,
                    first_seen: event.created_at.clone(),
                    last_seen: event.created_at.clone(),
                });

            summary.event_count = summary.event_count.saturating_add(1);
            match event.status.as_str() {
                "firing" => summary.firing_count = summary.firing_count.saturating_add(1),
                "suppressed" => {
                    summary.suppressed_count = summary.suppressed_count.saturating_add(1);
                }
                "silenced" => summary.silenced_count = summary.silenced_count.saturating_add(1),
                "recovered" => summary.recovered_count = summary.recovered_count.saturating_add(1),
                _ => {}
            }
            if event.created_at > summary.last_seen {
                summary.last_seen.clone_from(&event.created_at);
                summary.latest_status.clone_from(&event.status);
                summary.latest_event_type.clone_from(&event.event_type);
                summary.latest_message.clone_from(&event.message);
            }
            if event.created_at < summary.first_seen {
                summary.first_seen.clone_from(&event.created_at);
            }
        }

        let mut items: Vec<_> = summaries.into_values().collect();
        items.sort_by(|left, right| {
            right
                .last_seen
                .cmp(&left.last_seen)
                .then_with(|| left.rule_name.cmp(&right.rule_name))
                .then_with(|| left.resource_type.cmp(&right.resource_type))
                .then_with(|| left.resource_id.cmp(&right.resource_id))
        });
        Ok(items)
    }

    pub async fn count_events(&self) -> Result<AlertEventStatusCounts, sea_orm::DbErr> {
        let rows = alert_event::Entity::find().all(&self.db).await?;
        let mut counts = AlertEventStatusCounts::default();
        for row in rows {
            counts.total_events = counts.total_events.saturating_add(1);
            *counts.by_status.entry(row.status).or_insert(0) += 1;
            if row.resource_type == "script_execution_governance" {
                counts.script_failure_events = counts.script_failure_events.saturating_add(1);
                if let Some(failure_class) = row.failure_class {
                    *counts.by_failure_class.entry(failure_class).or_insert(0) += 1;
                }
            }
        }
        Ok(counts)
    }

    pub async fn get_event(&self, id: &str) -> Result<Option<AlertEventSummary>, sea_orm::DbErr> {
        alert_event::Entity::find_by_id(id.to_owned())
            .one(&self.db)
            .await
            .map(|model| model.map(AlertEventSummary::from))
    }

    pub async fn record_script_governance_recovery(
        &self,
        event_id: &str,
    ) -> Result<Option<AlertEventSummary>, sea_orm::DbErr> {
        let Some(previous) = self.get_event(event_id).await? else {
            return Ok(None);
        };
        let _ = alert_event::ActiveModel {
            id: Set(new_id("alert-event")),
            rule_id: Set(previous.rule_id.clone()),
            rule_name: Set(previous.rule_name.clone()),
            severity: Set(previous.severity.clone()),
            status: Set("recovered".to_owned()),
            event_type: Set("script_governance_recovery".to_owned()),
            resource_type: Set(previous.resource_type.clone()),
            resource_id: Set(previous.resource_id.clone()),
            failure_class: Set(previous.failure_class.clone()),
            message: Set(Some(format!("recovered from {}", previous.status))),
            dedupe_key: Set(format!(
                "{}:recovery:{}",
                previous.rule_id, previous.resource_id
            )),
            created_at: Set(now_rfc3339()),
        }
        .insert(&self.db)
        .await?;
        Ok(Some(previous))
    }

    pub async fn record_script_governance_failure(
        &self,
        resource_id: &str,
        failure_class: &str,
        message: &str,
    ) -> Result<Vec<AlertEventSummary>, sea_orm::DbErr> {
        let rules = self.list_rules().await?;
        let now = now_rfc3339();
        let mut created_events = Vec::new();
        for rule in rules.into_iter().filter(|rule| rule.enabled) {
            let Ok(condition) = serde_json::from_str::<serde_json::Value>(&rule.condition_json)
            else {
                continue;
            };
            if condition.get("type").and_then(|v| v.as_str()) != Some("script_governance_failure") {
                continue;
            }
            if condition.get("failure_class").and_then(|v| v.as_str()) != Some(failure_class) {
                continue;
            }
            let threshold = condition
                .get("threshold")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(1)
                .max(1);
            let dedupe_window_start = rfc3339_seconds_ago(rule.dedupe_seconds.max(1));
            let recent_matching_count = alert_event::Entity::find()
                .filter(alert_event::Column::RuleId.eq(rule.id.clone()))
                .filter(alert_event::Column::FailureClass.eq(Some(failure_class.to_owned())))
                .filter(alert_event::Column::CreatedAt.gte(dedupe_window_start.clone()))
                .count(&self.db)
                .await?;
            let recent_firing_count = alert_event::Entity::find()
                .filter(alert_event::Column::RuleId.eq(rule.id.clone()))
                .filter(alert_event::Column::Status.eq("firing"))
                .filter(alert_event::Column::FailureClass.eq(Some(failure_class.to_owned())))
                .filter(alert_event::Column::CreatedAt.gte(dedupe_window_start))
                .count(&self.db)
                .await?;
            let status = if let Some(silenced_until) = &rule.silenced_until {
                if silenced_until > &now {
                    "silenced"
                } else if recent_matching_count.saturating_add(1) < threshold {
                    "suppressed"
                } else if recent_firing_count > 0 {
                    "suppressed"
                } else {
                    "firing"
                }
            } else if recent_matching_count.saturating_add(1) < threshold {
                "suppressed"
            } else if recent_firing_count > 0 {
                "suppressed"
            } else {
                "firing"
            };
            let model = alert_event::ActiveModel {
                id: Set(new_id("alert-event")),
                rule_id: Set(rule.id.clone()),
                rule_name: Set(rule.name.clone()),
                severity: Set(rule.severity.clone()),
                status: Set(status.to_owned()),
                event_type: Set("script_governance_failure".to_owned()),
                resource_type: Set("script_execution_governance".to_owned()),
                resource_id: Set(resource_id.to_owned()),
                failure_class: Set(Some(failure_class.to_owned())),
                message: Set(Some(message.to_owned())),
                dedupe_key: Set(format!("{}:{resource_id}:{failure_class}", rule.id)),
                created_at: Set(now.clone()),
            }
            .insert(&self.db)
            .await?;
            created_events.push(AlertEventSummary::from(model));
        }
        Ok(created_events)
    }
}

fn rfc3339_seconds_ago(seconds: i64) -> String {
    time::OffsetDateTime::now_utc()
        .saturating_sub(time::Duration::seconds(seconds.max(1)))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

impl From<alert_rule::Model> for AlertRuleSummary {
    fn from(value: alert_rule::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            severity: value.severity,
            condition_json: value.condition_json,
            channels_json: value.channels_json,
            enabled: value.enabled,
            dedupe_seconds: value.dedupe_seconds,
            silenced_until: value.silenced_until,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<alert_event::Model> for AlertEventSummary {
    fn from(value: alert_event::Model) -> Self {
        Self {
            id: value.id,
            rule_id: value.rule_id,
            rule_name: value.rule_name,
            severity: value.severity,
            status: value.status,
            event_type: value.event_type,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            failure_class: value.failure_class,
            message: value.message,
            dedupe_key: value.dedupe_key,
            created_at: value.created_at,
        }
    }
}

impl From<alert_delivery_attempt::Model> for AlertDeliveryAttemptSummary {
    fn from(value: alert_delivery_attempt::Model) -> Self {
        Self {
            id: value.id,
            event_id: value.event_id,
            rule_id: value.rule_id,
            provider: value.provider,
            target: value.target,
            delivered: value.delivered,
            status_code: value.status_code,
            error: value.error,
            attempt: value.attempt,
            retry_state: value.retry_state,
            next_retry_at: value.next_retry_at,
            created_at: value.created_at,
        }
    }
}
