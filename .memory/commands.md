# 命令记录

当前仓库已初始化 Rust workspace。

预期基础命令：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
```

若前端初始化在 `./web`：

```bash
cd web
bun install
bun run lint
bun run typecheck
bun test
bun run build
```


## 已验证命令（001-bootstrap）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/readyz
```


## 已验证命令（002-http-api-and-openapi）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/readyz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
curl -fsS http://0.0.0.0:9090/api/v1/system/info
curl -fsS http://0.0.0.0:9090/api/v1/cluster
curl -fsS http://0.0.0.0:9090/api/v1/jobs
curl -sS -o /tmp/create-job.json -w '%{http_code}' -H 'content-type: application/json' -d '{"name":"nightly"}' http://0.0.0.0:9090/api/v1/jobs
```


## HTTP 响应体契约检查

业务接口响应必须包含 `code`、`message`、`data`：

```bash
curl -fsS http://0.0.0.0:9090/api/v1/system/info
curl -fsS http://0.0.0.0:9090/api/v1/jobs
curl -sS -o /tmp/create-job.json -w '%{http_code}' -H 'content-type: application/json' -d '{"name":"nightly"}' http://0.0.0.0:9090/api/v1/jobs
```

Expected: success responses use `code=0`; failures use non-zero code; `data` key is always present.


## 已验证命令（003-worker-tunnel）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
# Smoke also verifies 0.0.0.0:9998 accepts TCP connection for Worker Tunnel gRPC listener.
```


## 已验证命令（004-storage-and-scheduler）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
curl -fsS http://0.0.0.0:9090/api/v1/jobs
curl -fsS -H 'content-type: application/json' -d '{"namespace":"default","app":"demo","name":"nightly","schedule_type":"api"}' http://0.0.0.0:9090/api/v1/jobs
curl -fsS http://0.0.0.0:9090/api/v1/jobs
```

说明：本阶段新增 SeaORM storage crate；SQLite dev DB 使用 `config/dev.toml` 的 `sqlite://scheduler-dev.db?mode=rwc`。


## 已验证命令（005-basic-scheduler）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
curl -fsS http://0.0.0.0:9090/api/v1/jobs
curl -fsS -H 'content-type: application/json' -d '{"namespace":"default","app":"demo","name":"manual-demo"}' http://0.0.0.0:9090/api/v1/jobs
curl -fsS -H 'content-type: application/json' -d '{"trigger_type":"api"}' http://0.0.0.0:9090/api/v1/jobs/<job_id>:trigger
curl -fsS http://0.0.0.0:9090/api/v1/jobs/<job_id>/instances
curl -fsS http://0.0.0.0:9090/api/v1/instances/<instance_id>
```


## 已验证命令（006-worker-sdk-rust-and-java-starter）

```bash
mvn -f sdks/java/pom.xml -q test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
```

说明：Rust Worker SDK 集成测试会启动内存 Worker Tunnel server，验证主动连接、注册与心跳 ping；Java SDK 当前验证 Maven 多模块编译测试。


## 已验证命令（007-web-ui-foundation）

```bash
cd web
bun install
bun run lint
bun run typecheck
bun test
bun run build

# 后端 / Java 回归仍需保持
mvn -f sdks/java/pom.xml -q test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
curl -fsS http://0.0.0.0:9090/api/v1/jobs
```

说明：Web build 当前有 Vite 大 chunk 警告（Ant Design bundle），不影响构建通过；后续可用路由级动态 import 拆包。


## 已验证命令（008-container-deployment）

```bash
docker compose config
docker build -t scheduler:dev .
docker build -t scheduler-web:dev ./web
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080/
curl -fsS http://0.0.0.0:8080/api/v1/jobs
docker compose down
python - <<'PY'
from pathlib import Path
import yaml
items = list(yaml.safe_load_all(Path('deploy/k8s/scheduler.yaml').read_text()))
assert all(item and item.get('apiVersion') and item.get('kind') for item in items)
print(f'k8s yaml documents: {len(items)}')
PY

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun install --cwd web
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
```

说明：`kubectl` 当前环境未安装，因此 K8s 做了 YAML 结构解析验证；Docker/Compose 已完成真实镜像构建与 Web -> backend API 代理冒烟。


## 已验证命令（009-worker-dispatch）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api/v1/jobs
mvn -f sdks/java/pom.xml -q test
bun install --cwd web
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t scheduler:dev .
docker build -t scheduler-web:dev ./web
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080/api/v1/jobs
docker compose down
```


