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

fn notification_channel_example_suffix(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = true;
    for item in value.chars() {
        if item.is_ascii_uppercase() {
            if !previous_was_separator {
                normalized.push('_');
            }
            normalized.push(item);
            previous_was_separator = false;
        } else if item.is_ascii_alphanumeric() {
            normalized.push(item.to_ascii_uppercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }
    let trimmed = normalized.trim_matches('_').to_owned();
    if trimmed.is_empty() {
        "CUSTOM".to_owned()
    } else {
        trimmed
    }
}

fn assert_provider_template_example_secret_refs_are_channel_private_values(channel_types: &[Value]) {
    for provider_type in channel_types.iter().filter(|item| {
        item["type"] == "slack"
            || item["type"] == "dingtalk"
            || item["type"] == "feishu"
            || item["type"] == "wechat_work"
            || item["type"] == "pagerduty"
            || item["type"] == "email"
            || item["type"] == "webhook"
    }) {
        let provider = provider_type["type"]
            .as_str()
            .unwrap_or_else(|| panic!("provider type should be a string"));
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
            for example in examples {
                let secret_refs = &example["secretRefs"];
                let rendered = secret_refs.to_string();
                assert!(
                    !rendered.contains("env:TIKEO_NOTIFICATION_CHANNEL_"),
                    "{provider}/{message_type_id} example secretRefs should demonstrate direct channel-private values"
                );
                assert!(
                    example["description"]
                        .as_str()
                        .is_some_and(|description| description.contains("direct values")
                            && description.contains("env:NAME")),
                    "{provider}/{message_type_id} example should document direct values and env compatibility"
                );
                match provider {
                    "slack" | "dingtalk" | "feishu" | "wechat_work" | "webhook" => {
                        assert!(
                            secret_refs["url"]
                                .as_str()
                                .is_some_and(|value| value.starts_with("https://")),
                            "{provider}/{message_type_id} should include a direct webhook URL"
                        );
                    }
                    "pagerduty" => assert!(
                        secret_refs["routingKey"].as_str().is_some_and(|value| {
                            value.contains("PAGERDUTY")
                                && value.contains(&notification_channel_example_suffix(message_type_id))
                        }),
                        "{provider}/{message_type_id} should include a direct routing key placeholder"
                    ),
                    "email" => {
                        assert!(secret_refs.get("smtpUrl").is_none());
                        assert!(
                            secret_refs["password"].as_str().is_some_and(|value| {
                                value.contains("SMTP")
                                    && value.contains(&notification_channel_example_suffix(message_type_id))
                            }),
                            "email/{message_type_id} should include a direct SMTP password placeholder"
                        );
                    }
                    _ => {}
                }
                if matches!(provider, "dingtalk" | "feishu") {
                    assert!(
                        secret_refs["signingKey"].as_str().is_some_and(|value| {
                            value.contains("SEC_")
                                && value.contains(&notification_channel_example_suffix(message_type_id))
                        }),
                        "{provider}/{message_type_id} should include a direct signing secret placeholder"
                    );
                }
                for global_ref in [
                    "TIKEO_NOTIFICATION_WEBHOOK_URL",
                    "SLACK_WEBHOOK_URL",
                    "DINGTALK_WEBHOOK_URL",
                    "FEISHU_WEBHOOK_URL",
                    "FEISHU_BOT_SECRET",
                    "WECOM_WEBHOOK_URL",
                    "PAGERDUTY_ROUTING_KEY",
                    "TIKEO_SMTP_URL",
                ] {
                    assert!(
                        !rendered.contains(global_ref),
                        "{provider}/{message_type_id} example should not use shared ref {global_ref}"
                    );
                }
            }
        }
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


