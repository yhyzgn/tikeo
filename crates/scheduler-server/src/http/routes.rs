//! HTTP route handlers for the management API.
#![allow(clippy::redundant_pub_crate)]

pub(crate) mod audit;
pub(crate) mod common;
pub(crate) mod jobs;
pub(crate) mod scripts;
pub(crate) mod system;
pub(crate) mod users;
pub(crate) mod workflows;

pub use audit::list_audit_logs;
pub use jobs::{
    create_job, get_job_instance, list_instance_attempts, list_instance_logs, list_job_instances,
    list_jobs, trigger_job,
};
pub use scripts::{
    create_script, delete_script, diff_script_versions, get_script, list_script_versions,
    list_scripts, update_script,
};
pub use system::{cluster_status, system_info};
pub use users::{create_user, delete_user, list_users, update_user};
pub use workflows::{
    create_workflow, get_workflow, get_workflow_instance as get_workflow_instance_route,
    list_workflows, run_workflow, stream_instance_events, validate_workflow,
};

pub(crate) use common::client_ip;
