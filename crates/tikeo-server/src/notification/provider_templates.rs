//! Provider-specific Notification Center payload template rendering.

use tikeo_storage::NotificationMessageSummary;

use crate::alert::AlertPayload;

use super::delivery::{
    alert_payload_from_message, notification_payload, notification_text, optional_string,
    pagerduty_severity,
};

/// Email alert payload from message.
pub(super) fn email_alert_payload_from_message(
    message: &NotificationMessageSummary,
    config: &serde_json::Map<String, serde_json::Value>,
) -> AlertPayload {
    let mut payload = alert_payload_from_message(message);
    let message_template = message_template_from_payload(message);
    if let Some(template) = message_template.as_ref().or_else(|| config.get("template")) {
        if let Some(subject) = template_string(template, &["subject", "title"]) {
            payload.rule_name = render_template(&subject, message);
        }
        if let Some(body) = template_string(template, &["body", "text", "content"]) {
            payload.message = render_template(&body, message);
        }
    }
    payload
}

/// Missing required template reason.
pub(super) fn missing_required_template_reason(
    provider: &str,
    message: &NotificationMessageSummary,
    config: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    let default_type = match provider {
        "dingtalk" | "feishu" | "wechat_work" | "wecom" => "text",
        _ => return None,
    };
    let message_template = message_template_from_payload(message);
    if message_template
        .as_ref()
        .or_else(|| config.get("template"))
        .is_some()
    {
        return None;
    }
    let message_type = message_type_from_template(config, message_template.as_ref(), default_type);
    let requires_template = match provider {
        "dingtalk" => matches!(message_type.as_str(), "link" | "actionCard" | "feedCard"),
        "feishu" => matches!(
            message_type.as_str(),
            "image" | "share_chat" | "shareChat" | "interactive"
        ),
        "wechat_work" | "wecom" => matches!(
            message_type.as_str(),
            "image" | "news" | "file" | "voice" | "template_card" | "templateCard"
        ),
        _ => false,
    };
    requires_template.then(|| {
        format!(
            "{provider} {message_type} delivery requires a channel inline template or enabled policy notification template"
        )
    })
}

/// Webhook payload.
pub(super) fn webhook_payload(
    message: &NotificationMessageSummary,
    config: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let message_template = message_template_from_payload(message);
    let body = message_template
        .as_ref()
        .or_else(|| config.get("template"))
        .and_then(|template| {
            template
                .get("body")
                .cloned()
                .or_else(|| Some(template.clone()))
        })
        .unwrap_or_else(|| notification_payload(message));
    render_template_jsonish(body, message)
}

/// Slack payload.
pub(super) fn slack_payload(
    message: &NotificationMessageSummary,
    config: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let message_template = message_template_from_payload(message);
    let message_type = message_type_from_template(config, message_template.as_ref(), "text");
    let Some(template) = message_template.as_ref().or_else(|| config.get("template")) else {
        return with_optional_slack_thread(
            serde_json::json!({ "text": notification_text(message) }),
            config,
        );
    };
    let mut body = match message_type.as_str() {
        "blockKit" | "block_kit" | "blocks" => {
            let mut body = if template.is_object() {
                template.clone()
            } else {
                serde_json::json!({ "text": render_template(template.as_str().unwrap_or_default(), message) })
            };
            render_template_value(&mut body, message);
            parse_object_json_fields(&mut body, &["blocks", "attachments"]);
            ensure_object_field(&mut body, "text", || notification_text(message));
            body
        }
        "attachments" | "attachment" => {
            let mut body = if template.is_object() {
                template.clone()
            } else {
                serde_json::json!({ "text": render_template(template.as_str().unwrap_or_default(), message) })
            };
            render_template_value(&mut body, message);
            parse_object_json_fields(&mut body, &["attachments"]);
            ensure_object_field(&mut body, "text", || notification_text(message));
            body
        }
        _ => serde_json::json!({ "text": render_template(&template_text(template), message) }),
    };
    if let Some(thread_ts) =
        template_string_from_config(config, Some(template), &["threadTs", "thread_ts"])
    {
        insert_string_field(&mut body, "thread_ts", thread_ts);
    }
    body
}

