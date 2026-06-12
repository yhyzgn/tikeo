    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn notification_center_api_redacts_channels_and_validates_policies() {
        let app = router().await;
        let _billing_scope = post_json(
            app.clone(),
            "/api/v1/apps",
            r#"{"namespace":"default","name":"billing"}"#,
        )
        .await;

        let types = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/notification-channel-types").await)
            .await
            .unwrap_or_else(|error| panic!("notification channel types should respond: {error}"));
        assert!(types.status().is_success());
        let body = axum::body::to_bytes(types.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let types_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(types_json["code"], 0);
        assert!(types_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("types should be an array"))
            .iter()
            .any(|item| item["type"] == "feishu" && item["category"] == "office_bot"));
        assert!(types_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("types should be an array"))
            .iter()
            .filter(|item| item["pluginProvided"] == false)
            .all(|item| item["supportsTestSend"] == true));
        let email_type = types_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("types should be an array"))
            .iter()
            .find(|item| item["type"] == "email")
            .unwrap_or_else(|| panic!("email type should be present"));
        assert!(email_type["secretConfigKeys"]
            .as_array()
            .unwrap_or_else(|| panic!("email secret keys should be an array"))
            .iter()
            .any(|key| key == "password"));

        let channel_types = types_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("types should be an array"));
        assert_provider_template(channel_types, "slack", &["text", "blockKit", "attachments"]);
        assert_provider_template(
            channel_types,
            "dingtalk",
            &["text", "markdown", "link", "actionCard", "feedCard"],
        );
        assert_provider_template(
            channel_types,
            "feishu",
            &["text", "post", "image", "share_chat", "interactive"],
        );
        assert_provider_template(
            channel_types,
            "wechat_work",
            &["text", "markdown", "markdown_v2", "image", "news", "file", "voice", "template_card"],
        );
        assert_provider_template(
            channel_types,
            "pagerduty",
            &["trigger", "acknowledge", "resolve"],
        );
        assert_provider_template(channel_types, "email", &["plain", "html"]);
        assert_provider_template(channel_types, "webhook", &["json"]);
        for provider in ["slack", "dingtalk", "feishu", "wechat_work", "pagerduty", "email", "webhook"] {
            assert_provider_template_examples(channel_types, provider);
        }
        assert_provider_template_has_field(channel_types, "dingtalk", "atUserIds");
        assert_provider_template_has_field(channel_types, "wechat_work", "mentionedList");
        assert_provider_template_has_field(channel_types, "wechat_work", "mentionedMobileList");
        assert_provider_template_has_field(channel_types, "pagerduty", "customDetails");
        assert_provider_template_has_field(channel_types, "pagerduty", "clientUrl");
        assert_provider_template_has_field(channel_types, "slack", "threadTs");
        let slack_type = channel_types
            .iter()
            .find(|item| item["type"] == "slack")
            .unwrap_or_else(|| panic!("slack type should be present"));
        assert_eq!(slack_type["requiredConfigKeys"].as_array().map(Vec::len), Some(0));
        assert!(slack_type["requiredTargetKeys"]
            .as_array()
            .unwrap_or_else(|| panic!("slack requiredTargetKeys should be an array"))
            .iter()
            .any(|key| key == "url"));

        let missing_scope = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"app","name":"Missing scope","provider":"webhook","enabled":true,"secretRefs":{"url":"env:TIKEO_NOTIFICATION_WEBHOOK_URL"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("missing scope channel should respond: {error}"));
        assert_eq!(missing_scope.status(), axum::http::StatusCode::BAD_REQUEST);

        let invalid_message_type = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"global","name":"Bad message type","provider":"wechat_work","enabled":true,"secretRefs":{"url":"env:TIKEO_WECOM_WEBHOOK_URL"},"config":{"messageType":"unsupported"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("invalid message type should respond: {error}"));
        assert_eq!(invalid_message_type.status(), axum::http::StatusCode::BAD_REQUEST);

        let missing_template_field = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"global","name":"Bad template","provider":"wechat_work","enabled":true,"secretRefs":{"url":"env:TIKEO_WECOM_WEBHOOK_URL"},"config":{"messageType":"voice","template":{}}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("missing template field should respond: {error}"));
        assert_eq!(missing_template_field.status(), axum::http::StatusCode::BAD_REQUEST);

        let raw_secret_config = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"global","name":"Raw routing key","provider":"pagerduty","enabled":true,"config":{"routingKey":"pd-secret-routing-key"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("raw secret config should respond: {error}"));
        assert_eq!(raw_secret_config.status(), axum::http::StatusCode::BAD_REQUEST);

        let raw_authorization_header = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"global","name":"Raw auth header","provider":"webhook","enabled":true,"config":{"url":"https://hooks.example.com/services/token","headers":{"Authorization":"Bearer plain-secret"},"messageType":"json","template":{"body":{"text":"{{subject}}"}}}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("raw authorization header should respond: {error}"));
        assert_eq!(
            raw_authorization_header.status(),
            axum::http::StatusCode::BAD_REQUEST
        );

        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Billing Feishu","provider":"feishu","enabled":true,"config":{"url":"https://open.feishu.cn/open-apis/bot/v2/hook/super-secret-token","mentionAll":true,"messageType":"text","template":{"text":"{{subject}}"}},"secretRefs":{"signingKey":"env:FEISHU_BOT_SECRET"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification channel create should respond: {error}"));
        assert!(created.status().is_success());
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let created_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(created_json["code"], 0);
        assert_eq!(created_json["data"]["provider"], "feishu");
        assert_eq!(created_json["data"]["targetRedacted"], "https://open.feishu.cn/...");
        assert_eq!(created_json["data"]["secretConfigured"], true);
        assert!(!created_json.to_string().contains("super-secret-token"));
        assert!(!created_json.to_string().contains("FEISHU_BOT_SECRET"));
        assert!(created_json["data"].get("secretRefsJson").is_none());
        let channel_id = created_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("channel id should be present"))
            .to_owned();


        let edit_patch = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/notification-channels/{channel_id}"),
                    r#"{"name":"Billing Feishu renamed"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("metadata-only channel patch should respond: {error}"));
        assert!(edit_patch.status().is_success());
        let body = axum::body::to_bytes(edit_patch.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let edit_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(edit_json["data"]["targetRedacted"], "https://open.feishu.cn/...");
        assert_eq!(edit_json["data"]["secretConfigured"], true);
        assert!(!edit_json.to_string().contains("FEISHU_BOT_SECRET"));

        let secret_ref_only = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Secret ref webhook","provider":"webhook","enabled":true,"config":{"messageType":"json"},"secretRefs":{"url":"env:TIKEO_NOTIFICATION_WEBHOOK_URL","authorization":"env:TIKEO_NOTIFICATION_AUTH"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("secret-ref-only webhook channel should respond: {error}"));
        assert!(secret_ref_only.status().is_success());
        let body = axum::body::to_bytes(secret_ref_only.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let secret_ref_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(secret_ref_json["code"], 0);
        assert_eq!(secret_ref_json["data"]["targetConfigured"], true);
        assert_eq!(secret_ref_json["data"]["targetRedacted"], "webhook:secret-ref");
        assert_eq!(secret_ref_json["data"]["secretConfigured"], true);
        assert!(secret_ref_json["data"].get("secretRefsJson").is_none());
        assert!(!secret_ref_json.to_string().contains("TIKEO_NOTIFICATION_WEBHOOK_URL"));
        assert!(!secret_ref_json.to_string().contains("TIKEO_NOTIFICATION_AUTH"));

        let pagerduty_secret_ref_only = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Secret ref PagerDuty","provider":"pagerduty","enabled":true,"config":{"messageType":"trigger","template":{"summary":"{{subject}}"}},"secretRefs":{"routingKey":"env:TIKEO_PAGERDUTY_ROUTING_KEY"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("secret-ref-only pagerduty channel should respond: {error}"));
        assert!(pagerduty_secret_ref_only.status().is_success());
        let body = axum::body::to_bytes(pagerduty_secret_ref_only.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let pagerduty_secret_ref_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(pagerduty_secret_ref_json["code"], 0);
        assert_eq!(
            pagerduty_secret_ref_json["data"]["targetRedacted"],
            "pagerduty:secret-ref"
        );
        assert!(pagerduty_secret_ref_json["data"]
            .get("secretRefsJson")
            .is_none());
        assert!(!pagerduty_secret_ref_json
            .to_string()
            .contains("TIKEO_PAGERDUTY_ROUTING_KEY"));

        let email_secret_ref_only = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Secret ref email","provider":"email","enabled":true,"config":{"to":["ops@example.com"],"username":"tikeo","messageType":"plain","template":{"subject":"{{subject}}","body":"{{body}}"}},"secretRefs":{"smtpUrl":"env:TIKEO_SMTP_URL","password":"env:TIKEO_SMTP_PASSWORD"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("secret-ref-only email channel should respond: {error}"));
        assert!(email_secret_ref_only.status().is_success());
        let body = axum::body::to_bytes(email_secret_ref_only.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let email_secret_ref_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(email_secret_ref_json["code"], 0);
        assert_eq!(email_secret_ref_json["data"]["targetRedacted"], "ops@example.com");
        assert!(email_secret_ref_json["data"].get("secretRefsJson").is_none());
        assert!(!email_secret_ref_json.to_string().contains("TIKEO_SMTP_URL"));
        assert!(!email_secret_ref_json
            .to_string()
            .contains("TIKEO_SMTP_PASSWORD"));

        let missing_channel_policy = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-policies",
                    r#"{"ownerType":"job","ownerId":"job-billing-nightly","name":"Missing channel policy","eventFamily":"job_instance","eventFilter":{"statuses":["failed"]},"channelRefs":[{"channelId":"notification-channel-missing"}],"severity":"critical","enabled":true,"dedupeSeconds":300}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("missing channel policy should respond: {error}"));
        assert_eq!(
            missing_channel_policy.status(),
            axum::http::StatusCode::BAD_REQUEST
        );
        let body = axum::body::to_bytes(missing_channel_policy.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let missing_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(missing_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("channel does not exist")));

        let disabled = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Disabled webhook","provider":"webhook","enabled":false,"config":{"url":"https://hooks.example.com/services/disabled-token","messageType":"json","template":{"body":"{}"}}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("disabled channel create should respond: {error}"));
        assert!(disabled.status().is_success());
        let body = axum::body::to_bytes(disabled.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let disabled_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let disabled_channel_id = disabled_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("disabled channel id should be present"));
        let disabled_policy_body = format!(
            r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"Disabled channel policy","eventFamily":"job_instance","eventFilter":{{"statuses":["failed"]}},"channelRefs":[{{"channelId":"{disabled_channel_id}"}}],"severity":"critical","enabled":true,"dedupeSeconds":300}}"#
        );
        let disabled_channel_policy = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-policies",
                    &disabled_policy_body,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("disabled channel policy should respond: {error}"));
        assert_eq!(
            disabled_channel_policy.status(),
            axum::http::StatusCode::BAD_REQUEST
        );
        let body = axum::body::to_bytes(disabled_channel_policy.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let disabled_policy_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(disabled_policy_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("channel is disabled")));

        let listed = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/notification-channels").await)
            .await
            .unwrap_or_else(|error| panic!("notification channels should list: {error}"));
        assert!(listed.status().is_success());
        let body = axum::body::to_bytes(listed.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let listed_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let listed_channels = listed_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("notification channels list should be an array"));
        assert!(
            listed_channels.len() >= 5,
            "list should include user-created channels and seeded normal channel rows: {listed_json}"
        );
        assert!(listed_channels
            .iter()
            .any(|item| item["id"] == "notification-channel-example-slack-text"));
        assert!(listed_channels
            .iter()
            .any(|item| item["id"] == channel_id));
        assert!(!listed_json.to_string().contains("super-secret-token"));
        assert!(!listed_json.to_string().contains("FEISHU_BOT_SECRET"));

        let slack_template = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{"templateKey":"ops.slack.for-feishu-mismatch","name":"Slack mismatch","provider":"slack","messageType":"text","enabled":true,"body":{"text":"{{subject}}"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("slack template should respond: {error}"));
        assert!(slack_template.status().is_success());
        let mismatch_body = format!(
            r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"Mismatched template policy","eventFamily":"job_instance","eventFilter":{{"statuses":["failed"]}},"channelRefs":[{{"channelId":"{channel_id}"}}],"templateRef":"ops.slack.for-feishu-mismatch","severity":"critical","enabled":true,"dedupeSeconds":300}}"#
        );
        let mismatched_template_policy = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-policies",
                    &mismatch_body,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("mismatched template policy should respond: {error}"));
        assert_eq!(
            mismatched_template_policy.status(),
            axum::http::StatusCode::BAD_REQUEST
        );
        let body = axum::body::to_bytes(mismatched_template_policy.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let mismatch_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(mismatch_json["message"].as_str().is_some_and(|message| {
            message.contains("template provider slack does not match channel provider")
        }));

        let disabled_template = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{"templateKey":"ops.feishu.disabled","name":"Disabled Feishu","provider":"feishu","messageType":"text","enabled":false,"body":{"text":"{{subject}}"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("disabled template should respond: {error}"));
        assert!(disabled_template.status().is_success());
        for (name, template_ref, expected) in [
            (
                "Disabled template policy",
                "ops.feishu.disabled",
                "template is disabled",
            ),
            (
                "Missing template policy",
                "ops.feishu.missing",
                "template does not exist",
            ),
        ] {
            let body = format!(
                r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"{name}","eventFamily":"job_instance","eventFilter":{{"statuses":["failed"]}},"channelRefs":[{{"channelId":"{channel_id}"}}],"templateRef":"{template_ref}","severity":"critical","enabled":true,"dedupeSeconds":300}}"#
            );
            let response = app
                .clone()
                .oneshot(
                    admin_json_request_builder(
                        app.clone(),
                        "POST",
                        "/api/v1/notification-policies",
                        &body,
                    )
                    .await,
                )
                .await
                .unwrap_or_else(|error| panic!("{name} should respond: {error}"));
            assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap_or_else(|error| panic!("body should collect: {error}"));
            let json: Value = serde_json::from_slice(&body)
                .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
            assert!(
                json["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(expected)),
                "{name} should mention {expected}: {json}"
            );
        }

        let feishu_template = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{"templateKey":"ops.feishu.enabled","name":"Enabled Feishu","provider":"feishu","messageType":"text","enabled":true,"body":{"text":"{{subject}}"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("feishu template should respond: {error}"));
        assert!(feishu_template.status().is_success());
        let webhook_channel = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"global","name":"Global webhook","provider":"webhook","enabled":true,"config":{"messageType":"json","template":{"body":{"text":"{{subject}}"}}},"secretRefs":{"url":"env:TIKEO_NOTIFICATION_WEBHOOK_URL"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("webhook channel should respond: {error}"));
        assert!(webhook_channel.status().is_success());
        let body = axum::body::to_bytes(webhook_channel.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let webhook_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let webhook_channel_id = webhook_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("webhook channel id should be present"));
        let multi_provider_body = format!(
            r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"Mixed provider template policy","eventFamily":"job_instance","eventFilter":{{"statuses":["failed"]}},"channelRefs":[{{"channelId":"{channel_id}"}},{{"channelId":"{webhook_channel_id}"}}],"templateRef":"ops.feishu.enabled","severity":"critical","enabled":true,"dedupeSeconds":300}}"#
        );
        let mixed_template_policy = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-policies",
                    &multi_provider_body,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("mixed provider policy should respond: {error}"));
        assert_eq!(
            mixed_template_policy.status(),
            axum::http::StatusCode::BAD_REQUEST
        );
        let body = axum::body::to_bytes(mixed_template_policy.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let mixed_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(mixed_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("channel provider(s): webhook")));

        let policy_body = format!(
            r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"Billing failure notifications","eventFamily":"job_instance","eventFilter":{{"statuses":["failed","retry_exhausted"]}},"channelRefs":[{{"channelId":"{channel_id}"}}],"severity":"critical","enabled":true,"dedupeSeconds":300}}"#
        );
        let created_policy = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-policies",
                    &policy_body,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification policy create should respond: {error}"));
        assert!(created_policy.status().is_success());
        let body = axum::body::to_bytes(created_policy.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let policy_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(policy_json["code"], 0);
        assert_eq!(policy_json["data"]["eventFamily"], "job_instance");
        let policy_id = policy_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("policy id should be present"));

        let invalid_update = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/notification-policies/{policy_id}"),
                    r#"{"channelRefs":[{"channelId":"notification-channel-missing"}]}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("invalid policy update should respond: {error}"));
        assert_eq!(
            invalid_update.status(),
            axum::http::StatusCode::BAD_REQUEST
        );

        let validation = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    format!("/api/v1/notification-policies/{policy_id}:validate"),
                    "{}",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("policy validation should respond: {error}"));
        assert!(validation.status().is_success());
        let body = axum::body::to_bytes(validation.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let validation_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(validation_json["data"]["valid"], true);
        assert_eq!(validation_json["data"]["channelCount"], 1);

        let retry_due = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-delivery-attempts:retry-due",
                    r#"{"limit":10,"maxAttempts":3,"backoffSeconds":300}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification retry-due should respond: {error}"));
        assert!(retry_due.status().is_success());
        let body = axum::body::to_bytes(retry_due.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let retry_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(retry_json["code"], 0);
        assert_eq!(retry_json["data"]["scanned"], 0);

        let blocked_delete = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "DELETE",
                    format!("/api/v1/notification-channels/{channel_id}"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification channel delete should respond: {error}"));
        assert_eq!(blocked_delete.status(), axum::http::StatusCode::CONFLICT);
        let body = axum::body::to_bytes(blocked_delete.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let blocked_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_ne!(blocked_json["code"], 0);
        assert!(blocked_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("referenced by 1 notification policy")));
    }


    #[tokio::test]
    async fn notification_channel_test_send_delivers_and_records_detailed_result() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("webhook listener should bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("webhook listener should expose addr: {error}"));
        let received = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));
        let received_server = received.clone();
        let server = tokio::spawn(async move {
            let app = axum::Router::new().route(
                "/notify",
                axum::routing::post(move |body: String| {
                    let received = received_server.clone();
                    async move {
                        *received.lock().await = body;
                        axum::http::StatusCode::ACCEPTED
                    }
                }),
            );
            axum::serve(listener, app)
                .await
                .unwrap_or_else(|error| panic!("webhook listener should serve: {error}"));
        });

        let app = router().await;
        let create_body = format!(
            r#"{{"scopeType":"global","name":"Loopback smoke webhook","provider":"webhook","enabled":true,"config":{{"url":"http://{address}/notify","messageType":"json","template":{{"body":{{"subject":"{{{{subject}}}}","body":"{{{{body}}}}","event":"{{{{eventType}}}}"}}}}}},"safetyPolicy":{{"allowInsecureLoopback":true}}}}"#
        );
        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    &create_body,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification channel create should respond: {error}"));
        assert!(created.status().is_success());
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let created_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let channel_id = created_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("channel id should be present"));

        let tested = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    format!("/api/v1/notification-channels/{channel_id}/test-send"),
                    r#"{"subject":"Smoke subject","body":"Smoke body","severity":"info","eventType":"notification.test"}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification channel test send should respond: {error}"));
        assert!(tested.status().is_success());
        let body = axum::body::to_bytes(tested.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let tested_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(tested_json["code"], 0);
        let result = &tested_json["data"];
        assert_eq!(result["channelId"], channel_id);
        assert_eq!(result["provider"], "webhook");
        assert_eq!(result["delivered"], true);
        assert_eq!(result["statusCode"], 202);
        assert_eq!(result["retryState"], "delivered");
        assert!(
            result["messageId"]
                .as_str()
                .is_some_and(|value| value.starts_with("notification-message_"))
        );
        assert!(
            result["attemptId"]
                .as_str()
                .is_some_and(|value| value.starts_with("notification-delivery_"))
        );
        assert_eq!(result["renderedPayload"]["subject"], "Smoke subject");
        assert!(result["targetRedacted"].as_str().is_some_and(|value| value.starts_with("http://127.0.0.1:") && value.ends_with("/...")));
        assert!(!tested_json.to_string().contains("/notify"));

        let delivered_body = received.lock().await.clone();
        assert!(delivered_body.contains("Smoke subject"), "provider should receive rendered test body: {delivered_body}");
        assert!(!delivered_body.contains("secretRefsJson"));

        let messages = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", format!("/api/v1/notification-messages?source_id={channel_id}&event_type=notification.test")).await)
            .await
            .unwrap_or_else(|error| panic!("notification messages should respond: {error}"));
        assert!(messages.status().is_success());
        let body = axum::body::to_bytes(messages.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let messages_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(messages_json["data"].as_array().map(Vec::len), Some(1));
        assert_eq!(messages_json["data"][0]["status"], "delivered");

        server.abort();
    }

    #[tokio::test]
    async fn notification_channel_test_send_fails_closed_for_disabled_channel() {
        let app = router().await;
        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"global","name":"Disabled test webhook","provider":"webhook","enabled":false,"config":{"url":"https://hooks.example.com/services/test-token","messageType":"json","template":{"body":{"text":"{{subject}}"}}}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("disabled notification channel create should respond: {error}"));
        assert!(created.status().is_success());
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let created_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let channel_id = created_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("channel id should be present"));

        let tested = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    format!("/api/v1/notification-channels/{channel_id}/test-send"),
                    "{}",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("disabled notification channel test should respond: {error}"));
        assert_eq!(tested.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(tested.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let tested_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(tested_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("disabled")));
        assert!(!tested_json.to_string().contains("test-token"));
    }


    #[tokio::test]
    async fn notification_template_api_crud_render_and_policy_linkage_are_validated() {
        let app = router().await;

        let invalid = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{"templateKey":"bad slack","name":"Bad Slack","provider":"slack","messageType":"blockKit","body":{"text":"{{subject}}"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("invalid template should respond: {error}"));
        assert_eq!(invalid.status(), axum::http::StatusCode::BAD_REQUEST);

        let missing_required = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{"templateKey":"ops.slack.missing","name":"Missing blocks","provider":"slack","messageType":"blockKit","body":{"text":"{{subject}}"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("missing required field should respond: {error}"));
        assert_eq!(
            missing_required.status(),
            axum::http::StatusCode::BAD_REQUEST
        );

        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{"templateKey":"ops.slack.failure","name":"Ops Slack failure","description":"Job failure Block Kit","provider":"slack","messageType":"blockKit","enabled":true,"body":{"subject":"[{{severity}}] {{subject}}","body":"{{body}} / {{eventType}}","text":"{{subject}}","blocks":[{"type":"section","text":{"type":"mrkdwn","text":"*{{subject}}*\n{{body}}"}}]},"variables":{"severity":"critical"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("template create should respond: {error}"));
        assert!(created.status().is_success());
        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let created_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(created_json["code"], 0);
        assert_eq!(created_json["data"]["templateKey"], "ops.slack.failure");
        assert_eq!(created_json["data"]["provider"], "slack");
        assert!(!created_json.to_string().contains("secretRefsJson"));
        assert!(!created_json.to_string().contains("routingKey"));
        let template_id = created_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("template id should be present"))
            .to_owned();

        let rendered = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    format!("/api/v1/notification-templates/{template_id}/render"),
                    r#"{"sample":{"subject":"Nightly failed","body":"exit 2","severity":"critical","eventType":"job_instance.failed","resourceId":"billing-nightly"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("template render should respond: {error}"));
        assert!(rendered.status().is_success());
        let body = axum::body::to_bytes(rendered.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let rendered_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(rendered_json["data"]["provider"], "slack");
        assert_eq!(rendered_json["data"]["messageType"], "blockKit");
        assert_eq!(
            rendered_json["data"]["rendered"]["blocks"][0]["text"]["text"],
            "*Nightly failed*\nexit 2"
        );
        assert_eq!(
            rendered_json["data"]["rendered"]["subject"],
            "[critical] Nightly failed"
        );

        let rendered_by_key = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates/ops.slack.failure/render",
                    r#"{"sample":{"subject":"Key render","body":"body","severity":"warning"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("template render by dotted key should respond: {error}"));
        assert!(rendered_by_key.status().is_success());
        let body = axum::body::to_bytes(rendered_by_key.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let rendered_by_key_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(
            rendered_by_key_json["data"]["rendered"]["subject"],
            "[warning] Key render"
        );

        let draft_render = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates/draft.unsaved.slack/render",
                    r#"{"provider":"slack","messageType":"text","template":{"text":"Draft {{subject}}"},"sample":{"subject":"preview"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("unsaved draft render should respond: {error}"));
        assert!(draft_render.status().is_success());
        let body = axum::body::to_bytes(draft_render.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let draft_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(draft_json["data"]["rendered"]["text"], "Draft preview");

        let unknown_token = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{"templateKey":"ops.slack.unsafe","name":"Unsafe","provider":"slack","messageType":"text","body":{"text":"{{subject}} {{env.SECRET}}"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("unknown token template should respond: {error}"));
        assert_eq!(unknown_token.status(), axum::http::StatusCode::BAD_REQUEST);

        let unknown_sample_token = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    format!("/api/v1/notification-templates/{template_id}/render"),
                    r#"{"sample":{"subject":"{{body}}","body":"second-pass"}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("single-pass sample render should respond: {error}"));
        assert!(unknown_sample_token.status().is_success());
        let body = axum::body::to_bytes(unknown_sample_token.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let unknown_sample_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(unknown_sample_json["data"]["rendered"]["text"], "{{body}}");

        let listed = app
            .clone()
            .oneshot(admin_request_builder(app.clone(), "GET", "/api/v1/notification-templates?provider=slack&enabled=true").await)
            .await
            .unwrap_or_else(|error| panic!("templates list should respond: {error}"));
        assert!(listed.status().is_success());
        let body = axum::body::to_bytes(listed.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let listed_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(listed_json["data"].as_array().map(Vec::len), Some(1));

        let patched = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "PATCH",
                    format!("/api/v1/notification-templates/{template_id}"),
                    r#"{"name":"Ops Slack failure v2","enabled":false}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("template patch should respond: {error}"));
        assert!(patched.status().is_success());
        let body = axum::body::to_bytes(patched.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let patched_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert_eq!(patched_json["data"]["name"], "Ops Slack failure v2");
        assert_eq!(patched_json["data"]["enabled"], false);

        let deleted = app
            .clone()
            .oneshot(admin_request_builder(app, "DELETE", format!("/api/v1/notification-templates/{template_id}")).await)
            .await
            .unwrap_or_else(|error| panic!("template delete should respond: {error}"));
        assert!(deleted.status().is_success());
    }

    #[tokio::test]
    async fn notification_template_validation_covers_builtin_provider_message_types() {
        let app = router().await;
        let valid_cases = vec![
            ("slack-text", "slack", "text", serde_json::json!({"text":"{{subject}}"})),
            ("slack-blocks", "slack", "blockKit", serde_json::json!({"text":"{{subject}}","blocks":[{"type":"section","text":{"type":"mrkdwn","text":"{{body}}"}}]})),
            ("slack-attachments", "slack", "attachments", serde_json::json!({"text":"{{subject}}","attachments":[{"text":"{{body}}"}]})),
            ("dingtalk-text", "dingtalk", "text", serde_json::json!({"content":"{{subject}}"})),
            ("dingtalk-markdown", "dingtalk", "markdown", serde_json::json!({"title":"{{subject}}","text":"{{body}}"})),
            ("dingtalk-link", "dingtalk", "link", serde_json::json!({"title":"{{subject}}","text":"{{body}}","messageUrl":"https://example.com/{{resourceId}}"})),
            ("dingtalk-action", "dingtalk", "actionCard", serde_json::json!({"title":"{{subject}}","text":"{{body}}"})),
            ("dingtalk-feed", "dingtalk", "feedCard", serde_json::json!({"links":[{"title":"{{subject}}","messageURL":"https://example.com/{{resourceId}}"}]})),
            ("feishu-text", "feishu", "text", serde_json::json!({"text":"{{subject}}"})),
            ("feishu-post", "feishu", "post", serde_json::json!({"title":"{{subject}}","content":[[{"tag":"text","text":"{{body}}"}]]})),
            ("feishu-image", "feishu", "image", serde_json::json!({"imageKey":"{{resourceId}}"})),
            ("feishu-share", "feishu", "share_chat", serde_json::json!({"shareChatId":"{{resourceId}}"})),
            ("feishu-card", "feishu", "interactive", serde_json::json!({"card":{"header":{"title":{"tag":"plain_text","content":"{{subject}}"}}}})),
            ("wecom-text", "wechat_work", "text", serde_json::json!({"content":"{{subject}}"})),
            ("wecom-markdown", "wechat_work", "markdown", serde_json::json!({"content":"{{body}}"})),
            ("wecom-markdown2", "wechat_work", "markdown_v2", serde_json::json!({"content":"{{body}}"})),
            ("wecom-image", "wechat_work", "image", serde_json::json!({"base64":"{{resourceId}}","md5":"{{messageId}}"})),
            ("wecom-news", "wechat_work", "news", serde_json::json!({"articles":[{"title":"{{subject}}","url":"https://example.com/{{resourceId}}"}]})),
            ("wecom-file", "wechat_work", "file", serde_json::json!({"media_id":"{{resourceId}}"})),
            ("wecom-voice", "wechat_work", "voice", serde_json::json!({"media_id":"{{resourceId}}"})),
            ("wecom-card", "wechat_work", "template_card", serde_json::json!({"templateCard":{"card_type":"text_notice","main_title":{"title":"{{subject}}"}}})),
            ("pager-trigger", "pagerduty", "trigger", serde_json::json!({"summary":"{{subject}}"})),
            ("pager-ack", "pagerduty", "acknowledge", serde_json::json!({})),
            ("pager-resolve", "pagerduty", "resolve", serde_json::json!({})),
            ("webhook-json", "webhook", "json", serde_json::json!({"body":{"text":"{{subject}}"}})),
            ("email-plain", "email", "plain", serde_json::json!({"subject":"{{subject}}","body":"{{body}}"})),
            ("email-html", "email", "html", serde_json::json!({"subject":"{{subject}}","body":"{{body}}","html":"<b>{{body}}</b>"})),
        ];

        for (key, provider, message_type, body) in valid_cases {
            let request = serde_json::json!({
                "templateKey": format!("test.{key}"),
                "name": key,
                "provider": provider,
                "messageType": message_type,
                "body": body
            })
            .to_string();
            let response = app
                .clone()
                .oneshot(
                    admin_json_request_builder(
                        app.clone(),
                        "POST",
                        "/api/v1/notification-templates",
                        &request,
                    )
                    .await,
                )
                .await
                .unwrap_or_else(|error| panic!("{key} template should respond: {error}"));
            assert!(
                response.status().is_success(),
                "{provider}/{message_type} template should be valid, got {}",
                response.status()
            );
        }

        for (key, provider, message_type, body, expected) in [
            (
                "missing-slack-blocks",
                "slack",
                "blockKit",
                serde_json::json!({"text":"{{subject}}"}),
                "requires blocks",
            ),
            (
                "bad-slack-blocks-json",
                "slack",
                "blockKit",
                serde_json::json!({"text":"{{subject}}","blocks":"not-json"}),
                "must be valid JSON array",
            ),
            (
                "missing-dingtalk-link-url",
                "dingtalk",
                "link",
                serde_json::json!({"title":"{{subject}}","text":"{{body}}"}),
                "requires messageUrl",
            ),
            (
                "unsafe-token",
                "webhook",
                "json",
                serde_json::json!({"body":{"text":"{{env.SECRET}}"}}),
                "token is not allowed",
            ),
            (
                "unsupported-message",
                "feishu",
                "unknown",
                serde_json::json!({}),
                "messageType is not supported",
            ),
        ] {
            let request = serde_json::json!({
                "templateKey": format!("test.{key}"),
                "name": key,
                "provider": provider,
                "messageType": message_type,
                "body": body
            })
            .to_string();
            let response = app
                .clone()
                .oneshot(
                    admin_json_request_builder(
                        app.clone(),
                        "POST",
                        "/api/v1/notification-templates",
                        &request,
                    )
                    .await,
                )
                .await
                .unwrap_or_else(|error| panic!("{key} invalid template should respond: {error}"));
            assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap_or_else(|error| panic!("body should collect: {error}"));
            let json: Value = serde_json::from_slice(&body)
                .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
            assert!(
                json["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(expected)),
                "{key} error should mention {expected}: {json}"
            );
        }
    }

    #[tokio::test]
    async fn notification_channel_test_send_rejects_plugin_provider_until_explicitly_supported() {
        let app = router().await;
        let plugin_created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/plugins",
                    r#"{
                      "name":"Ops Bridge Test Send",
                      "kind":"notification",
                      "processorTypes":[],
                      "alertChannelTypes":[{
                        "type":"ops_bridge_test",
                        "label":"Ops Bridge Test",
                        "targetKind":"webhook",
                        "description":"Webhook-compatible notification bridge",
                        "template":{
                          "defaultMessageType":"ticket",
                          "messageTypes":[{
                            "id":"ticket",
                            "label":"Ticket",
                            "description":"Create an external incident ticket",
                            "templateFields":[
                              {"key":"title","label":"Title","type":"string","required":true}
                            ]
                          }],
                          "secretFields":[{"key":"url","label":"Webhook URL secret ref","type":"string","required":true,"secret":true}],
                          "templateVariables":["{{subject}}"],
                          "docs":[]
                        }
                      }],
                      "enabled":true
                    }"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("plugin create should respond: {error}"));
        assert!(plugin_created.status().is_success());

        let channel_created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{
                      "scopeType":"global",
                      "name":"Ops bridge test",
                      "provider":"ops_bridge_test",
                      "enabled":true,
                      "config":{"url":"https://ops.example.invalid/hook","messageType":"ticket","template":{"title":"{{subject}}"}}
                    }"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("plugin channel create should respond: {error}"));
        assert!(channel_created.status().is_success());
        let body = axum::body::to_bytes(channel_created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let created_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let channel_id = created_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("channel id should be present"));

        let tested = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    format!("/api/v1/notification-channels/{channel_id}/test-send"),
                    "{}",
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("plugin channel test send should respond: {error}"));
        assert_eq!(tested.status(), axum::http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(tested.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let tested_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        assert!(tested_json["message"]
            .as_str()
            .is_some_and(|message| message.contains("does not support test send")));
        assert!(!tested_json.to_string().contains("ops.example.invalid"));
    }

    #[tokio::test]
    async fn plugin_provider_template_metadata_drives_channel_and_template_validation() {
        let app = router().await;
        let plugin_created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/plugins",
                    r#"{
                      "name":"Ops Bridge",
                      "kind":"notification",
                      "processorTypes":[],
                      "alertChannelTypes":[{
                        "type":"ops_bridge",
                        "label":"Ops Bridge",
                        "targetKind":"webhook",
                        "description":"Webhook-compatible notification bridge",
                        "template":{
                          "defaultMessageType":"ticket",
                          "messageTypes":[{
                            "id":"ticket",
                            "label":"Ticket",
                            "description":"Create an external incident ticket",
                            "templateFields":[
                              {"key":"title","label":"Title","type":"string","required":true},
                              {"key":"body","label":"Body","type":"textarea"}
                            ]
                          }],
                          "secretFields":[{"key":"url","label":"Webhook URL secret ref","type":"string","required":true,"secret":true}],
                          "templateVariables":["{{subject}}","{{body}}"],
                          "docs":[]
                        }
                      }],
                      "enabled":true
                    }"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("plugin create should respond: {error}"));
        assert!(plugin_created.status().is_success());

        let channel_created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{
                      "scopeType":"global",
                      "name":"Ops bridge",
                      "provider":"ops_bridge",
                      "enabled":true,
                      "secretRefs":{"url":"env:OPS_BRIDGE_WEBHOOK_URL"},
                      "config":{"messageType":"ticket","template":{"title":"{{subject}}","body":"{{body}}"}}
                    }"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("plugin channel create should respond: {error}"));
        assert!(
            channel_created.status().is_success(),
            "plugin provider channel should use plugin template metadata instead of built-in webhook defaults: {}",
            channel_created.status()
        );

        let template_created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{
                      "templateKey":"ops.bridge.ticket",
                      "name":"Ops bridge ticket",
                      "provider":"ops_bridge",
                      "messageType":"ticket",
                      "body":{"title":"{{subject}}","body":"{{body}}"}
                    }"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("plugin template create should respond: {error}"));
        assert!(template_created.status().is_success());

        let invalid_template = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app,
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{
                      "templateKey":"ops.bridge.missing-title",
                      "name":"Ops bridge missing title",
                      "provider":"ops_bridge",
                      "messageType":"ticket",
                      "body":{"body":"{{body}}"}
                    }"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("plugin invalid template should respond: {error}"));
        assert_eq!(invalid_template.status(), axum::http::StatusCode::BAD_REQUEST);
    }

fn assert_provider_template(channel_types: &[Value], provider: &str, expected_message_types: &[&str]) {
    let item = channel_types
        .iter()
        .find(|item| item["type"] == provider)
        .unwrap_or_else(|| panic!("{provider} type should be present"));
    let template = &item["template"];
    for key in [
        "messageTypes",
        "configFields",
        "secretFields",
        "templateVariables",
        "docs",
    ] {
        assert!(
            !template[key].is_null(),
            "{provider} template should expose {key}"
        );
    }
    let message_types = template["messageTypes"]
        .as_array()
        .unwrap_or_else(|| panic!("{provider} messageTypes should be an array"));
    for expected in expected_message_types {
        assert!(
            message_types.iter().any(|item| item["id"] == *expected || item["type"] == *expected),
            "{provider} messageTypes should include {expected}"
        );
    }
    let variables = template["templateVariables"]
        .as_array()
        .unwrap_or_else(|| panic!("{provider} templateVariables should be an array"));
    for variable in [
        "subject",
        "body",
        "eventType",
        "resourceType",
        "resourceId",
        "severity",
        "messageId",
        "policyId",
        "dedupeKey",
    ] {
        assert!(
            variables
                .iter()
                .any(|item| item == variable || item == &format!("{{{{{variable}}}}}")),
            "{provider} templateVariables should include {variable}"
        );
    }
}

fn assert_provider_template_has_field(
    channel_types: &[Value],
    provider: &str,
    expected_field: &str,
) {
    let provider_type = channel_types
        .iter()
        .find(|item| item["type"] == provider)
        .unwrap_or_else(|| panic!("{provider} provider should be present"));
    assert!(
        provider_type["template"].to_string().contains(expected_field),
        "{provider} provider template should include field {expected_field}"
    );
}

fn assert_provider_template_examples(channel_types: &[Value], provider: &str) {
    let provider_type = channel_types
        .iter()
        .find(|item| item["type"] == provider)
        .unwrap_or_else(|| panic!("{provider} provider should be present"));
    let message_types = provider_type["template"]["messageTypes"]
        .as_array()
        .unwrap_or_else(|| panic!("{provider} messageTypes should be an array"));
    for message_type in message_types {
        let message_type_id = message_type["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{provider} message type should have id"));
        let examples = message_type["examples"].as_array().unwrap_or_else(|| {
            panic!("{provider}/{message_type_id} should expose examples")
        });
        assert!(
            (1..=2).contains(&examples.len()),
            "{provider}/{message_type_id} should expose 1-2 examples"
        );
        for example in examples {
            assert!(
                example["name"].as_str().is_some_and(|value| !value.trim().is_empty()),
                "{provider}/{message_type_id} example should have a name"
            );
            assert!(
                example.get("template").is_some() || example.get("config").is_some(),
                "{provider}/{message_type_id} example should provide template or config"
            );
            let rendered = example.to_string();
            assert!(
                !rendered.contains("***redacted***") && !rendered.contains("xoxb-"),
                "{provider}/{message_type_id} example must not contain redacted markers or raw tokens"
            );
        }
    }
}


    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_notification_node_materializes_notification_center_message_and_attempt() {
        let app = router().await;
        let channel_created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"global","name":"Workflow webhook","provider":"webhook","enabled":true,"config":{"url":"https://hooks.example.com/services/workflow-token","messageType":"json","template":{"body":{"text":"{{subject}}"}}}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification channel create should respond: {error}"));
        assert!(channel_created.status().is_success());
        let body = axum::body::to_bytes(channel_created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        let channel_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("body should be JSON: {error}"));
        let channel_id = channel_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("channel id should exist"));

        let template_created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-templates",
                    r#"{"templateKey":"workflow.node.notice","name":"Workflow node notice","provider":"webhook","messageType":"json","enabled":true,"body":{"body":{"text":"Workflow {{resourceId}} notification","node":"{{resourceId}}","event":"{{eventType}}"}}}"#,
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification template create should respond: {error}"));
        assert!(template_created.status().is_success());
        let body = axum::body::to_bytes(template_created.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("template body should collect: {error}"));
        let template_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("template body should be JSON: {error}"));
        let template_id = template_json["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("template id should exist"));

        let workflow_body = serde_json::json!({
            "name": "workflow-notify",
            "definition": {
                "nodes": [
                    {
                        "key": "notify",
                        "name": "Notify",
                        "kind": "notification",
                        "config": {
                            "channelRefs": [{"channelId": channel_id}],
                            "templateRef": template_id,
                            "subject": "Workflow notification requested",
                            "body": "A workflow notification node was materialized",
                            "severity": "warning"
                        }
                    }
                ],
                "edges": []
            }
        })
        .to_string();
        let workflow = post_json(app.clone(), "/api/v1/workflows", &workflow_body).await;
        assert_eq!(workflow["code"], 0);
        let workflow_id = workflow["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow id should exist"));

        let run = post_json(
            app.clone(),
            &format!("/api/v1/workflows/{workflow_id}/run"),
            r#"{"triggerType":"api"}"#,
        )
        .await;
        assert_eq!(run["code"], 0);
        assert!(run["data"]["id"].as_str().is_some_and(|value| !value.is_empty()));

        let materialized = post_json(
            app.clone(),
            "/api/v1/workflow-instances/materialize-next",
            "{}",
        )
        .await;
        assert_eq!(materialized["code"], 0);
        assert_eq!(materialized["data"]["node"]["nodeKey"], "notify");
        assert_eq!(materialized["data"]["node"]["status"], "succeeded");

        let workflow_node_instance_id = materialized["data"]["node"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("workflow node instance id should exist"));

        let messages = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app.clone(),
                    "GET",
                    format!("/api/v1/notification-messages?source_id={workflow_node_instance_id}&event_type=workflow_node.notification_requested"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification messages should respond: {error}"));
        assert!(messages.status().is_success());
        let body = axum::body::to_bytes(messages.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("messages body should collect: {error}"));
        let messages_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("messages body should be JSON: {error}"));
        assert_eq!(messages_json["code"], 0);
        let messages = messages_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("messages should be an array"));
        assert_eq!(messages.len(), 1, "expected one workflow notification message: {messages_json}");
        let message = &messages[0];
        assert_eq!(message["sourceType"], "workflow_node_instance");
        assert_eq!(message["sourceId"], workflow_node_instance_id);
        assert_eq!(message["eventType"], "workflow_node.notification_requested");
        assert_eq!(message["resourceType"], "workflow_node");
        assert_eq!(message["resourceId"], "notify");
        assert_eq!(message["severity"], "warning");
        assert_eq!(message["subject"], "Workflow notification requested");
        assert_eq!(message["body"], "A workflow notification node was materialized");
        let payload = message["payloadJson"]
            .as_str()
            .and_then(|payload| serde_json::from_str::<Value>(payload).ok())
            .unwrap_or_else(|| panic!("payloadJson should be JSON: {}", message["payloadJson"]));
        assert_eq!(payload["workflowNodeInstanceId"], workflow_node_instance_id);
        assert_eq!(payload["templateRef"], template_id);
        assert_eq!(payload["templateKey"], "workflow.node.notice");
        let policy_id = message["policyId"]
            .as_str()
            .unwrap_or_else(|| panic!("message policy id should exist"));

        let attempts = app
            .clone()
            .oneshot(
                admin_request_builder(
                    app,
                    "GET",
                    format!("/api/v1/notification-delivery-attempts?policy_id={policy_id}"),
                )
                .await,
            )
            .await
            .unwrap_or_else(|error| panic!("delivery attempts should respond: {error}"));
        assert!(attempts.status().is_success());
        let body = axum::body::to_bytes(attempts.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("attempts body should collect: {error}"));
        let attempts_json: Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("attempts body should be JSON: {error}"));
        let attempts = attempts_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("attempts should be an array"));
        assert_eq!(attempts.len(), 1, "expected one delivery attempt: {attempts_json}");
        assert_eq!(attempts[0]["channelId"], channel_id);
        assert_eq!(attempts[0]["retryState"], "retry_pending");
    }
