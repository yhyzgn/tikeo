#![allow(missing_docs)]

use super::delivery::{
    NotificationProviderClient, notification_channel_from_delivery_config, parse_json_object,
};
use super::provider_templates::{email_alert_payload_from_message, feishu_payload};
use super::*;
use crate::alert::{AlertDeliveryPolicy, NotificationChannel};
use crate::cluster::StandaloneCoordinator;
use tikeo_core::{ExecutionMode, TriggerType};
use tikeo_storage::{
    AuditLogRepository, CreateJob, CreateJobInstance, CreateNotificationChannel,
    CreateNotificationPolicy, CreateNotificationTemplate, JobInstanceRepository,
    NotificationChannelDeliveryConfig, NotificationDeliveryAttemptFilters,
    NotificationMessageFilters, NotificationTemplateRepository, connect_and_migrate,
};

include!("tests/part_01.rs");
include!("tests/part_02.rs");
