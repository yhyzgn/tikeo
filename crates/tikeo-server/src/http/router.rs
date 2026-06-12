//! HTTP router assembly.

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Extension, Json, Router,
    extract::MatchedPath,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use tikeo_storage::{
    AuditLogRepository, JobInstanceAttemptRepository, JobInstanceLogRepository,
    JobInstanceRepository, JobRepository, ScriptRepository, UserRepository, WorkflowRepository,
    connect_and_migrate,
};
use utoipa::OpenApi;

use crate::cluster::StandaloneCoordinator;

use super::{
    AppState, auth,
    health::{healthz, readyz},
    openapi::ApiDoc,
    routes, sdk_api_keys, trace,
};

/// Construct the HTTP router with an explicit application state.
pub fn router_with_state(state: AppState) -> Router {
    let recorder =
        std::sync::Arc::new(metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder());
    let handle = recorder.handle();
    let metrics_recorder = Arc::clone(&recorder);

    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route(
            "/metrics",
            get(move || {
                let handle = handle.clone();
                let recorder = Arc::clone(&metrics_recorder);
                std::future::ready(metrics::with_local_recorder(&*recorder, || handle.render()))
            }),
        )
        .nest(
            "/api/v1",
            api_router()
                .layer(middleware::from_fn(record_http_metrics))
                .layer(middleware::from_fn(trace::trace_http)),
        )
        .route("/api-docs/openapi.json", get(openapi_json))
        .layer(Extension(recorder))
        .with_state(Arc::new(state))
}

