#![allow(missing_docs)]

use serde::Serialize;
use tikeo_storage::ScopeRepository;
use utoipa::ToSchema;

use crate::{
    http::{AppState, error::ApiError},
    notification::validate_notification_template_tokens,
};

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationChannelTypeSummary {
    pub r#type: String,
    pub label: String,
    pub category: String,
    pub target_kind: String,
    pub description: String,
    pub required_config_keys: Vec<String>,
    pub required_target_keys: Vec<String>,
    pub secret_config_keys: Vec<String>,
    pub supports_test_send: bool,
    pub plugin_provided: bool,
    pub template: serde_json::Value,
}

pub(super) fn builtin_channel_types() -> Vec<NotificationChannelTypeSummary> {
    [
        (
            "webhook",
            "Generic Webhook",
            "webhook",
            "HTTP webhook",
            Vec::<&str>::new(),
            vec!["url"],
            vec!["authorization"],
        ),
        (
            "slack",
            "Slack Incoming Webhook",
            "office_bot",
            "Slack robot webhook",
            Vec::<&str>::new(),
            vec!["url"],
            vec![],
        ),
        (
            "dingtalk",
            "DingTalk Robot",
            "office_bot",
            "DingTalk robot webhook",
            Vec::<&str>::new(),
            vec!["url"],
            vec!["signingKey"],
        ),
        (
            "feishu",
            "Feishu/Lark Bot",
            "office_bot",
            "Feishu/Lark bot webhook",
            Vec::<&str>::new(),
            vec!["url"],
            vec!["signingKey"],
        ),
        (
            "wechat_work",
            "WeCom Bot",
            "office_bot",
            "WeChat Work/WeCom robot webhook",
            Vec::<&str>::new(),
            vec!["url"],
            vec![],
        ),
        (
            "pagerduty",
            "PagerDuty Events API",
            "incident",
            "PagerDuty Events v2 integration",
            Vec::<&str>::new(),
            vec!["routingKey"],
            vec!["routingKey"],
        ),
        (
            "email",
            "SMTP Email",
            "email",
            "SMTP email delivery",
            vec!["to", "host"],
            vec!["host"],
            vec![
                "password",
                "passwordSecretRef",
                "smtpUrl",
                "smtpUrlSecretRef",
            ],
        ),
    ]
    .into_iter()
    .map(
        |(r#type, label, category, description, required, required_target, secret)| {
            NotificationChannelTypeSummary {
                r#type: r#type.to_owned(),
                label: label.to_owned(),
                category: category.to_owned(),
                target_kind: if r#type == "email" {
                    "email"
                } else {
                    "webhook"
                }
                .to_owned(),
                description: description.to_owned(),
                required_config_keys: required.into_iter().map(str::to_owned).collect(),
                required_target_keys: required_target.into_iter().map(str::to_owned).collect(),
                secret_config_keys: secret.into_iter().map(str::to_owned).collect(),
                supports_test_send: true,
                plugin_provided: false,
                template: builtin_channel_template(r#type),
            }
        },
    )
    .collect()
}

