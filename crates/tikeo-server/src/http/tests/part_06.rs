    #[tokio::test]
    async fn notification_center_api_redacts_channels_and_validates_policies() {
        let app = router().await;
        create_billing_scope(app.clone()).await;

        assert_notification_channel_types(app.clone()).await;
        assert_invalid_notification_channel_inputs(app.clone()).await;
        let channel_id = create_redacted_feishu_channel(app.clone()).await;
        assert_secret_ref_only_channels(app.clone()).await;
        assert_policy_channel_validation(app.clone(), &channel_id).await;
        assert_notification_channel_listing(app.clone(), &channel_id).await;
        assert_template_policy_validation(app.clone(), &channel_id).await;
        let policy_id = create_and_validate_notification_policy(app.clone(), &channel_id).await;
        assert_retry_due_and_blocked_channel_delete(app, &channel_id, &policy_id).await;
    }

    async fn create_billing_scope(app: axum::Router) {
        let _billing_scope = post_json(
            app,
            "/api/v1/apps",
            r#"{"namespace":"default","name":"billing"}"#,
        )
        .await;
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("body should collect: {error}"));
        serde_json::from_slice(&body).unwrap_or_else(|error| panic!("body should be JSON: {error}"))
    }

    async fn admin_response(
        app: axum::Router,
        method: &str,
        uri: impl ToString,
        body: impl ToString,
    ) -> axum::response::Response {
        let body = body.to_string();
        app.clone()
            .oneshot(admin_json_request_builder(app, method, uri, &body).await)
            .await
            .unwrap_or_else(|error| panic!("admin JSON route should respond: {error}"))
    }

    async fn admin_value(
        app: axum::Router,
        method: &str,
        uri: impl ToString,
        body: impl ToString,
    ) -> Value {
        let response = admin_response(app, method, uri, body).await;
        assert!(response.status().is_success());
        response_json(response).await
    }

    async fn assert_bad_request(
        app: axum::Router,
        method: &str,
        uri: impl ToString,
        body: impl ToString,
        expected_message: Option<&str>,
    ) {
        let response = admin_response(app, method, uri, body).await;
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        if let Some(expected) = expected_message {
            let json = response_json(response).await;
            assert!(
                json["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(expected)),
                "error should mention {expected}: {json}"
            );
        }
    }

    async fn assert_notification_channel_types(app: axum::Router) {
        let types = app
            .clone()
            .oneshot(
                admin_request_builder(app, "GET", "/api/v1/notification-channel-types").await,
            )
            .await
            .unwrap_or_else(|error| panic!("notification channel types should respond: {error}"));
        assert!(types.status().is_success());
        let types_json = response_json(types).await;
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
        let channel_types = types_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("types should be an array"));
        let email_type = channel_types
            .iter()
            .find(|item| item["type"] == "email")
            .unwrap_or_else(|| panic!("email type should be present"));
        assert!(email_type["secretConfigKeys"]
            .as_array()
            .unwrap_or_else(|| panic!("email secret keys should be an array"))
            .iter()
            .any(|key| key == "password"));

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
        assert_provider_metadata_does_not_embed_runtime_examples(channel_types);
        assert_provider_template(
            channel_types,
            "wechat_work",
            &[
                "text",
                "markdown",
                "markdown_v2",
                "image",
                "news",
                "file",
                "voice",
                "template_card",
            ],
        );
        assert_provider_template(channel_types, "pagerduty", &["trigger", "acknowledge", "resolve"]);
        assert_provider_template(channel_types, "email", &["plain", "html"]);
        assert_provider_template(channel_types, "webhook", &["json"]);
        for provider in ["slack", "dingtalk", "feishu", "wechat_work", "pagerduty", "email", "webhook"] {
            assert_provider_template_has_no_runtime_examples(channel_types, provider);
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
    }

    async fn assert_invalid_notification_channel_inputs(app: axum::Router) {
        for body in [
            r#"{"scopeType":"app","name":"Missing scope","provider":"webhook","enabled":true,"secretRefs":{"url":"env:TIKEO_NOTIFICATION_WEBHOOK_URL"}}"#,
            r#"{"scopeType":"global","name":"Bad message type","provider":"wechat_work","enabled":true,"secretRefs":{"url":"env:TIKEO_WECOM_WEBHOOK_URL"},"config":{"messageType":"unsupported"}}"#,
            r#"{"scopeType":"global","name":"Bad template","provider":"wechat_work","enabled":true,"secretRefs":{"url":"env:TIKEO_WECOM_WEBHOOK_URL"},"config":{"messageType":"voice","template":{}}}"#,
            r#"{"scopeType":"global","name":"Raw routing key","provider":"pagerduty","enabled":true,"config":{"routingKey":"pd-secret-routing-key"}}"#,
            r#"{"scopeType":"global","name":"Raw auth header","provider":"webhook","enabled":true,"config":{"url":"https://hooks.example.com/services/token","headers":{"Authorization":"Bearer plain-secret"},"messageType":"json","template":{"body":{"text":"{{subject}}"}}}}"#,
        ] {
            assert_bad_request(
                app.clone(),
                "POST",
                "/api/v1/notification-channels",
                body,
                None,
            )
            .await;
        }
    }

    async fn create_redacted_feishu_channel(app: axum::Router) -> String {
        let created_json = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-channels",
            r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Billing Feishu","provider":"feishu","enabled":true,"config":{"url":"https://open.feishu.cn/open-apis/bot/v2/hook/super-secret-token","mentionAll":true,"messageType":"text","template":{"text":"{{subject}}"}},"secretRefs":{"signingKey":"env:FEISHU_BOT_SECRET"}}"#,
        )
        .await;
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

        let edit_json = admin_value(
            app,
            "PATCH",
            format!("/api/v1/notification-channels/{channel_id}"),
            r#"{"name":"Billing Feishu renamed"}"#,
        )
        .await;
        assert_eq!(edit_json["data"]["targetRedacted"], "https://open.feishu.cn/...");
        assert_eq!(edit_json["data"]["secretConfigured"], true);
        assert!(!edit_json.to_string().contains("FEISHU_BOT_SECRET"));
        channel_id
    }

    async fn assert_secret_ref_only_channels(app: axum::Router) {
        let webhook = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-channels",
            r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Secret ref webhook","provider":"webhook","enabled":true,"config":{"messageType":"json"},"secretRefs":{"url":"env:TIKEO_NOTIFICATION_WEBHOOK_URL","authorization":"env:TIKEO_NOTIFICATION_AUTH"}}"#,
        )
        .await;
        assert_eq!(webhook["data"]["targetConfigured"], true);
        assert_eq!(webhook["data"]["targetRedacted"], "webhook:secret-ref");
        assert_eq!(webhook["data"]["secretConfigured"], true);
        assert!(webhook["data"].get("secretRefsJson").is_none());
        assert!(!webhook.to_string().contains("TIKEO_NOTIFICATION_WEBHOOK_URL"));
        assert!(!webhook.to_string().contains("TIKEO_NOTIFICATION_AUTH"));

        let pagerduty = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-channels",
            r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Secret ref PagerDuty","provider":"pagerduty","enabled":true,"config":{"messageType":"trigger","template":{"summary":"{{subject}}"}},"secretRefs":{"routingKey":"env:TIKEO_PAGERDUTY_ROUTING_KEY"}}"#,
        )
        .await;
        assert_eq!(pagerduty["data"]["targetRedacted"], "pagerduty:secret-ref");
        assert!(pagerduty["data"].get("secretRefsJson").is_none());
        assert!(!pagerduty.to_string().contains("TIKEO_PAGERDUTY_ROUTING_KEY"));

        let email = admin_value(
            app,
            "POST",
            "/api/v1/notification-channels",
            r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Secret ref email","provider":"email","enabled":true,"config":{"to":["ops@example.com"],"username":"tikeo","messageType":"plain","template":{"subject":"{{subject}}","body":"{{body}}"}},"secretRefs":{"smtpUrl":"env:TIKEO_SMTP_URL","password":"env:TIKEO_SMTP_PASSWORD"}}"#,
        )
        .await;
        assert_eq!(email["data"]["targetRedacted"], "ops@example.com");
        assert!(email["data"].get("secretRefsJson").is_none());
        assert!(!email.to_string().contains("TIKEO_SMTP_URL"));
        assert!(!email.to_string().contains("TIKEO_SMTP_PASSWORD"));
    }

    async fn assert_policy_channel_validation(app: axum::Router, channel_id: &str) {
        assert_bad_request(
            app.clone(),
            "POST",
            "/api/v1/notification-policies",
            r#"{"ownerType":"job","ownerId":"job-billing-nightly","name":"Missing channel policy","eventFamily":"job_instance","eventFilter":{"statuses":["failed"]},"channelRefs":[{"channelId":"notification-channel-missing"}],"severity":"critical","enabled":true,"dedupeSeconds":300}"#,
            Some("channel does not exist"),
        )
        .await;

        let disabled = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-channels",
            r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Disabled webhook","provider":"webhook","enabled":false,"config":{"url":"https://hooks.example.com/services/disabled-token","messageType":"json","template":{"body":"{}"}}}"#,
        )
        .await;
        let disabled_channel_id = disabled["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("disabled channel id should be present"));
        assert_bad_request(
            app,
            "POST",
            "/api/v1/notification-policies",
            format!(
                r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"Disabled channel policy","eventFamily":"job_instance","eventFilter":{{"statuses":["failed"]}},"channelRefs":[{{"channelId":"{disabled_channel_id}"}}],"severity":"critical","enabled":true,"dedupeSeconds":300}}"#
            ),
            Some("channel is disabled"),
        )
        .await;
        assert!(!channel_id.is_empty());
    }

    async fn assert_notification_channel_listing(app: axum::Router, channel_id: &str) {
        let listed = app
            .clone()
            .oneshot(admin_request_builder(app, "GET", "/api/v1/notification-channels").await)
            .await
            .unwrap_or_else(|error| panic!("notification channels should list: {error}"));
        assert!(listed.status().is_success());
        let listed_json = response_json(listed).await;
        let listed_channels = listed_json["data"]
            .as_array()
            .unwrap_or_else(|| panic!("notification channels list should be an array"));
        assert!(
            listed_channels.len() >= 4,
            "list should include user-created channels without schema-seeded example rows: {listed_json}"
        );
        assert!(
            listed_channels.iter().all(|item| !item["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("notification-channel-example-"))),
            "schema migrations must not expose notification channel example rows: {listed_json}"
        );
        assert!(listed_channels.iter().any(|item| item["id"] == channel_id));
        assert!(!listed_json.to_string().contains("super-secret-token"));
        assert!(!listed_json.to_string().contains("FEISHU_BOT_SECRET"));
    }

    async fn assert_template_policy_validation(app: axum::Router, channel_id: &str) {
        let _slack_template = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-templates",
            r#"{"templateKey":"ops.slack.for-feishu-mismatch","name":"Slack mismatch","provider":"slack","messageType":"text","enabled":true,"body":{"text":"{{subject}}"}}"#,
        )
        .await;
        assert_bad_request(
            app.clone(),
            "POST",
            "/api/v1/notification-policies",
            format!(
                r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"Mismatched template policy","eventFamily":"job_instance","eventFilter":{{"statuses":["failed"]}},"channelRefs":[{{"channelId":"{channel_id}"}}],"templateRef":"ops.slack.for-feishu-mismatch","severity":"critical","enabled":true,"dedupeSeconds":300}}"#
            ),
            Some("template provider slack does not match channel provider"),
        )
        .await;

        let _disabled_template = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-templates",
            r#"{"templateKey":"ops.feishu.disabled","name":"Disabled Feishu","provider":"feishu","messageType":"text","enabled":false,"body":{"text":"{{subject}}"}}"#,
        )
        .await;
        for (name, template_ref, expected) in [
            ("Disabled template policy", "ops.feishu.disabled", "template is disabled"),
            ("Missing template policy", "ops.feishu.missing", "template does not exist"),
        ] {
            assert_bad_request(
                app.clone(),
                "POST",
                "/api/v1/notification-policies",
                format!(
                    r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"{name}","eventFamily":"job_instance","eventFilter":{{"statuses":["failed"]}},"channelRefs":[{{"channelId":"{channel_id}"}}],"templateRef":"{template_ref}","severity":"critical","enabled":true,"dedupeSeconds":300}}"#
                )
                .as_str(),
                Some(expected),
            )
            .await;
        }

        let _feishu_template = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-templates",
            r#"{"templateKey":"ops.feishu.enabled","name":"Enabled Feishu","provider":"feishu","messageType":"text","enabled":true,"body":{"text":"{{subject}}"}}"#,
        )
        .await;
        let webhook_channel = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-channels",
            r#"{"scopeType":"global","name":"Global webhook","provider":"webhook","enabled":true,"config":{"messageType":"json","template":{"body":{"text":"{{subject}}"}}},"secretRefs":{"url":"env:TIKEO_NOTIFICATION_WEBHOOK_URL"}}"#,
        )
        .await;
        let webhook_channel_id = webhook_channel["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("webhook channel id should be present"));
        assert_bad_request(
            app,
            "POST",
            "/api/v1/notification-policies",
            format!(
                r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"Mixed provider template policy","eventFamily":"job_instance","eventFilter":{{"statuses":["failed"]}},"channelRefs":[{{"channelId":"{channel_id}"}},{{"channelId":"{webhook_channel_id}"}}],"templateRef":"ops.feishu.enabled","severity":"critical","enabled":true,"dedupeSeconds":300}}"#
            ),
            Some("channel provider(s): webhook"),
        )
        .await;
    }

    async fn create_and_validate_notification_policy(app: axum::Router, channel_id: &str) -> String {
        let created = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-policies",
            format!(
                r#"{{"ownerType":"job","ownerId":"job-billing-nightly","name":"Billing failure notifications","eventFamily":"job_instance","eventFilter":{{"statuses":["failed","retry_exhausted"]}},"channelRefs":[{{"channelId":"{channel_id}"}}],"severity":"critical","enabled":true,"dedupeSeconds":300}}"#
            ),
        )
        .await;
        assert_eq!(created["code"], 0);
        assert_eq!(created["data"]["eventFamily"], "job_instance");
        let policy_id = created["data"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("policy id should be present"))
            .to_owned();

        assert_bad_request(
            app.clone(),
            "PATCH",
            format!("/api/v1/notification-policies/{policy_id}"),
            r#"{"channelRefs":[{"channelId":"notification-channel-missing"}]}"#,
            None,
        )
        .await;
        let validation = admin_value(
            app,
            "POST",
            format!("/api/v1/notification-policies/{policy_id}:validate"),
            "{}",
        )
        .await;
        assert_eq!(validation["data"]["valid"], true);
        assert_eq!(validation["data"]["channelCount"], 1);
        policy_id
    }

    async fn assert_retry_due_and_blocked_channel_delete(
        app: axum::Router,
        channel_id: &str,
        policy_id: &str,
    ) {
        assert!(!policy_id.is_empty());
        let retry = admin_value(
            app.clone(),
            "POST",
            "/api/v1/notification-delivery-attempts:retry-due",
            r#"{"limit":10,"maxAttempts":3,"backoffSeconds":300}"#,
        )
        .await;
        assert_eq!(retry["code"], 0);
        assert_eq!(retry["data"]["scanned"], 0);

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
        let blocked_json = response_json(blocked_delete).await;
        assert_ne!(blocked_json["code"], 0);
        assert!(blocked_json["message"].as_str().is_some_and(|message| {
            message.contains("referenced by 1 notification policy")
        }));
    }