/// Dingtalk payload.
pub(super) fn dingtalk_payload(
    message: &NotificationMessageSummary,
    config: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let message_template = message_template_from_payload(message);
    let message_type = message_type_from_template(config, message_template.as_ref(), "text");
    let template = message_template.as_ref().or_else(|| config.get("template"));
    let default_text = notification_text(message);
    let mut body = match message_type.as_str() {
        "markdown" => serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "title": template.and_then(|t| template_string(t, &["title"])).unwrap_or_else(|| message.subject.clone()),
                "text": template.and_then(|t| template_string(t, &["text", "body", "content"])).unwrap_or(default_text),
            },
            "at": dingtalk_at(config),
        }),
        "link" => serde_json::json!({
            "msgtype": "link",
            "link": dingtalk_object_payload(template)
        }),
        "actionCard" => serde_json::json!({
            "msgtype": "actionCard",
            "actionCard": dingtalk_object_payload(template)
        }),
        "feedCard" => serde_json::json!({
            "msgtype": "feedCard",
            "feedCard": dingtalk_feed_card_payload(template)
        }),
        _ => serde_json::json!({
            "msgtype": "text",
            "text": { "content": template.map_or(default_text, |t| render_template(&template_text(t), message)) },
            "at": dingtalk_at(config),
        }),
    };
    render_template_value(&mut body, message);
    normalize_nested_json_strings(&mut body);
    body
}

/// Feishu payload.
pub(super) fn feishu_payload(
    message: &NotificationMessageSummary,
    config: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let message_template = message_template_from_payload(message);
    let message_type = message_type_from_template(config, message_template.as_ref(), "text");
    let template = message_template.as_ref().or_else(|| config.get("template"));
    let mut body = match message_type.as_str() {
        "post" => serde_json::json!({
            "msg_type": "post",
            "content": feishu_post_content(template, serde_json::json!({
                "post": {"zh_cn": {"title": message.subject, "content": [[{"tag":"text", "text": message.body}]]}}
            }))
        }),
        "image" => serde_json::json!({
            "msg_type": "image",
            "content": {"image_key": template_string_or_default(template, &["imageKey", "image_key"], "")}
        }),
        "share_chat" | "shareChat" => serde_json::json!({
            "msg_type": "share_chat",
            "content": {"share_chat_id": template_string_or_default(template, &["shareChatId", "share_chat_id"], "")}
        }),
        "interactive" => serde_json::json!({
            "msg_type": "interactive",
            "card": feishu_card_payload(template, serde_json::json!({
                "header": {"title": {"tag":"plain_text", "content": message.subject}},
                "elements": [{"tag":"div", "text": {"tag":"plain_text", "content": message.body}}]
            }))
        }),
        _ => serde_json::json!({
            "msg_type": "text",
            "content": { "text": template.map_or_else(|| notification_text(message), |t| render_template(&template_text(t), message)) },
        }),
    };
    render_template_value(&mut body, message);
    normalize_nested_json_strings(&mut body);
    body
}