fn builtin_channel_template(provider: &str) -> serde_json::Value {
    let variables = serde_json::json!([
        "{{subject}}",
        "{{body}}",
        "{{eventType}}",
        "{{resourceType}}",
        "{{resourceId}}",
        "{{severity}}",
        "{{messageId}}",
        "{{policyId}}",
        "{{dedupeKey}}",
        "{{triggeredAt}}",
        "{{createdAt}}",
        "{{namespace}}",
        "{{app}}",
        "{{jobId}}",
        "{{jobName}}",
        "{{instanceId}}",
        "{{status}}",
        "{{triggerType}}",
        "{{executionMode}}",
        "{{startedAt}}",
        "{{finishedAt}}",
        "{{workerId}}",
        "{{operatorName}}",
        "{{operatorType}}",
        "{{reason}}",
        "{{logsUrl}}",
        "{{consoleUrl}}",
        "{{templateRef}}",
        "{{templateKey}}"
    ]);
    match provider {
        "slack" => serde_json::json!({
            "defaultMessageType": "text",
            "messageTypes": [
                {"id": "text", "label": "Text", "description": "Slack plain text incoming-webhook payload.", "templateFields": [{"key":"text","label":"Text template","type":"textarea","required":true}]},
                {"id": "blockKit", "label": "Block Kit", "description": "Slack blocks payload with fallback text.", "templateFields": [{"key":"text","label":"Fallback text","type":"textarea","required":true},{"key":"blocks","label":"Blocks JSON template","type":"textarea","required":true}]},
                {"id": "attachments", "label": "Attachments", "description": "Slack legacy attachments payload with fallback text.", "templateFields": [{"key":"text","label":"Fallback text","type":"textarea","required":true},{"key":"attachments","label":"Attachments JSON template","type":"textarea","required":true}]}
            ],
            "configFields": [{"key":"threadTs","label":"Thread timestamp","type":"string"}],
            "secretFields": [{"key":"url","label":"Webhook URL","type":"string","required":true,"secret":true}],
            "templateVariables": variables,
            "docs": [{"label":"Slack Incoming Webhooks","url":"https://docs.slack.dev/messaging/sending-messages-using-incoming-webhooks/"}]
        }),
        "dingtalk" => serde_json::json!({
            "defaultMessageType": "markdown",
            "messageTypes": [
                {"id":"text","label":"Text","description":"DingTalk text robot message.","templateFields":[{"key":"content","label":"Content template","type":"textarea","required":true}]},
                {"id":"markdown","label":"Markdown","description":"DingTalk markdown robot message.","templateFields":[{"key":"title","label":"Title template","type":"string","required":true},{"key":"text","label":"Markdown template","type":"textarea","required":true}]},
                {"id":"link","label":"Link","description":"DingTalk link message.","templateFields":[{"key":"title","label":"Title template","type":"string","required":true},{"key":"text","label":"Text template","type":"textarea","required":true},{"key":"messageUrl","label":"Message URL","type":"url","required":true},{"key":"picUrl","label":"Picture URL","type":"url"}]},
                {"id":"actionCard","label":"ActionCard","description":"DingTalk action card message.","templateFields":[{"key":"title","label":"Title template","type":"string","required":true},{"key":"text","label":"Markdown template","type":"textarea","required":true},{"key":"singleTitle","label":"Button title","type":"string"},{"key":"singleURL","label":"Button URL","type":"url"},{"key":"btnOrientation","label":"Button orientation","type":"select","options":[{"value":"0","label":"Vertical"},{"value":"1","label":"Horizontal"}]},{"key":"btns","label":"Buttons JSON template","type":"textarea"}]},
                {"id":"feedCard","label":"FeedCard","description":"DingTalk feed card message.","templateFields":[{"key":"links","label":"Links JSON template","type":"textarea","required":true}]}
            ],
            "configFields": [{"key":"atMobiles","label":"@ mobile numbers","type":"tags"},{"key":"atUserIds","label":"@ user IDs","type":"tags"},{"key":"isAtAll","label":"@ all members","type":"boolean","defaultValue":false}],
            "secretFields": [{"key":"url","label":"Webhook URL","type":"string","required":true,"secret":true},{"key":"signingKey","label":"Signing secret","type":"string","secret":true}],
            "templateVariables": variables,
            "docs": [{"label":"DingTalk custom robot","url":"https://open.dingtalk.com/document/group/custom-robot-access"},{"label":"DingTalk robot message types","url":"https://open.dingtalk.com/document/development/robot-message-type"}]
        }),
        "feishu" => serde_json::json!({
            "defaultMessageType": "text",
            "messageTypes": [
                {"id":"text","label":"Text","description":"Feishu/Lark plain text custom-bot message.","templateFields":[{"key":"text","label":"Text template","type":"textarea","required":true}]},
                {"id":"post","label":"Rich text post","description":"Feishu/Lark post message.","templateFields":[{"key":"title","label":"Title template","type":"string","required":true},{"key":"content","label":"Post content JSON template","type":"textarea","required":true}]},
                {"id":"image","label":"Image","description":"Feishu/Lark image message using image_key.","templateFields":[{"key":"imageKey","label":"Image key template","type":"string","required":true}]},
                {"id":"share_chat","label":"Share chat","description":"Feishu/Lark share_chat message using share_chat_id.","templateFields":[{"key":"shareChatId","label":"Share chat ID template","type":"string","required":true}]},
                {"id":"interactive","label":"Interactive card","description":"Feishu/Lark card message sent through a custom bot.","templateFields":[{"key":"card","label":"Card JSON template","type":"textarea","required":true}]}
            ],
            "configFields": [],
            "secretFields": [{"key":"url","label":"Webhook URL","type":"string","required":true,"secret":true},{"key":"signingKey","label":"Signing secret","type":"string","secret":true}],
            "templateVariables": variables,
            "docs": [{"label":"Feishu custom bot","url":"https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot"},{"label":"Feishu card with custom bot","url":"https://open.feishu.cn/document/common-capabilities/message-card/getting-started/send-message-cards-with-a-custom-bot"}]
        }),
        "wechat_work" => serde_json::json!({
            "defaultMessageType": "markdown",
            "messageTypes": [
                {"id":"text","label":"Text","description":"WeCom text group-robot message.","templateFields":[{"key":"content","label":"Content template","type":"textarea","required":true}]},
                {"id":"markdown","label":"Markdown","description":"WeCom markdown group-robot message.","templateFields":[{"key":"content","label":"Markdown template","type":"textarea","required":true}]},
                {"id":"markdown_v2","label":"Markdown v2","description":"WeCom markdown_v2 group-robot message.","templateFields":[{"key":"content","label":"Markdown v2 template","type":"textarea","required":true}]},
                {"id":"image","label":"Image","description":"WeCom image message with base64 and md5.","templateFields":[{"key":"base64","label":"Image base64 template","type":"textarea","required":true},{"key":"md5","label":"Image MD5 template","type":"string","required":true}]},
                {"id":"news","label":"News","description":"WeCom news/articles message.","templateFields":[{"key":"articles","label":"Articles JSON template","type":"textarea","required":true}]},
                {"id":"file","label":"File","description":"WeCom file message using media_id from upload API.","templateFields":[{"key":"media_id","label":"Media ID template","type":"string","required":true}]},
                {"id":"voice","label":"Voice","description":"WeCom voice message using media_id from upload API.","templateFields":[{"key":"media_id","label":"Media ID template","type":"string","required":true}]},
                {"id":"template_card","label":"Template card","description":"WeCom template_card rich notice message.","templateFields":[{"key":"templateCard","label":"Template card JSON template","type":"textarea","required":true}]}
            ],
            "configFields": [{"key":"mentionedList","label":"Mentioned user IDs","type":"tags"},{"key":"mentionedMobileList","label":"Mentioned mobile numbers","type":"tags"}],
            "secretFields": [{"key":"url","label":"Webhook URL","type":"string","required":true,"secret":true}],
            "templateVariables": variables,
            "docs": [{"label":"WeCom group robot","url":"https://developer.work.weixin.qq.com/document/path/91770"}]
        }),
        "pagerduty" => serde_json::json!({
            "defaultMessageType": "trigger",
            "messageTypes": [
                {"id":"trigger","label":"Trigger","description":"Create or update a PagerDuty alert.","templateFields":[{"key":"summary","label":"Summary template","type":"string","required":true}]},
                {"id":"acknowledge","label":"Acknowledge","description":"Acknowledge an existing PagerDuty event by dedup key.","templateFields":[]},
                {"id":"resolve","label":"Resolve","description":"Resolve an existing PagerDuty event by dedup key.","templateFields":[]}
            ],
            "configFields": [{"key":"dedupKey","label":"Dedup key template","type":"string"},{"key":"source","label":"Source template","type":"string","defaultValue":"tikeo"},{"key":"severity","label":"PagerDuty severity","type":"select","options":[{"value":"info","label":"Info"},{"value":"warning","label":"Warning"},{"value":"error","label":"Error"},{"value":"critical","label":"Critical"}]},{"key":"timestamp","label":"Event timestamp template","type":"string"},{"key":"component","label":"Component template","type":"string"},{"key":"group","label":"Group template","type":"string"},{"key":"class","label":"Class template","type":"string"},{"key":"client","label":"Client template","type":"string"},{"key":"clientUrl","label":"Client URL template","type":"url"},{"key":"links","label":"Links JSON template","type":"textarea"},{"key":"images","label":"Images JSON template","type":"textarea"},{"key":"customDetails","label":"Custom details JSON template","type":"textarea"}],
            "secretFields": [{"key":"routingKey","label":"Routing / integration key","type":"string","required":true,"secret":true}],
            "templateVariables": variables,
            "docs": [{"label":"PagerDuty Events API v2","url":"https://developer.pagerduty.com/docs/events-api-v2-overview"},{"label":"Send an alert event","url":"https://developer.pagerduty.com/docs/send-alert-event"}]
        }),
        "email" => serde_json::json!({
            "defaultMessageType": "plain",
            "messageTypes": [
                {"id":"plain","label":"Plain text","description":"Text/plain email body.","templateFields":[{"key":"subject","label":"Subject template","type":"string","required":true},{"key":"body","label":"Body template","type":"textarea","required":true}]},
                {"id":"html","label":"HTML template","description":"Schema-only HTML shape; current runtime falls back to text body.","templateFields":[{"key":"subject","label":"Subject template","type":"string","required":true},{"key":"html","label":"HTML template","type":"textarea"},{"key":"body","label":"Text fallback template","type":"textarea","required":true}]}
            ],
            "configFields": [
                {"key":"to","label":"Recipients","type":"emailList","required":true},
                {"key":"from","label":"From address","type":"string"},
                {"key":"host","label":"SMTP host","type":"string","required":true},
                {"key":"port","label":"SMTP port","type":"string","required":true,"defaultValue":"465"},
                {"key":"username","label":"SMTP username","type":"string"},
                {"key":"auth","label":"SMTP auth","type":"boolean","defaultValue":true},
                {"key":"ssl","label":"SSL/TLS","type":"boolean","defaultValue":true},
                {"key":"starttls","label":"STARTTLS","type":"boolean","defaultValue":false}
            ],
            "secretFields": [
                {"key":"password","label":"SMTP password","type":"string","secret":true},
                {"key":"smtpUrl","label":"SMTP URL (optional compatible override)","type":"string","secret":true}
            ],
            "templateVariables": variables,
            "docs": [{"label":"SMTP RFC 5321","url":"https://datatracker.ietf.org/doc/rfc5321/"},{"label":"Internet Message Format RFC 5322","url":"https://datatracker.ietf.org/doc/rfc5322/"}]
        }),
        _ => serde_json::json!({
            "defaultMessageType": "json",
            "messageTypes": [{"id":"json","label":"JSON payload","description":"Provider-neutral JSON webhook body.","templateFields":[{"key":"body","label":"JSON body template","type":"textarea","required":true}]}],
            "configFields": [],
            "secretFields": [{"key":"url","label":"Webhook URL","type":"string","required":true,"secret":true},{"key":"authorization","label":"Authorization header","type":"string","secret":true}],
            "templateVariables": variables,
            "docs": [{"label":"HTTP semantics RFC 9110","url":"https://datatracker.ietf.org/doc/rfc9110/"}]
        }),
    }
}

