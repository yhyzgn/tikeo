//! HTTP route handlers for the management API.
#![allow(clippy::redundant_pub_crate)]

pub(crate) mod alerts;
pub(crate) mod audit;
pub(crate) mod calendars;
pub(crate) mod common;
pub(crate) mod event_sources;
pub(crate) mod gitops;
pub(crate) mod jobs;
pub(crate) mod metrics;
pub(crate) mod observability;
pub(crate) mod oidc_identity;
pub(crate) mod plugins;
pub(crate) mod raft;
pub(crate) mod roles;
pub(crate) mod scheduling;
pub(crate) mod scope;
pub(crate) mod scripts;
pub(crate) mod security;
pub(crate) mod service_accounts;
pub(crate) mod system;
pub(crate) mod topology;
pub(crate) mod users;
pub(crate) mod workers;
pub(crate) mod workflows;

pub use alerts::{
    alert_delivery_queue_status, alert_rule_delivery_status, create_alert_rule,
    list_alert_delivery_attempts, list_alert_event_summaries, list_alert_events, list_alert_rules,
    resolve_alert_event, retry_due_alert_delivery_attempts,
};
pub use audit::{export_audit_logs, list_audit_logs};
pub use calendars::{delete_calendar, list_calendars, upsert_calendar};
pub use event_sources::trigger_inbound_webhook;
pub use gitops::{diff_gitops_manifest, export_gitops_manifest};
pub use jobs::{
    cancel_job_instance, create_job, delete_job, get_job_instance, list_instance_attempts,
    list_instance_logs, list_job_instances, list_job_versions, list_jobs, rollback_job,
    stream_instance_logs, stream_instances, trigger_job, update_job,
};
pub use metrics::metrics_summary;
pub use observability::observability_status;
pub use oidc_identity::{delete_oidc_identity, list_oidc_identities, upsert_oidc_identity};
pub use plugins::{create_plugin, delete_plugin, list_plugins, update_plugin};
pub use raft::{append_entries, propose_member_change};
pub use roles::{
    create_role, delete_role, list_roles, menu_permission_catalog, permission_catalog,
    ui_action_permission_catalog, update_role,
};
pub use scheduling::job_scheduling_advice;
pub use scope::{
    create_app, create_namespace, create_secret, create_worker_pool, delete_app, delete_namespace,
    delete_secret, delete_worker_pool, list_apps, list_namespaces, list_secrets, list_worker_pools,
    update_worker_pool_quota,
};
pub use scripts::{
    create_script, delete_script, diff_script_versions, get_script, list_script_versions,
    list_scripts, preview_script_release_gate, publish_script, rollback_script, update_script,
};
pub use security::transport_security_status;
pub use service_accounts::{
    create_service_account, disable_service_account, list_service_accounts, update_service_account,
};
pub use system::{cluster_diagnostics, cluster_status, system_info};
pub use topology::{job_impact, job_topology, workflow_replay};
pub use users::{create_user, delete_user, list_users, update_user};
pub use workers::{
    claim_dispatch_queue, dispatch_queue, list_workers, stream_dispatch_queue, stream_workers,
    worker_lifecycle_history,
};
pub use workflows::{
    advance_workflow_instance, complete_workflow_shard, create_workflow, dry_run_workflow,
    get_workflow, get_workflow_instance as get_workflow_instance_route, list_workflow_shards,
    list_workflows, materialize_next_workflow_node, rebalance_workflow_shards,
    recover_workflow_node, run_workflow, stream_instance_events, update_workflow,
    validate_workflow,
};

pub(crate) use common::{client_ip, trace_id};