/// Wechat work payload.
pub(super) fn wechat_work_payload(
    message: &NotificationMessageSummary,
    config: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let message_template = message_template_from_payload(message);
    let message_type = message_type_from_template(config, message_template.as_ref(), "text");
    let template = message_template.as_ref().or_else(|| config.get("template"));
    let mut body = match message_type.as_str() {
        "markdown_v2" | "markdownV2" => serde_json::json!({
            "msgtype": "markdown_v2",
            "markdown_v2": { "content": template.map_or_else(|| notification_text(message), |t| render_template(&template_text(t), message)) }
        }),
        "markdown" => serde_json::json!({
            "msgtype": "markdown",
            "markdown": { "content": template.map_or_else(|| notification_text(message), |t| render_template(&template_text(t), message)) }
        }),
        "image" => serde_json::json!({
            "msgtype": "image",
            "image": template_object_or_default(template, serde_json::json!({"base64": "", "md5": ""}))
        }),
        "news" => serde_json::json!({
            "msgtype": "news",
            "news": wechat_news_payload(template)
        }),
        "file" => serde_json::json!({
            "msgtype": "file",
            "file": template_object_or_default(template, serde_json::json!({"media_id": ""}))
        }),
        "voice" => serde_json::json!({
            "msgtype": "voice",
            "voice": template_object_or_default(template, serde_json::json!({"media_id": ""}))
        }),
        "template_card" | "templateCard" => serde_json::json!({
            "msgtype": "template_card",
            "template_card": wechat_template_card_payload(template)
        }),
        _ => serde_json::json!({
            "msgtype": "text",
            "text": { "content": template.map_or_else(|| notification_text(message), |t| render_template(&template_text(t), message)) },
            "mentioned_list": string_array(config, &["mentionedList", "mentioned_list"]),
            "mentioned_mobile_list": string_array(config, &["mentionedMobileList", "mentioned_mobile_list"]),
        }),
    };
    render_template_value(&mut body, message);
    normalize_nested_json_strings(&mut body);
    body
}

/// Pagerduty payload.
pub(super) fn pagerduty_payload(
    message: &NotificationMessageSummary,
    routing_key: &str,
    config: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let message_template = message_template_from_payload(message);
    let action =
        match message_type_from_template(config, message_template.as_ref(), "trigger").as_str() {
            "acknowledge" | "ack" => "acknowledge",
            "resolve" => "resolve",
            _ => "trigger",
        };
    let template = message_template.as_ref().or_else(|| config.get("template"));
    let mut custom_details = template
        .and_then(|template| {
            template
                .get("customDetails")
                .or_else(|| template.get("custom_details"))
        })
        .cloned()
        .or_else(|| {
            config
                .get("customDetails")
                .or_else(|| config.get("custom_details"))
                .or_else(|| config.get("customDetailsJson"))
                .or_else(|| config.get("custom_details_json"))
                .cloned()
        })
        .unwrap_or_else(|| notification_payload(message));
    render_template_value(&mut custom_details, message);
    custom_details = parse_json_string_value(custom_details);
    let mut body = serde_json::json!({
        "routing_key": routing_key,
        "event_action": action,
        "dedup_key": template_string_from_config(config, template, &["dedupKey", "dedup_key"]).unwrap_or_else(|| message.dedupe_key.clone()),
        "payload": {
            "summary": template_string_from_config(config, template, &["summary", "subject", "title"]).unwrap_or_else(|| message.subject.clone()),
            "source": template_string_from_config(config, template, &["source"]).unwrap_or_else(|| "tikeo".to_owned()),
            "severity": pagerduty_severity_from_config(config, template, &message.severity),
            "component": template_string_from_config(config, template, &["component"]).unwrap_or_else(|| message.resource_type.clone()),
            "custom_details": custom_details,
        },
    });
    if let Some(payload) = body.get_mut("payload") {
        for key in ["group", "class", "timestamp"] {
            if let Some(value) = template_string_from_config(config, template, &[key])
                && let Some(payload) = payload.as_object_mut()
            {
                payload.insert(key.to_owned(), serde_json::Value::String(value));
            }
        }
    }
    for key in ["client"] {
        if let Some(value) = template_string_from_config(config, template, &[key]) {
            insert_string_field(&mut body, key, value);
        }
    }
    if let Some(value) = template_string_from_config(config, template, &["clientUrl", "client_url"])
    {
        insert_string_field(&mut body, "client_url", value);
    }
    if let Some(value) = template
        .and_then(|template| template.get("links"))
        .or_else(|| config.get("links"))
        .cloned()
    {
        let mut links = value;
        render_template_value(&mut links, message);
        body.as_object_mut()
            .map(|object| object.insert("links".to_owned(), parse_json_string_value(links)));
    }
    if let Some(value) = template
        .and_then(|template| template.get("images"))
        .or_else(|| config.get("images"))
        .cloned()
    {
        let mut images = value;
        render_template_value(&mut images, message);
        body.as_object_mut()
            .map(|object| object.insert("images".to_owned(), parse_json_string_value(images)));
    }
    render_template_value(&mut body, message);
    body
}