pub(super) struct ChannelValidationInput<'a> {
    pub(super) scope_type: &'a str,
    pub(super) namespace: Option<&'a str>,
    pub(super) app: Option<&'a str>,
    pub(super) worker_pool: Option<&'a str>,
    pub(super) provider: &'a str,
    pub(super) name: &'a str,
    pub(super) config: &'a serde_json::Value,
    pub(super) secret_refs: &'a serde_json::Value,
}

pub(super) async fn validate_channel_request(
    state: &AppState,
    input: ChannelValidationInput<'_>,
) -> Result<(), ApiError> {
    let ChannelValidationInput {
        scope_type,
        namespace,
        app,
        worker_pool,
        provider,
        name,
        config,
        secret_refs,
    } = input;
    if !matches!(scope_type, "global" | "namespace" | "app" | "worker_pool") {
        return Err(ApiError::bad_request(
            "scopeType must be global, namespace, app, or worker_pool",
        ));
    }
    if name.trim().is_empty() {
        return Err(ApiError::bad_request(
            "notification channel name is required",
        ));
    }
    validate_channel_scope(state, scope_type, namespace, app, worker_pool).await?;
    if !valid_slug(provider) {
        return Err(ApiError::bad_request("provider must be a lowercase slug"));
    }
    let provider_supported = builtin_channel_types()
        .iter()
        .any(|item| item.r#type == provider)
        || state
            .plugins
            .resolve_alert_channel_type(provider)
            .await
            .map_err(|error| ApiError::storage(&error))?
            .is_some();
    if !provider_supported {
        return Err(ApiError::bad_request(format!(
            "notification provider is not registered: {provider}"
        )));
    }
    validate_no_raw_secret_config(provider, config)?;
    validate_provider_message_template_for_state(state, provider, config).await?;
    if provider == "email" {
        if !json_field_present(config, "to") && !json_field_present(config, "recipients") {
            return Err(ApiError::bad_request(
                "email channel requires to or recipients",
            ));
        }
        if !json_field_present_any(config, &["smtpUrl", "smtp_url", "url"])
            && !json_field_present_any(secret_refs, &["smtpUrl", "smtp_url", "url"])
            && !json_field_present_any(config, &["smtpUrlSecretRef", "smtp_url_secret_ref"])
            && !json_field_present_any(secret_refs, &["smtpUrlSecretRef", "smtp_url_secret_ref"])
            && !json_field_present(config, "host")
        {
            return Err(ApiError::bad_request(
                "email channel requires host or smtpUrl/smtpUrlSecretRef",
            ));
        }
        return Ok(());
    }
    if matches!(provider, "pagerduty") {
        if !json_field_present_any(
            config,
            &[
                "routingKey",
                "routing_key",
                "integrationKey",
                "integration_key",
            ],
        ) && !json_field_present_any(
            secret_refs,
            &[
                "routingKey",
                "routing_key",
                "integrationKey",
                "integration_key",
            ],
        ) {
            return Err(ApiError::bad_request(
                "pagerduty channel requires routingKey or integrationKey",
            ));
        }
        return Ok(());
    }
    if !json_field_present_any(config, &["url", "webhookUrl", "webhook_url"])
        && !json_field_present_any(secret_refs, &["url", "webhookUrl", "webhook_url"])
    {
        return Err(ApiError::bad_request("webhook-style channel requires url"));
    }
    Ok(())
}

async fn validate_channel_scope(
    state: &AppState,
    scope_type: &str,
    namespace: Option<&str>,
    app: Option<&str>,
    worker_pool: Option<&str>,
) -> Result<(), ApiError> {
    let required = |value: Option<&str>| value.is_some_and(|item| !item.trim().is_empty());
    match scope_type {
        "global" => Ok(()),
        "namespace" if !required(namespace) => {
            Err(ApiError::bad_request("namespace scope requires namespace"))
        }
        "app" if !required(namespace) || !required(app) => Err(ApiError::bad_request(
            "app scope requires namespace and app",
        )),
        "worker_pool" if !required(namespace) || !required(app) || !required(worker_pool) => Err(
            ApiError::bad_request("worker_pool scope requires namespace, app, and workerPool"),
        ),
        _ => {
            let scopes = ScopeRepository::new(state.users.db());
            if let Some(namespace) = namespace
                && !scopes
                    .list_namespaces()
                    .await
                    .map_err(|error| ApiError::storage(&error))?
                    .iter()
                    .any(|item| item.name == namespace)
            {
                return Err(ApiError::bad_request("namespace does not exist"));
            }
            if let (Some(namespace), Some(app)) = (namespace, app) {
                let apps = scopes
                    .list_apps(Some(namespace))
                    .await
                    .map_err(|error| ApiError::storage(&error))?;
                if !apps.iter().any(|item| item.name == app) {
                    return Err(ApiError::bad_request(
                        "app does not exist in selected namespace",
                    ));
                }
            }
            if let (Some(namespace), Some(app), Some(worker_pool)) = (namespace, app, worker_pool) {
                let pools = scopes
                    .list_worker_pools(Some(namespace), Some(app))
                    .await
                    .map_err(|error| ApiError::storage(&error))?;
                if !pools.iter().any(|item| item.name == worker_pool) {
                    return Err(ApiError::bad_request(
                        "workerPool does not exist in selected namespace/app",
                    ));
                }
            }
            Ok(())
        }
    }
}

fn validate_no_raw_secret_config(
    provider: &str,
    config: &serde_json::Value,
) -> Result<(), ApiError> {
    let forbidden: &[&str] = match provider {
        "pagerduty" | "pager_duty" => &[
            "routingKey",
            "routing_key",
            "integrationKey",
            "integration_key",
        ],
        "dingtalk" | "feishu" => &[
            "signingKey",
            "signing_key",
            "secret",
            "secretKey",
            "secret_key",
        ],
        "email" => &["password"],
        "webhook" => &["authorization"],
        _ => &[],
    };
    if forbidden.iter().any(|key| json_field_present(config, key)) {
        return Err(ApiError::bad_request(format!(
            "{provider} secret fields must be configured through secretRefs"
        )));
    }
    if provider == "webhook" && has_sensitive_raw_header(config) {
        return Err(ApiError::bad_request(
            "webhook secret headers must be configured through secretRefs.headers",
        ));
    }
    Ok(())
}

fn has_sensitive_raw_header(config: &serde_json::Value) -> bool {
    config
        .get("headers")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|headers| {
            headers
                .iter()
                .any(|(name, value)| is_sensitive_header_name(name) && json_value_present(value))
        })
}

