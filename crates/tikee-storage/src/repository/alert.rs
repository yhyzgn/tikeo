#![allow(missing_docs)]

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::entities::{alert_event, alert_rule};

use super::util::{new_id, now_rfc3339};

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

    pub async fn record_script_governance_failure(
        &self,
        resource_id: &str,
        failure_class: &str,
        message: &str,
    ) -> Result<(), sea_orm::DbErr> {
        let rules = self.list_rules().await?;
        let now = now_rfc3339();
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
                .unwrap_or(1);
            let firing_count = alert_event::Entity::find()
                .filter(alert_event::Column::RuleId.eq(rule.id.clone()))
                .filter(alert_event::Column::Status.eq("firing"))
                .filter(alert_event::Column::FailureClass.eq(Some(failure_class.to_owned())))
                .count(&self.db)
                .await?;
            let status = if let Some(silenced_until) = &rule.silenced_until {
                if silenced_until > &now {
                    "silenced"
                } else if firing_count.saturating_add(1) < threshold {
                    continue;
                } else if firing_count > 0 {
                    "suppressed"
                } else {
                    "firing"
                }
            } else if firing_count.saturating_add(1) < threshold {
                continue;
            } else if firing_count > 0 {
                "suppressed"
            } else {
                "firing"
            };
            let _ = alert_event::ActiveModel {
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
        }
        Ok(())
    }
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
