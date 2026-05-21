# 020-review-remediation：015-019 善后整改

## 背景

015-019 阶段已推进用户管理、动态脚本管理、脚本版本/diff、审计/告警/指标、前端路由。但 review 发现这些阶段存在“功能打勾多、平台级工程治理不足”的问题，需要专门善后。

## 必须整改的问题清单

### 安全阻断

1. **静态 Admin Bearer 后门**：`scheduler-init-token` 可绕过登录成为 admin。必须删除，仅允许初始化账号通过登录获取 session token。
2. **审计日志记录明文 token**：login/logout 不得将 Bearer token 明文写入 `audit_logs.resource_id`，必须脱敏或存不可逆摘要。
3. **Webhook SSRF 风险**：告警 webhook 必须限制 HTTPS、拒绝 localhost/私网/IP metadata，并设置超时。

### 脚本版本语义

4. 创建脚本必须生成初始版本。
5. 更新脚本必须在事务中写入更新后的不可变版本快照，避免“历史版本滞后一拍”。
6. `script_id + version_number` 必须唯一；diff 应通过 DB 精确查询版本，不应全量 list 后内存 find。
7. diff 不应使用过度简化算法；至少要输出标准头与 hunk，并使用 LCS 类算法保证重复行/移动附近变更更稳定。

### 可观测性/审计

8. `/metrics` 不能只是空端点；至少接入 HTTP request count/latency、Worker 连接数、dispatch 计数。
9. 审计写入失败不能静默吞掉，至少 warn 记录。
10. audit list 的分页/过滤不能假装支持；当前阶段至少在设计中明确后续补齐。

### 前端与质量门禁

11. `cargo fmt --check`、`bun lint` 必须通过后才能提交。
12. 修复 Web 未使用 import/变量。
13. 后续需要拆分 `ScriptsPage`、路由 meta、统一 query/error 层，但本阶段先记录为后续治理项。

## 本阶段交付

- 修复安全阻断项。
- 修复脚本版本核心语义。
- 补最低可观测性指标。
- 修复 fmt/lint/typecheck/test/build 门禁。
- 更新 `.memory/*` 与 `design/scheduler-architecture-design.md` 路线图，避免把“骨架完成”误标成“平台能力完成”。

## 验证要求

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
./sdks/java/gradlew -p sdks/java test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
```