fn is_sensitive_header_name(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    normalized == "authorization"
        || normalized == "proxy-authorization"
        || normalized == "x-api-key"
        || normalized == "x-auth-token"
        || normalized == "x-tikeo-api-key"
        || normalized.contains("secret")
        || normalized.contains("token")
}

fn json_value_present(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::String(item) => !item.trim().is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Object(items) => !items.is_empty(),
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => true,
    }
}

pub(super) async fn validate_provider_message_template_for_state(
    state: &AppState,
    provider: &str,
    config: &serde_json::Value,
) -> Result<(), ApiError> {
    if is_builtin_provider(provider) {
        return validate_provider_message_template(provider, config);
    }
    let Some(plugin_type) = state
        .plugins
        .resolve_alert_channel_type(provider)
        .await
        .map_err(|error| ApiError::storage(&error))?
    else {
        return Err(ApiError::bad_request(format!(
            "notification provider is not registered: {provider}"
        )));
    };
    validate_provider_message_template_from_metadata(provider, &plugin_type.template, config, false)
}

pub(super) fn validate_provider_message_template(
    provider: &str,
    config: &serde_json::Value,
) -> Result<(), ApiError> {
    validate_provider_message_template_from_metadata(
        provider,
        &builtin_channel_template(provider),
        config,
        true,
    )
}