fn pagerduty_severity_from_config(
    config: &serde_json::Map<String, serde_json::Value>,
    template: Option<&serde_json::Value>,
    message_severity: &str,
) -> &'static str {
    match template_string_from_config(config, template, &["severity"])
        .unwrap_or_else(|| message_severity.to_owned())
        .as_str()
    {
        "critical" => "critical",
        "error" => "error",
        "warning" => "warning",
        _ => pagerduty_severity(message_severity),
    }
}

fn message_template_from_payload(
    message: &NotificationMessageSummary,
) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(&message.payload_json)
        .ok()
        .and_then(|payload| payload.get("template").cloned())
}

fn message_type_from_template(
    config: &serde_json::Map<String, serde_json::Value>,
    template: Option<&serde_json::Value>,
    default: &str,
) -> String {
    template
        .and_then(|template| template_string(template, &["messageType", "message_type"]))
        .or_else(|| optional_string(config, &["messageType", "message_type"]))
        .unwrap_or_else(|| default.to_owned())
}

fn template_text(template: &serde_json::Value) -> String {
    template
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| template_string(template, &["text", "body", "content", "message"]))
        .unwrap_or_else(|| "{{subject}}: {{body}}".to_owned())
}

fn template_string(template: &serde_json::Value, keys: &[&str]) -> Option<String> {
    if let Some(value) = template.as_str() {
        return Some(value.to_owned());
    }
    keys.iter()
        .find_map(|key| template.get(*key).and_then(serde_json::Value::as_str))
        .map(ToOwned::to_owned)
}

fn template_string_or_default(
    template: Option<&serde_json::Value>,
    keys: &[&str],
    default: &str,
) -> String {
    template
        .and_then(|template| template_string(template, keys))
        .unwrap_or_else(|| default.to_owned())
}

fn template_object_or_default(
    template: Option<&serde_json::Value>,
    default: serde_json::Value,
) -> serde_json::Value {
    template
        .filter(|template| template.is_object())
        .cloned()
        .unwrap_or(default)
}