pub(super) async fn router_for_database(database_url: &str) -> Result<Router> {
    let db = connect_and_migrate(database_url)
        .await
        .with_context(|| format!("failed to initialize storage at {database_url}"))?;
    Ok(router_with_state(AppState::new(
        JobRepository::new(db.clone()),
        JobInstanceRepository::new(db.clone()),
        JobInstanceLogRepository::new(db.clone()),
        JobInstanceAttemptRepository::new(db.clone()),
        UserRepository::new(db.clone()),
        ScriptRepository::new(db.clone()),
        WorkflowRepository::new(db.clone()),
        AuditLogRepository::new(db),
        crate::tunnel::WorkerRegistry::default(),
        StandaloneCoordinator::shared("standalone-http"),
    )))
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn record_http_metrics(request: Request<axum::body::Body>, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.extensions().get::<MatchedPath>().map_or_else(
        || request.uri().path().to_owned(),
        |matched| matched.as_str().to_owned(),
    );
    let started = std::time::Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16().to_string();
    let labels = [
        ("method", method.as_str().to_owned()),
        ("path", path),
        ("status", status),
    ];
    metrics::counter!("tikeo_http_requests_total", &labels).increment(1);
    metrics::histogram!("tikeo_http_request_duration_seconds", &labels)
        .record(started.elapsed().as_secs_f64());
    response
}

#[allow(clippy::too_many_lines)]
pub(super) fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/system/info", get(routes::system_info))
        .route("/metrics/summary", get(routes::metrics_summary))
        .route(
            "/security/transport",
            get(routes::transport_security_status),
        )
        .route("/observability/status", get(routes::observability_status))
        .route("/cluster", get(routes::cluster_status))
        .route("/cluster/diagnostics", get(routes::cluster_diagnostics))
        .route(
            "/raft/append-entries",
            axum::routing::post(routes::append_entries),
        )
        .route(
            "/raft/members:propose",
            axum::routing::post(routes::propose_member_change),
        )
        .route("/auth/bootstrap", get(auth::bootstrap_status))
        .route(
            "/auth/bootstrap/register",
            axum::routing::post(auth::register_bootstrap_admin),
        )
        .route("/auth/login", axum::routing::post(auth::login))
        .route("/auth/status", get(auth::status))
        .route("/auth/oidc/authorize", get(auth::oidc_authorize))
        .route("/auth/oidc/callback", get(auth::oidc_callback))
        .route(
            "/oidc-identities",
            get(routes::list_oidc_identities).post(routes::upsert_oidc_identity),
        )
        .route(
            "/oidc-identities/{id}",
            axum::routing::delete(routes::delete_oidc_identity),
        )
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", axum::routing::post(auth::logout))
        .route(
            "/auth/api-tokens",
            get(auth::list_api_tokens).post(auth::create_api_token),
        )
        .route(
            "/auth/api-tokens/{id}",
            axum::routing::delete(auth::revoke_api_token),
        )
        .route(
            "/auth/api-tokens/{id}/rotate",
            axum::routing::post(auth::rotate_api_token),
        )
        .route(
            "/plugins",
            get(routes::list_plugins).post(routes::create_plugin),
        )
        .route(
            "/plugins/{id}",
            axum::routing::patch(routes::update_plugin).delete(routes::delete_plugin),
        )
        .route("/gitops/manifest", get(routes::export_gitops_manifest))
        .route(
            "/gitops/diff",
            axum::routing::post(routes::diff_gitops_manifest),
        )
        .route(
            "/management/service-accounts",
            get(routes::list_service_accounts).post(routes::create_service_account),
        )
        .route(
            "/management/service-accounts/{id}",
            axum::routing::patch(routes::update_service_account)
                .delete(routes::disable_service_account),
        )
        .route(
            "/management/api-keys",
            get(sdk_api_keys::list_sdk_api_keys).post(sdk_api_keys::create_sdk_api_key),
        )
        .route(
            "/management/api-keys/{id}",
            axum::routing::patch(sdk_api_keys::update_sdk_api_key)
                .delete(sdk_api_keys::revoke_sdk_api_key),
        )
        .route("/roles", get(routes::list_roles).post(routes::create_role))
        .route(
            "/roles/{id}",
            axum::routing::patch(routes::update_role).delete(routes::delete_role),
        )
        .route("/permissions/catalog", get(routes::permission_catalog))
        .route(
            "/menu-permissions/catalog",
            get(routes::menu_permission_catalog),
        )
        .route(
            "/ui-action-permissions/catalog",
            get(routes::ui_action_permission_catalog),
        )
        .route(
            "/users",
            axum::routing::get(routes::list_users).post(routes::create_user),
        )
        .route(
            "/users/{id}",
            axum::routing::patch(routes::update_user).delete(routes::delete_user),
        )
        .route(
            "/scripts",
            get(routes::list_scripts).post(routes::create_script),
        )
        .route(
            "/scripts/{id}",
            get(routes::get_script)
                .patch(routes::update_script)
                .delete(routes::delete_script),
        )
        .route("/scripts/{id}/versions", get(routes::list_script_versions))
        .route(
            "/scripts/{id}/release-gate",
            get(routes::preview_script_release_gate),
        )
        .route(
            "/scripts/{id}/publish",
            axum::routing::post(routes::publish_script),
        )
        .route(
            "/scripts/{id}/rollback",
            axum::routing::post(routes::rollback_script),
        )
        .route("/scripts/{id}/diff", get(routes::diff_script_versions))
        .route(
            "/workflows",
            get(routes::list_workflows).post(routes::create_workflow),
        )
        .route(
            "/workflows/dry-run",
            axum::routing::post(routes::dry_run_workflow),
        )
        .route(
            "/workflows/{id}",
            get(routes::get_workflow).patch(routes::update_workflow),
        )
        .route(
            "/workflows/{id}/validate",
            axum::routing::post(routes::validate_workflow),
        )
        .route(
            "/workflows/{id}/run",
            axum::routing::post(routes::run_workflow),
        )
        .route(
            "/workflow-instances/materialize-next",
            axum::routing::post(routes::materialize_next_workflow_node),
        )
        .route(
            "/workflow-instances/{id}",
            get(routes::get_workflow_instance_route),
        )
        .route(
            "/workflow-instances/{id}/replay",
            get(routes::workflow_replay),
        )
        .route(
            "/workflow-instances/{id}/advance",
            axum::routing::post(routes::advance_workflow_instance),
        )
        .route(
            "/workflow-instances/{id}/recover",
            axum::routing::post(routes::recover_workflow_node),
        )
        .route(
            "/workflow-instances/{id}/shards",
            get(routes::list_workflow_shards),
        )
        .route(
            "/workflow-instances/{id}/shards/rebalance",
            axum::routing::post(routes::rebalance_workflow_shards),
        )
        .route(
            "/workflow-shards/{id}/complete",
            axum::routing::post(routes::complete_workflow_shard),
        )
        .route(
            "/events/instances/{id}/stream",
            get(routes::stream_instance_events),
        )
        .route(
            "/events/webhooks/{job}",
            axum::routing::post(routes::trigger_inbound_webhook),
        )
        .route(
            "/namespaces",
            get(routes::list_namespaces).post(routes::create_namespace),
        )
        .route(
            "/namespaces/{id}",
            axum::routing::delete(routes::delete_namespace),
        )
        .route("/apps", get(routes::list_apps).post(routes::create_app))
        .route("/apps/{id}", axum::routing::delete(routes::delete_app))
        .route(
            "/secrets",
            get(routes::list_secrets).post(routes::create_secret),
        )
        .route(
            "/secrets/{id}",
            axum::routing::delete(routes::delete_secret),
        )
        .route(
            "/calendars",
            get(routes::list_calendars).post(routes::upsert_calendar),
        )
        .route(
            "/calendars/{id}",
            axum::routing::delete(routes::delete_calendar),
        )
        .route(
            "/worker-pools",
            get(routes::list_worker_pools).post(routes::create_worker_pool),
        )
        .route(
            "/worker-pools/{id}",
            axum::routing::delete(routes::delete_worker_pool),
        )
        .route(
            "/worker-pools/{id}/quota",
            axum::routing::patch(routes::update_worker_pool_quota),
        )
        .route("/jobs", get(routes::list_jobs).post(routes::create_job))
        .route("/jobs/topology", get(routes::job_topology))
        .route("/jobs/{job}/impact", get(routes::job_impact))
        .route(
            "/jobs/{job}/notification-bindings",
            get(routes::list_job_notification_bindings)
                .post(routes::create_job_notification_binding),
        )
        .route(
            "/jobs/{job}/notification-bindings:validate",
            axum::routing::post(routes::validate_job_notification_binding),
        )
        .route(
            "/jobs/{job}/notification-bindings:preview",
            axum::routing::post(routes::preview_job_notification_binding),
        )
        .route(
            "/jobs/{job}/notification-bindings/{binding}",
            get(routes::get_job_notification_binding)
                .patch(routes::update_job_notification_binding)
                .delete(routes::delete_job_notification_binding),
        )
        .route(
            "/jobs/{job_action}",
            axum::routing::post(routes::trigger_job)
                .patch(routes::update_job)
                .delete(routes::delete_job),
        )
        .route(
            "/jobs/{job}/scheduling-advice",
            get(routes::job_scheduling_advice),
        )
        .route("/jobs/{job}/versions", get(routes::list_job_versions))
        .route(
            "/jobs/{job}/rollback",
            axum::routing::post(routes::rollback_job),
        )
        .route("/jobs/{job}/instances", get(routes::list_job_instances))
        .route("/instances/stream", get(routes::stream_instances))
        .route("/instances/{instance}", get(routes::get_job_instance))
        .route(
            "/instances/{instance}/cancel",
            axum::routing::post(routes::cancel_job_instance),
        )
        .route(
            "/instances/{instance}/logs",
            get(routes::list_instance_logs),
        )
        .route(
            "/instances/{instance}/logs/stream",
            get(routes::stream_instance_logs),
        )
        .route(
            "/instances/{instance}/attempts",
            get(routes::list_instance_attempts),
        )
        .route("/workers", get(routes::list_workers))
        .route("/workers/stream", get(routes::stream_workers))
        .route("/workers/history", get(routes::worker_lifecycle_history))
        .route("/dispatch-queue", get(routes::dispatch_queue))
        .route("/dispatch-queue/stream", get(routes::stream_dispatch_queue))
        .route(
            "/dispatch-queue:claim",
            axum::routing::post(routes::claim_dispatch_queue),
        )
        .route("/audit-logs", get(routes::list_audit_logs))
        .route("/audit-logs:export", get(routes::export_audit_logs))
        .route(
            "/notification-channel-types",
            get(routes::list_notification_channel_types),
        )
        .route(
            "/notification-channels",
            get(routes::list_notification_channels).post(routes::create_notification_channel),
        )
        .route(
            "/notification-channels/{id}",
            get(routes::get_notification_channel)
                .patch(routes::update_notification_channel)
                .delete(routes::delete_notification_channel),
        )
        .route(
            "/notification-channels/{id}/test-send",
            axum::routing::post(routes::test_notification_channel),
        )
        .route(
            "/notification-policies",
            get(routes::list_notification_policies).post(routes::create_notification_policy),
        )
        .route(
            "/notification-policies/{policy_action}",
            get(routes::get_notification_policy)
                .patch(routes::update_notification_policy)
                .delete(routes::delete_notification_policy)
                .post(routes::validate_notification_policy),
        )
        .route(
            "/notification-templates",
            get(routes::list_notification_templates).post(routes::create_notification_template),
        )
        .route(
            "/notification-templates/{template_action}",
            get(routes::get_notification_template)
                .patch(routes::update_notification_template)
                .delete(routes::delete_notification_template)
                .post(routes::render_notification_template),
        )
        .route(
            "/notification-templates/{id}/render",
            axum::routing::post(routes::render_notification_template_by_id),
        )
        .route(
            "/notification-messages",
            get(routes::list_notification_messages),
        )
        .route(
            "/notification-messages/{id}/trace",
            get(routes::get_notification_message_trace),
        )
        .route(
            "/notification-delivery-attempts",
            get(routes::list_notification_delivery_attempts),
        )
        .route(
            "/notification-delivery-attempts:queue-status",
            get(routes::notification_delivery_queue_status),
        )
        .route(
            "/notification-delivery-attempts:retry-due",
            axum::routing::post(routes::retry_due_notification_delivery_attempts),
        )
        .route(
            "/alert-rules",
            get(routes::list_alert_rules).post(routes::create_alert_rule),
        )
        .route(
            "/alert-rules/{id}/delivery-status",
            get(routes::alert_rule_delivery_status),
        )
        .route(
            "/alert-delivery-attempts",
            get(routes::list_alert_delivery_attempts),
        )
        .route(
            "/alert-delivery-attempts:queue-status",
            get(routes::alert_delivery_queue_status),
        )
        .route(
            "/alert-delivery-attempts:retry-due",
            axum::routing::post(routes::retry_due_alert_delivery_attempts),
        )
        .route("/alert-events", get(routes::list_alert_events))
        .route(
            "/alert-events/{id}/resolve",
            axum::routing::post(routes::resolve_alert_event),
        )
        .route(
            "/alert-events:summary",
            axum::routing::get(routes::list_alert_event_summaries),
        )
}