## 已验证命令（010-scheduler-tick-loop）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS -H 'content-type: application/json' -d '{"namespace":"default","app":"demo","name":"fast","schedule_type":"fixed_rate","schedule_expr":"1s"}' http://0.0.0.0:9090/api/v1/jobs
curl -fsS http://0.0.0.0:9090/api/v1/jobs/<job_id>/instances
mvn -f sdks/java/pom.xml -q test
bun install --cwd web
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t scheduler:dev .
docker build -t scheduler-web:dev ./web
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080/api/v1/jobs
docker compose down
```


## 已验证命令（011-instance-logs）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t scheduler:dev .
docker build -t scheduler-web:dev ./web
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080/api/v1/jobs
docker compose down
```



## 2026-05-19 — 012-auth-rbac-foundation

```bash
cargo fmt --all
cargo test --workspace --all-features
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
```

完整最终验证已执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun install --cwd web
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t scheduler:dev .
docker build -t scheduler-web:dev ./web
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080
docker compose down
```

额外本地 server auth smoke：

```bash
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/api/v1/auth/login -H 'content-type: application/json' -d '{"username":"scheduler_init","password":"Scheduler@2026!"}'
TOKEN=$(curl -fsS http://0.0.0.0:9090/api/v1/auth/login -H 'content-type: application/json' -d '{"username":"scheduler_init","password":"Scheduler@2026!"}' | jq -r '.data.token')
curl -fsS http://0.0.0.0:9090/api/v1/auth/me -H "authorization: Bearer $TOKEN"
curl -fsS http://0.0.0.0:9090/api/v1/jobs -H 'content-type: application/json' -H "authorization: Bearer $TOKEN" -d '{"namespace":"default","app":"smoke","name":"auth-smoke"}'
```


## 2026-05-19 — 013-broadcast-execution / bridge container validation

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
DOCKER_BUILDKIT=1 docker build -t scheduler:dev .
DOCKER_BUILDKIT=1 docker build -t scheduler-web:dev ./web
docker compose down --remove-orphans || true
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080
curl -fsS http://0.0.0.0:8080/api/v1/system/info
curl -fsS http://0.0.0.0:8080/api-docs/openapi.json
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
docker compose ps
docker compose down
```


## 2026-05-19 — dev script / config directory validation

```bash
./scripts/dev.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
DOCKER_BUILDKIT=1 docker build -t scheduler:dev .
DOCKER_BUILDKIT=1 docker build -t scheduler-web:dev ./web
docker compose down --remove-orphans || true
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080
docker compose down
```


## 2026-05-19 — UI modernization / SQLite compatibility validation

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api/v1/jobs
curl -fsS http://0.0.0.0:9090/api/v1/jobs/job_019e3ec775b177b0bd1f804874c84f3c/instances
./scripts/dev.sh
```

## 已验证命令（015-user-management-and-rbac + SessionStore）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
cargo run --bin scheduler -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"scheduler_init","password":"Scheduler@2026!"}'
```

说明：登录冒烟验证返回 `atk_` opaque token；Vite build 仍提示 Ant Design 相关大 chunk 警告，不影响构建通过。

## 本轮新增检查（禁止外键 / users.password）

```bash
sqlite3 scheduler-dev.db "SELECT name, sql FROM sqlite_master WHERE type='table' AND sql LIKE '%REFERENCES%';"
sqlite3 scheduler-dev.db "PRAGMA table_info(users);"
```

Expected:
- 第一条无输出，表示当前 SQLite dev DB 无数据库级外键。
- `users` 表包含 `password` 列，不再包含 `password_hash`。

## 已验证命令（021 RBAC/service hardening 开发中）

```bash
cargo check --workspace --all-features
bun run --cwd web typecheck
```

## 已验证命令（021 RBAC/service hardening 完成）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
```

## 已验证命令（022 开发中）

```bash
cargo check --workspace --all-features
bun run --cwd web typecheck
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p scheduler-server workflow_create_validate_and_run_returns_envelopes --all-features
cargo test -p scheduler-server user_management_and_rbac_integration --all-features
```

## 已验证命令（022 Phase2 workflow foundation 完成）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
```

## 已验证命令（023 开发中）

```bash
cargo check --workspace --all-features
bun run --cwd web typecheck
```

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p scheduler-server workflow_create_validate_run_and_advance_returns_envelopes --all-features
bun run --cwd web lint
```

## 已验证命令（023 Phase2 workflow visual/mapreduce 完成）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
```

## 已验证命令（024 Phase2 distributed worker/recovery 完成）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
mvn -f sdks/java/pom.xml -q test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
```
