use crate::cluster::{
    ClusterMode, ClusterRole, ClusterStatus, StandaloneCoordinator, StaticCoordinator,
    coordinator_from_config_with_storage,
};
use axum::{Router, body::Body, http::Request, routing::get};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tikee_config::{
    ClusterConfig, ClusterModeConfig, ClusterPeerConfig, ScriptGovernanceConfig, TlsEndpointConfig,
};
use tikee_core::{ExecutionMode, TriggerType};
use tikee_proto::worker::v1::RegisterWorker;
use tikee_storage::{
    AppendJobInstanceLog, AuditLogRepository, CompleteWorkflowShardInput, CreateAuditLog,
    CreateJob, CreateJobInstance, CreateWorkflow, JobInstanceAttemptRepository,
    JobInstanceLogRepository, JobInstanceRepository, JobRepository, RaftRepository,
    ScriptRepository, UserRepository, WorkflowDefinition, WorkflowNodeSpec, WorkflowRepository,
    connect_and_migrate,
};
use url::Url;

const ADMIN_LOGIN: &str = r#"{"username":"tikee_init","password":"Tikee@2026!"}"#;
use tower::ServiceExt;

use crate::http::{AppState, router_with_state, serve_listener_with_state};

include!("tests/part_01.rs");
include!("tests/part_02.rs");
include!("tests/part_03.rs");
include!("tests/part_04.rs");
include!("tests/part_05.rs");