fn dingtalk_object_payload(template: Option<&serde_json::Value>) -> serde_json::Value {
    let mut payload = template
        .filter(|template| template.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(object) = payload.as_object_mut() {
        if let Some(value) = object.remove("btnOrientation") {
            object.insert("btnOrientation".to_owned(), value);
        }
        if let Some(value) = object.get_mut("btns") {
            *value = parse_json_string_value(value.clone());
        }
    }
    payload
}

fn dingtalk_feed_card_payload(template: Option<&serde_json::Value>) -> serde_json::Value {
    let Some(template) = template else {
        return serde_json::json!({});
    };
    if let Some(links) = template.get("links") {
        return serde_json::json!({ "links": links.clone() });
    }
    if template.is_object() {
        template.clone()
    } else {
        serde_json::json!({})
    }
}

fn feishu_post_content(
    template: Option<&serde_json::Value>,
    default: serde_json::Value,
) -> serde_json::Value {
    let Some(template) = template else {
        return default;
    };
    if template.get("post").is_some() {
        return template.clone();
    }
    serde_json::json!({
        "post": {
            "zh_cn": {
                "title": template_string(template, &["title"]).unwrap_or_else(|| "Tikeo notification".to_owned()),
                "content": template.get("content").cloned().unwrap_or_else(|| serde_json::json!([[{"tag":"text","text":"{{body}}"}]]))
            }
        }
    })
}

fn feishu_card_payload(
    template: Option<&serde_json::Value>,
    default: serde_json::Value,
) -> serde_json::Value {
    let Some(template) = template else {
        return default;
    };
    template
        .get("card")
        .cloned()
        .unwrap_or_else(|| template.clone())
}

fn wechat_news_payload(template: Option<&serde_json::Value>) -> serde_json::Value {
    let Some(template) = template else {
        return serde_json::json!({});
    };
    if let Some(articles) = template.get("articles") {
        return serde_json::json!({ "articles": articles.clone() });
    }
    if template.is_object() {
        template.clone()
    } else {
        serde_json::json!({})
    }
}

fn wechat_template_card_payload(template: Option<&serde_json::Value>) -> serde_json::Value {
    let Some(template) = template else {
        return serde_json::json!({});
    };
    template
        .get("templateCard")
        .or_else(|| template.get("template_card"))
        .or_else(|| template.get("card"))
        .cloned()
        .unwrap_or_else(|| template.clone())
}

fn ensure_object_field(value: &mut serde_json::Value, key: &str, default: impl FnOnce() -> String) {
    if let Some(object) = value.as_object_mut()
        && !object.contains_key(key)
    {
        object.insert(key.to_owned(), serde_json::Value::String(default()));
    }
}

fn insert_string_field(value: &mut serde_json::Value, key: &str, item: String) {
    if let Some(object) = value.as_object_mut()
        && !item.trim().is_empty()
    {
        object.insert(key.to_owned(), serde_json::Value::String(item));
    }
}

fn with_optional_slack_thread(
    mut body: serde_json::Value,
    config: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    if let Some(thread_ts) = optional_string(config, &["threadTs", "thread_ts"]) {
        insert_string_field(&mut body, "thread_ts", thread_ts);
    }
    body
}

/// Render template value.
pub(super) fn render_template_value(
    value: &mut serde_json::Value,
    message: &NotificationMessageSummary,
) {
    match value {
        serde_json::Value::String(item) => *item = render_template(item, message),
        serde_json::Value::Array(items) => {
            for item in items {
                render_template_value(item, message);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                render_template_value(item, message);
            }
        }
        _ => {}
    }
}

/// Validate template tokens.
///
/// # Errors
///
/// Returns an error when the underlying operation fails.
pub(super) fn validate_template_tokens(value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::String(item) => validate_template_string(item),
        serde_json::Value::Array(items) => {
            for item in items {
                validate_template_tokens(item)?;
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for item in map.values() {
                validate_template_tokens(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn render_template_jsonish(
    value: serde_json::Value,
    message: &NotificationMessageSummary,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(template) => parse_json_string_value(serde_json::Value::String(
            render_template(&template, message),
        )),
        mut other => {
            render_template_value(&mut other, message);
            normalize_nested_json_strings(&mut other);
            other
        }
    }
}

fn normalize_nested_json_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(_) => {
            let parsed = parse_json_string_value(value.clone());
            *value = parsed;
        }
        serde_json::Value::Array(items) => {
            for item in items {
                normalize_nested_json_strings(item);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                normalize_nested_json_strings(item);
            }
        }
        _ => {}
    }
}

fn parse_object_json_fields(value: &mut serde_json::Value, fields: &[&str]) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    for field in fields {
        if let Some(item) = object.get_mut(*field) {
            *item = parse_json_string_value(item.clone());
        }
    }
}

fn parse_json_string_value(value: serde_json::Value) -> serde_json::Value {
    let serde_json::Value::String(raw) = value else {
        return value;
    };
    let trimmed = raw.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return serde_json::Value::String(raw);
    }
    serde_json::from_str(trimmed).unwrap_or(serde_json::Value::String(raw))
}

fn template_string_from_config(
    config: &serde_json::Map<String, serde_json::Value>,
    template: Option<&serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    template
        .and_then(|template| template_string(template, keys))
        .or_else(|| optional_string(config, keys))
}

fn dingtalk_at(config: &serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "atMobiles": string_array(config, &["atMobiles", "at_mobiles"]),
        "atUserIds": string_array(config, &["atUserIds", "at_user_ids"]),
        "isAtAll": config.get("isAtAll").or_else(|| config.get("is_at_all")).and_then(serde_json::Value::as_bool).unwrap_or(false),
    })
}

