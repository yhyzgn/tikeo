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
        assert_provider_template_example_secret_refs_are_channel_private_values(channel_types);
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
