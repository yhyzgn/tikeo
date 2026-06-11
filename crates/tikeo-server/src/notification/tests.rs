#![allow(missing_docs)]

use super::*;
use tikeo_core::{ExecutionMode, TriggerType};
use tikeo_storage::{
    AuditLogRepository, CreateJob, CreateJobInstance, CreateNotificationChannel,
    CreateNotificationPolicy, CreateNotificationTemplate, JobInstanceRepository,
    NotificationDeliveryAttemptFilters, NotificationMessageFilters, NotificationTemplateRepository,
    connect_and_migrate,
};

include!("tests/part_01.rs");
include!("tests/part_02.rs");
