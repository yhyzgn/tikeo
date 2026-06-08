# 命令记录

当前仓库已初始化 Rust workspace。

预期基础命令：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin tikeo -- serve --config config/dev.toml
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
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/readyz
```


## 已验证命令（002-http-api-and-openapi）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin tikeo -- serve --config config/dev.toml
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
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
# Smoke also verifies 0.0.0.0:9998 accepts TCP connection for Worker Tunnel gRPC listener.
```


## 已验证命令（004-storage-and-tikeo）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
curl -fsS http://0.0.0.0:9090/api/v1/jobs
curl -fsS -H 'content-type: application/json' -d '{"namespace":"default","app":"demo","name":"nightly","schedule_type":"api"}' http://0.0.0.0:9090/api/v1/jobs
curl -fsS http://0.0.0.0:9090/api/v1/jobs
```

说明：本阶段新增 SeaORM storage crate；SQLite dev DB 使用 `config/dev.toml` 的 `sqlite://tikeo-dev.db?mode=rwc`。


## 已验证命令（005-basic-tikeo）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin tikeo -- serve --config config/dev.toml
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
./sdks/java/gradlew -p sdks/java test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
```

说明：Rust Worker SDK 集成测试会启动内存 Worker Tunnel server，验证主动连接、注册与心跳 ping；Java SDK 当前验证 Gradle 多模块编译测试。


## 已验证命令（007-web-ui-foundation）

```bash
cd web
bun install
bun run lint
bun run typecheck
bun test
bun run build

# 后端 / Java 回归仍需保持
./sdks/java/gradlew -p sdks/java test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api-docs/openapi.json
curl -fsS http://0.0.0.0:9090/api/v1/jobs
```

说明：Web build 当前有 Vite 大 chunk 警告（Ant Design bundle），不影响构建通过；后续可用路由级动态 import 拆包。


## 已验证命令（008-container-deployment）

```bash
docker compose config
docker build -t tikeo:dev .
docker build -t tikeo-web:dev ./web
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080/
curl -fsS http://0.0.0.0:8080/api/v1/jobs
docker compose down
python - <<'PY'
from pathlib import Path
import yaml
items = list(yaml.safe_load_all(Path('deploy/k8s/tikeo.yaml').read_text()))
assert all(item and item.get('apiVersion') and item.get('kind') for item in items)
print(f'k8s yaml documents: {len(items)}')
PY

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
./sdks/java/gradlew -p sdks/java test
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
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api/v1/jobs
./sdks/java/gradlew -p sdks/java test
bun install --cwd web
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t tikeo:dev .
docker build -t tikeo-web:dev ./web
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080/api/v1/jobs
docker compose down
```


## 已验证命令（010-tikeo-tick-loop）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS -H 'content-type: application/json' -d '{"namespace":"default","app":"demo","name":"fast","schedule_type":"fixed_rate","schedule_expr":"1s"}' http://0.0.0.0:9090/api/v1/jobs
curl -fsS http://0.0.0.0:9090/api/v1/jobs/<job_id>/instances
./sdks/java/gradlew -p sdks/java test
bun install --cwd web
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t tikeo:dev .
docker build -t tikeo-web:dev ./web
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
./sdks/java/gradlew -p sdks/java test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t tikeo:dev .
docker build -t tikeo-web:dev ./web
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
./sdks/java/gradlew -p sdks/java test
bun install --cwd web
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
docker build -t tikeo:dev .
docker build -t tikeo-web:dev ./web
docker compose up -d --no-build
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:8080
docker compose down
```

额外本地 server auth smoke：

```bash
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/api/v1/auth/login -H 'content-type: application/json' -d '{"username":"tikeo_init","password":"Tikeo@2026!"}'
TOKEN=$(curl -fsS http://0.0.0.0:9090/api/v1/auth/login -H 'content-type: application/json' -d '{"username":"tikeo_init","password":"Tikeo@2026!"}' | jq -r '.data.token')
curl -fsS http://0.0.0.0:9090/api/v1/auth/me -H "authorization: Bearer $TOKEN"
curl -fsS http://0.0.0.0:9090/api/v1/jobs -H 'content-type: application/json' -H "authorization: Bearer $TOKEN" -d '{"namespace":"default","app":"smoke","name":"auth-smoke"}'
```


