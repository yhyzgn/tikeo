    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn notification_center_api_redacts_channels_and_validates_policies() {
        let app = router().await;

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
            .all(|item| item["supportsTestSend"] == false));
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

        let created = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Billing Feishu","provider":"feishu","enabled":true,"config":{"url":"https://open.feishu.cn/open-apis/bot/v2/hook/super-secret-token","mentionAll":true},"secretRefs":{"signingKey":"env:FEISHU_BOT_SECRET"}}"#,
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

        let secret_ref_only = app
            .clone()
            .oneshot(
                admin_json_request_builder(
                    app.clone(),
                    "POST",
                    "/api/v1/notification-channels",
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Secret ref webhook","provider":"webhook","enabled":true,"config":{},"secretRefs":{"url":"env:TIKEO_NOTIFICATION_WEBHOOK_URL","authorization":"env:TIKEO_NOTIFICATION_AUTH"}}"#,
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
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Secret ref PagerDuty","provider":"pagerduty","enabled":true,"config":{},"secretRefs":{"routingKey":"env:TIKEO_PAGERDUTY_ROUTING_KEY"}}"#,
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
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Secret ref email","provider":"email","enabled":true,"config":{"to":["ops@example.com"],"username":"tikeo"},"secretRefs":{"smtpUrl":"env:TIKEO_SMTP_URL","password":"env:TIKEO_SMTP_PASSWORD"}}"#,
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
                    r#"{"scopeType":"app","namespace":"default","app":"billing","name":"Disabled webhook","provider":"webhook","enabled":false,"config":{"url":"https://hooks.example.com/services/disabled-token"}}"#,
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
        assert_eq!(listed_json["data"].as_array().map(Vec::len), Some(5));
        assert!(!listed_json.to_string().contains("super-secret-token"));
        assert!(!listed_json.to_string().contains("FEISHU_BOT_SECRET"));

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
