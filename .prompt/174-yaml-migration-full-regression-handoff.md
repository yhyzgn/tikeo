# 174 — YAML migration / warning-clean full regression handoff

## 本阶段已完成

- dev/tikeo 配置统一迁移到 YAML 后，按历史清单完成全线业务输出回归，不只以编译通过作为验收。
- Rust SDK standalone protobuf 生成警告已从源头修复：`sdks/rust/tikeo/build.rs` 复用生成后处理，补文档、压缩编码默认值、const setter、非 float message `Eq` derive；`worker.proto` 注释修正为 rustdoc 友好格式。
- Migration CLI full-chain smoke 的断言更新为当前业务输出：固定 Spring Boot starter 依赖 `0.3.10`、legacy scheduler keys 从原配置中移除、保留最小 worker/management placeholders、import payload 归档且不调用 server。
- 严禁 warning/错误屏蔽规则已进入项目规范；当前精确扫描仅命中 `AGENTS.md` / `prompt.md` 中的规则文本，没有 source `#[allow]` / `#[expect]` 绕过。

## 已验证

- Rust/backend：`cargo fmt --all -- --check`、`cargo build --workspace --all-features`、`cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings`、`cargo test --workspace --all-features`、`RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`。
- Contract/static：GitHub workflow/docs/management contract tests、Node runtime verifier、source-size、diff-check、suppression scan。
- Web/docs：`bun run --cwd web lint/typecheck/build`、`bun test --cwd web src`、`bun run --cwd docs docs:typecheck`、`bun run --cwd docs docs:build`、web live route smoke。
- SDK/demo：Java SDK，Node SDK，Python SDK/demo，Go SDK/demo，Rust SDK/demo with `RUSTFLAGS=-D warnings`，Spring Boot 2/3/4 worker demos。
- Business smokes：notification provider e2e、management trigger e2e、SDK API key live smoke、migration CLI full-chain smoke。
- Deployment：`docker compose config`、server/web/docs Docker images build。

## 后续注意

- `.dev/reports/full-regression-20260625/` 和其他 `.dev/reports/*` 是本地证据目录，默认不提交。
- Live Slack/DingTalk/Feishu/WeCom/PagerDuty SaaS 验证仍需真实凭据；当前本地 provider/loopback 验证只证明 payload、redaction、retry/DLQ 和 trace 行为。
- 继续保持 Worker 主动出站连接边界，HTTP 业务响应 `{code,message,data}`，源码单文件 <=1500 行，入口文件保持薄。
- 任何后续 warning 必须修根因；禁止新增或保留 `#[allow]`、`#[expect]`、lint 降级或等价屏蔽。

## CI follow-up on 2026-06-25

- Remote CI run `28149289282` failed only in `Rust SDK + demo`; Coverage and other CI jobs were already green.
- Root cause: root workspace verification did not cover standalone `sdks/rust/tikeo` rustfmt/clippy gates, so SDK formatting drift and clippy warning debt reached CI.
- Fixed without `allow`/`expect` suppression and verified with the exact CI Rust SDK job command sequence, including `cargo package --manifest-path sdks/rust/tikeo/Cargo.toml --allow-dirty` because that is the existing release packaging command, not a lint suppression.