fn validate_provider_message_template_from_metadata(
    provider: &str,
    template_meta: &serde_json::Value,
    config: &serde_json::Value,
    enforce_builtin_shapes: bool,
) -> Result<(), ApiError> {
    let template = config.get("template");
    if let Some(template) = template {
        validate_notification_template_tokens(template).map_err(|error| {
            ApiError::bad_request(format!("notification template is unsafe: {error}"))
        })?;
    }
    let Some(message_types) = template_meta
        .get("messageTypes")
        .and_then(serde_json::Value::as_array)
        .filter(|items| !items.is_empty())
    else {
        return Ok(());
    };
    let message_type = config
        .get("messageType")
        .or_else(|| config.get("message_type"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            template_meta
                .get("defaultMessageType")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("json");
    let Some(message_meta) = message_types.iter().find(|item| item["id"] == message_type) else {
        return Err(ApiError::bad_request(format!(
            "{provider} messageType is not supported: {message_type}"
        )));
    };
    let Some(template) = template else {
        return Ok(());
    };
    for field in message_meta
        .get("templateFields")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        if field
            .get("required")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            let Some(key) = field.get("key").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !json_field_present(template, key) {
                return Err(ApiError::bad_request(format!(
                    "{provider} {message_type} template requires {key}"
                )));
            }
        }
    }
    if enforce_builtin_shapes {
        validate_provider_template_field_shapes(provider, message_type, template)?;
    }
    Ok(())
}