fn string_array(config: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|key| config.get(*key))
        .map(|value| match value {
            serde_json::Value::String(item) if !item.trim().is_empty() => vec![item.clone()],
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|item| !item.trim().is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default()
}

fn render_template(template: &str, message: &NotificationMessageSummary) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(start) = remaining.find("{{") {
        rendered.push_str(&remaining[..start]);
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find("}}") else {
            rendered.push_str(&remaining[start..]);
            return rendered;
        };
        let token = after_start[..end].trim();
        if let Some(value) = template_token_value(token, message) {
            rendered.push_str(&value);
        } else {
            rendered.push_str("{{");
            rendered.push_str(&after_start[..end]);
            rendered.push_str("}}");
        }
        remaining = &after_start[end + 2..];
    }
    rendered.push_str(remaining);
    rendered
}

fn validate_template_string(template: &str) -> Result<(), String> {
    let mut remaining = template;
    while let Some(position) = first_template_delimiter(remaining) {
        if remaining[position..].starts_with("}}") {
            return Err("template contains unopened token delimiter }}".to_owned());
        }
        let after_start = &remaining[position + 2..];
        let Some(end) = after_start.find("}}") else {
            return Err("template contains unclosed token delimiter {{".to_owned());
        };
        let token = after_start[..end].trim();
        if template_token_value_for_validation(token).is_none() {
            return Err(format!("template token is not allowed: {token}"));
        }
        remaining = &after_start[end + 2..];
    }
    Ok(())
}

fn first_template_delimiter(value: &str) -> Option<usize> {
    match (value.find("{{"), value.find("}}")) {
        (Some(open), Some(close)) => Some(open.min(close)),
        (Some(open), None) => Some(open),
        (None, Some(close)) => Some(close),
        (None, None) => None,
    }
}

fn template_token_value_for_validation(token: &str) -> Option<()> {
    matches!(
        token,
        "subject"
            | "body"
            | "eventType"
            | "resourceType"
            | "resourceId"
            | "severity"
            | "messageId"
            | "policyId"
            | "dedupeKey"
            | "triggeredAt"
            | "createdAt"
            | "jobId"
            | "jobName"
            | "namespace"
            | "app"
            | "instanceId"
            | "status"
            | "triggerType"
            | "executionMode"
            | "startedAt"
            | "finishedAt"
            | "workerId"
            | "operatorName"
            | "operatorType"
            | "reason"
            | "logsUrl"
            | "consoleUrl"
    )
    .then_some(())
}

fn template_token_value(token: &str, message: &NotificationMessageSummary) -> Option<String> {
    match token {
        "subject" => Some(message.subject.clone()),
        "body" => Some(message.body.clone()),
        "eventType" => Some(message.event_type.clone()),
        "resourceType" => Some(message.resource_type.clone()),
        "resourceId" => Some(message.resource_id.clone()),
        "severity" => Some(message.severity.clone()),
        "messageId" => Some(message.id.clone()),
        "policyId" => Some(message.policy_id.clone()),
        "dedupeKey" => Some(message.dedupe_key.clone()),
        "triggeredAt" | "createdAt" => Some(message.created_at.clone()),
        other => payload_string_field(message, other),
    }
}

fn payload_string_field(message: &NotificationMessageSummary, key: &str) -> Option<String> {
    let payload = serde_json::from_str::<serde_json::Value>(&message.payload_json).ok()?;
    payload
        .get(key)
        .and_then(value_to_template_string)
        .or_else(|| match key {
            "operatorName" => payload
                .pointer("/operator/name")
                .and_then(value_to_template_string),
            "operatorType" => payload
                .pointer("/operator/type")
                .and_then(value_to_template_string),
            "logsUrl" => payload
                .pointer("/logs/url")
                .and_then(value_to_template_string),
            "consoleUrl" => payload
                .pointer("/console/url")
                .and_then(value_to_template_string)
                .or_else(|| {
                    payload
                        .pointer("/logs/url")
                        .and_then(value_to_template_string)
                }),
            "workerId" => payload
                .pointer("/instance/workerId")
                .and_then(value_to_template_string),
            _ => None,
        })
}

fn value_to_template_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(item) => Some(item.clone()),
        serde_json::Value::Number(item) => Some(item.to_string()),
        serde_json::Value::Bool(item) => Some(item.to_string()),
        _ => None,
    }
}