## 2026-05-19 — 013-broadcast-execution / bridge container validation

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
DOCKER_BUILDKIT=1 docker build -t tikeo:dev .
DOCKER_BUILDKIT=1 docker build -t tikeo-web:dev ./web
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
./sdks/java/gradlew -p sdks/java test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
DOCKER_BUILDKIT=1 docker build -t tikeo:dev .
DOCKER_BUILDKIT=1 docker build -t tikeo-web:dev ./web
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
./sdks/java/gradlew -p sdks/java test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
cargo run --bin tikeo -- serve --config config/dev.toml
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
./sdks/java/gradlew -p sdks/java test
bun run --cwd web lint
bun run --cwd web typecheck
bun test --cwd web
bun run --cwd web build
docker compose config
cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
curl -fsS http://0.0.0.0:9090/api/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"tikeo_init","password":"Tikeo@2026!"}'
```

说明：登录冒烟验证返回 `atk_` opaque token；Vite build 仍提示 Ant Design 相关大 chunk 警告，不影响构建通过。

## 本轮新增检查（禁止外键 / users.password）

```bash
sqlite3 tikeo-dev.db "SELECT name, sql FROM sqlite_master WHERE type='table' AND sql LIKE '%REFERENCES%';"
sqlite3 tikeo-dev.db "PRAGMA table_info(users);"
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
./sdks/java/gradlew -p sdks/java test
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
cargo test -p tikeo-server workflow_create_validate_and_run_returns_envelopes --all-features
cargo test -p tikeo-server user_management_and_rbac_integration --all-features
```

## 已验证命令（022 Phase2 workflow foundation 完成）

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

## 已验证命令（023 开发中）

```bash
cargo check --workspace --all-features
bun run --cwd web typecheck
```

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p tikeo-server workflow_create_validate_run_and_advance_returns_envelopes --all-features
bun run --cwd web lint
```

## 已验证命令（023 Phase2 workflow visual/mapreduce 完成）

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

## 已验证命令（024 Phase2 distributed worker/recovery 完成）

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

### SDK layout correction verification (2026-05-21)
```bash
./sdks/java/gradlew -p sdks/java test
./sdks/java/gradlew -p examples/java/spring-worker-demo test
cargo fmt --all -- --check
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
bun run --cwd web lint && bun run --cwd web typecheck && bun test --cwd web && bun run --cwd web build
DOCKER_BUILDKIT=1 docker build -t tikeo:dev .
```

### Rust SDK independent publishing cleanup verification (2026-05-21)
```bash
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --all-features
cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run --manifest-path examples/rust/worker-demo/Cargo.toml
DOCKER_BUILDKIT=1 docker build -t tikeo:dev .
cargo package --manifest-path sdks/rust/tikeo/Cargo.toml --allow-dirty
```

## tikeo rename verification commands (2026-05-22)

```bash
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo run -- --help
cd web && bun run typecheck && bun test && bun run build
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml
cargo test --manifest-path sdks/rust/tikeo/Cargo.toml --features wasm
cargo clippy --manifest-path sdks/rust/tikeo/Cargo.toml --all-targets --all-features -- -D warnings
cd sdks/java && ./gradlew test --warning-mode all --no-daemon
```

## 2026-06-04 Worker/SDK parity verification baseline

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- --test-threads=1
cargo build --workspace --all-features
cd web && bun install --frozen-lockfile && bun run lint && bun run typecheck && bun test && bun run build
cd sdks/java && ./gradlew test jar sourcesJar
cd sdks/go/tikeo && go test ./...
cd examples/go/worker-demo && go test ./...
cd sdks/rust/tikeo && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features && cargo package --allow-dirty
cd examples/rust/worker-demo && cargo test
```

GitHub Actions CI evidence: run `26947829951` success.

## 2026-06-08 coverage badge / README logo verification

```bash
python3 scripts/verify-github-actions-node-runtime.py --min-node-major 24
python3 - <<'PY'
from pathlib import Path
import yaml
assert 'jobs' in yaml.safe_load(Path('.github/workflows/coverage.yml').read_text())
for readme in [Path('README.md'), Path('README.zh-CN.md')]:
    text = readme.read_text()
    assert 'docs/assets/tikeo-logo-breathe.gif' in text
    assert 'https://codecov.io/gh/yhyzgn/tikeo/branch/main/graph/badge.svg"' in text
    assert 'flag=rust' not in text
    assert text.index('alt="Java 17+"') < text.index('alt="Java core SDK"') < text.index('alt="Rust SDK"')
PY
file docs/assets/tikeo-logo-breathe.gif
git diff --check

cd web && bun test src --coverage --coverage-reporter=lcov --coverage-dir=../coverage/web
cd sdks/nodejs/tikeo && bun test --coverage --coverage-reporter=lcov --coverage-dir=../../../coverage/nodejs-sdk
cd sdks/go/tikeo && go test ./... -covermode=atomic -coverprofile=../../../coverage/go-sdk.out -count=1
cd sdks/java && ./gradlew test jacocoTestReport --no-daemon
uv venv --clear .dev/python-coverage-venv
. .dev/python-coverage-venv/bin/activate
uv pip install pytest-cov -e sdks/python/tikeo[test] -e examples/python/worker-demo[test]
python -m pytest sdks/python/tikeo/tests examples/python/worker-demo/tests \
  --cov=tikeo --cov=tikeo_python_worker_demo --cov-report=xml:coverage/python.xml -q
```