pub(super) fn is_builtin_provider(provider: &str) -> bool {
    builtin_channel_types()
        .iter()
        .any(|item| item.r#type == provider)
}

fn validate_provider_template_field_shapes(
    provider: &str,
    message_type: &str,
    template: &serde_json::Value,
) -> Result<(), ApiError> {
    match (provider, message_type) {
        ("slack", "blockKit" | "blocks") => {
            validate_json_field_kind(template, "blocks", JsonFieldKind::Array)?;
            validate_optional_json_field_kind(template, "attachments", JsonFieldKind::Array)
        }
        ("slack", "attachments" | "attachment") => {
            validate_json_field_kind(template, "attachments", JsonFieldKind::Array)
        }
        ("dingtalk", "actionCard") => {
            validate_optional_json_field_kind(template, "btns", JsonFieldKind::Array)
        }
        ("dingtalk", "feedCard") => {
            validate_json_field_kind(template, "links", JsonFieldKind::Array)
        }
        ("feishu", "post") => {
            if template.get("post").is_some() {
                validate_optional_json_field_kind(template, "post", JsonFieldKind::Object)
            } else {
                validate_json_field_kind(template, "content", JsonFieldKind::Array)
            }
        }
        ("feishu", "interactive") => {
            validate_json_field_kind(template, "card", JsonFieldKind::Object)
        }
        ("wechat_work", "news") => {
            validate_json_field_kind(template, "articles", JsonFieldKind::Array)
        }
        ("wechat_work", "template_card" | "templateCard") => validate_json_field_kind(
            template,
            template
                .get("templateCard")
                .map_or("template_card", |_| "templateCard"),
            JsonFieldKind::Object,
        ),
        ("pagerduty", _) => {
            validate_optional_json_field_kind(template, "links", JsonFieldKind::Array)?;
            validate_optional_json_field_kind(template, "images", JsonFieldKind::Array)?;
            validate_optional_json_field_kind(template, "customDetails", JsonFieldKind::Object)?;
            validate_optional_json_field_kind(template, "custom_details", JsonFieldKind::Object)
        }
        ("webhook", "json") => validate_json_field_kind(template, "body", JsonFieldKind::AnyJson),
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Copy)]
enum JsonFieldKind {
    AnyJson,
    Array,
    Object,
}

fn validate_optional_json_field_kind(
    template: &serde_json::Value,
    key: &str,
    kind: JsonFieldKind,
) -> Result<(), ApiError> {
    if template.get(key).is_some() {
        validate_json_field_kind(template, key, kind)?;
    }
    Ok(())
}

fn validate_json_field_kind(
    template: &serde_json::Value,
    key: &str,
    kind: JsonFieldKind,
) -> Result<(), ApiError> {
    let Some(value) = template.get(key) else {
        return Ok(());
    };
    let parsed;
    let candidate = if let Some(raw) = value.as_str() {
        parsed = serde_json::from_str::<serde_json::Value>(raw.trim()).map_err(|_| {
            ApiError::bad_request(format!(
                "template field {key} must be valid JSON {}",
                json_kind_label(kind)
            ))
        })?;
        &parsed
    } else {
        value
    };
    let valid = match kind {
        JsonFieldKind::AnyJson => true,
        JsonFieldKind::Array => candidate.is_array(),
        JsonFieldKind::Object => candidate.is_object(),
    };
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!(
            "template field {key} must be JSON {}",
            json_kind_label(kind)
        )))
    }
}

const fn json_kind_label(kind: JsonFieldKind) -> &'static str {
    match kind {
        JsonFieldKind::AnyJson => "value",
        JsonFieldKind::Array => "array",
        JsonFieldKind::Object => "object",
    }
}

pub(super) fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn json_field_present(value: &serde_json::Value, key: &str) -> bool {
    value.get(key).is_some_and(|field| match field {
        serde_json::Value::String(item) => !item.trim().is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Null => false,
        _ => true,
    })
}

fn json_field_present_any(value: &serde_json::Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| json_field_present(value, key))
}

pub(super) fn json_to_string<T: serde::Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_owned())
}
