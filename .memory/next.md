# Latest completed slice

- 2026-06-25: YAML migration / warning-clean full business-output regression is complete locally. Rust SDK generated protobuf warnings are fixed at the generator/post-processing root, migration CLI smoke assertions now match current output, all source suppression bypasses are absent except red-line text in `AGENTS.md`/`prompt.md`, and business-output smokes for notifications, management trigger, SDK API keys, migration CLI, web/docs, SDKs/demos, and Docker images passed.

# Next Work

## Current priority direction

当前仍是功能/模块测试验收与发布阶段，不收缩、不臆造。Notification Center、job notification bindings、message trace、notification drawer UX、模板变量目录、docs Docker publish、release version sync、v0.2.9 formal release 均已有完成记录。Docs 站点已从 README 级介绍升级为面向人的运维/集成手册，但后续任何新功能都必须继续补用户路径、参考页和 zh-CN 镜像。

## Immediate next slice

1. v0.2.10 docs manual release is complete: remote main/tag, GitHub Release, CI/Coverage, Docker server/web/docs, and SDK publish workflows all passed.
2. Continue acceptance hardening items that are still not complete: alert-rule dual-write/backfill to Notification Center policies, delivery lease/idempotency hardening, and any missing UI/docs/test coverage found during module acceptance.
3. For the next code change, first reconcile local git with remote commit `c3f957ff50f51405e761c7d8fa75cc7c3bed6bf1` if normal git fetch is available; this release used GitHub REST API because local git smart protocol was blocked.

## Current verified baseline

- Notification Center baseline: explicit schema/repositories, routes/OpenAPI, config defaults, background worker, Web page, docs, RBAC/menu seed, job lifecycle materialization, alert event materialization, workflow notification node materialization, redaction, generic retry/DLQ, and provider delivery for webhook-style/Slack/DingTalk/Feishu/WeCom/PagerDuty/Email/plugin webhook-compatible providers.
- Template baseline: `notification_templates` has migration/entity/repository, CRUD/list/get/delete API, render preview endpoint, safe token replacement, provider/message-type validation, Web template drawer/preview, and policy template AutoComplete options restricted to enabled stored templates matching selected channel providers.
- Workflow/job notification baseline: Web uses Notification Center channel/template selectors; backend validates refs in dry-run/create/update/validate; runtime inline policy materialization fails closed if refs go stale; job notification bindings provide task-status delivery configuration and message trace/log passthrough.
- Docs site baseline: `docs/` is the Docusaurus module with Dockerfile/nginx config targeting `yhyzgn/tikeo-docs`; English and zh-CN operator manuals cover quickstart, installation, production deployment, SDK/API integration, configuration, Notification Center, SDKs, development, troubleshooting, and publishing/search/LLM surfaces.

## Standing constraints

- Functional/module testing acceptance phase: do not shrink scope; if anything missing/incomplete/untested/hallucinated is found, fill it production-grade or record a real blocker.
- Docs must be written for humans: prerequisites, exact commands, expected observations, troubleshooting, and production checklist. No internal handoff language in public docs.
- Alerts = rules/events/incidents; Notifications = channels/templates/policies/messages/delivery. Inbound webhook event sources are job triggers, not outbound notification channels.
- Never leak webhook tokens, SMTP passwords, PagerDuty keys, header credentials, or secret ref values in API responses/UI/docs/logs.
- Worker 重要可见性状态不得只在内存。
- 禁止约定命名匹配；必须使用结构化字段、labels 或 structuredCapabilities。
- 中文 i18n 必须完整中文，英文 i18n 必须英文，不要中英混合 label。
- 新 schema 变更必须进入显式 SeaORM migration；不得在 `connect_and_migrate` 后挂未记录的兼容补丁。
- Helm chart 不能部署业务 Worker 或创建业务 Worker 入站 Service；Worker 只能主动出站连接 Tikeo Worker Tunnel。
- 源文件 <=1500 行；`mod.rs` / `lib.rs` 等入口文件只做声明和 re-export。
- Web/frontend/docs package management and command execution must use `bun` / `bunx` unless explicitly overridden.

## New configuration note

- For Feishu/Lark or other office-platform cards opened outside the Tikeo browser origin, set `notification_delivery.public_console_base_url` / `TIKEO__NOTIFICATION_DELIVERY__PUBLIC_CONSOLE_BASE_URL` to the externally reachable Web base URL so card buttons contain absolute public console links.

- Red line remains active: do not add/keep `#[allow(...)]`, `#![allow(...)]`, `#[expect(...)]`, lint downgrades, or equivalent suppression bypasses; fix warning/error root causes instead.
