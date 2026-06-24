-- Development integration seed data for tikeo.
--
-- Usage:
--   1. Start the dev server once so migrations create the schema:
--      ./scripts/dev.sh
--   2. Apply this script to the SQLite dev database:
--      sqlite3 .dev/tikeo-dev.db < scripts/dev-seed.sql
--
-- The SQL is upsert-based for explicit refreshes. Prefer scripts/dev-seed.sh, which leaves existing local seed rows unchanged unless --refresh or TIKEO_DEV_SEED_REFRESH=1 is set.
-- It is intended for local development only, not production bootstrapping.

BEGIN TRANSACTION;

-- Local demo topology. Keep this aligned with scripts/dev-integration-seed.sh,
-- scripts/start-java-demo-workers.sh, and language worker demo defaults.
INSERT INTO namespaces (id, name, created_at, updated_at)
VALUES
  ('ns-dev-default', 'dev-default', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('ns-dev-alpha', 'dev-alpha', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('ns-dev-beta', 'dev-beta', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('ns-dev-ops', 'dev-ops', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  name = excluded.name,
  updated_at = excluded.updated_at;

INSERT INTO apps (id, namespace_id, name, created_at, updated_at)
VALUES
  ('app-dev-default', 'ns-dev-default', 'dev-default', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('app-dev-observability', 'ns-dev-default', 'observability-demo', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('app-dev-alpha-orders', 'ns-dev-alpha', 'orders', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('app-dev-alpha-billing', 'ns-dev-alpha', 'billing', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('app-dev-beta-analytics', 'ns-dev-beta', 'analytics', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('app-dev-ops-automation', 'ns-dev-ops', 'automation', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  namespace_id = excluded.namespace_id,
  name = excluded.name,
  updated_at = excluded.updated_at;

INSERT INTO worker_pools (id, namespace_id, app_id, name, max_queue_depth, max_concurrency, created_at, updated_at)
VALUES
  ('wp-dev-alpha-orders-boot2-blue', 'ns-dev-alpha', 'app-dev-alpha-orders', 'boot2-blue', 200, 8, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('wp-dev-alpha-orders-boot3-blue', 'ns-dev-alpha', 'app-dev-alpha-orders', 'boot3-blue', 200, 8, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('wp-dev-alpha-orders-go-blue', 'ns-dev-alpha', 'app-dev-alpha-orders', 'go-blue', 200, 8, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('wp-dev-alpha-orders-rust-blue', 'ns-dev-alpha', 'app-dev-alpha-orders', 'rust-blue', 200, 8, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('wp-dev-alpha-orders-python-blue', 'ns-dev-alpha', 'app-dev-alpha-orders', 'python-blue', 200, 8, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('wp-dev-alpha-orders-nodejs-blue', 'ns-dev-alpha', 'app-dev-alpha-orders', 'nodejs-blue', 200, 8, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('wp-dev-alpha-billing-boot4-green', 'ns-dev-alpha', 'app-dev-alpha-billing', 'boot4-green', 100, 4, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('wp-dev-beta-analytics-boot3-batch', 'ns-dev-beta', 'app-dev-beta-analytics', 'boot3-batch', 150, 6, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('wp-dev-ops-automation-boot4-ops', 'ns-dev-ops', 'app-dev-ops-automation', 'boot4-ops', 80, 3, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  namespace_id = excluded.namespace_id,
  app_id = excluded.app_id,
  name = excluded.name,
  max_queue_depth = excluded.max_queue_depth,
  max_concurrency = excluded.max_concurrency,
  updated_at = excluded.updated_at;

INSERT INTO users (id, username, email, password, role, bootstrap_admin, created_at)
VALUES
  ('usr-dev-operator', 'dev_operator', 'dev.operator@example.com', '$2b$10$vslUa5GAP.Mk3s4PPclu..miTj/beUTaSCR/HSZdfPVXmhA/7lmpm', 'operator', 0, '2026-01-01T00:00:00Z'),
  ('usr-dev-viewer', 'dev_viewer', 'dev.viewer@example.com', '$2b$10$vslUa5GAP.Mk3s4PPclu..miTj/beUTaSCR/HSZdfPVXmhA/7lmpm', 'viewer', 0, '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  username = excluded.username,
  email = excluded.email,
  password = excluded.password,
  role = excluded.role,
  bootstrap_admin = excluded.bootstrap_admin;

INSERT INTO user_roles (id, user_id, role_id, created_at)
SELECT 'ur-' || users.id || '-' || roles.id, users.id, roles.id, '2026-01-01T00:00:00Z'
FROM users
JOIN roles ON roles.name = users.role
WHERE users.id IN ('usr-dev-operator', 'usr-dev-viewer')
ON CONFLICT(id) DO UPDATE SET
  user_id = excluded.user_id,
  role_id = excluded.role_id;

INSERT INTO jobs (id, namespace_id, app_id, name, schedule_type, schedule_expr, processor_name, misfire_policy, enabled, canary_percent, canary_policy_json, retry_policy_json, created_at, updated_at)
VALUES
  ('job-dev-api-hello', 'ns-dev-alpha', 'app-dev-alpha-orders', 'api-hello', 'api', NULL, 'demo.echo', 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('job-dev-fixed-rate-heartbeat', 'ns-dev-alpha', 'app-dev-alpha-orders', 'fixed-rate-heartbeat', 'fixed_rate', '30s', 'demo.heartbeat', 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('job-dev-cron-minute-report', 'ns-dev-alpha', 'app-dev-alpha-orders', 'cron-minute-report', 'cron', '0/30 * * * * * *', 'demo.report', 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  namespace_id = excluded.namespace_id,
  app_id = excluded.app_id,
  name = excluded.name,
  schedule_type = excluded.schedule_type,
  schedule_expr = excluded.schedule_expr,
  processor_name = excluded.processor_name,
  misfire_policy = excluded.misfire_policy,
  enabled = excluded.enabled,
  canary_percent = excluded.canary_percent,
  canary_policy_json = excluded.canary_policy_json,
  retry_policy_json = excluded.retry_policy_json,
  updated_at = excluded.updated_at;

INSERT INTO job_instances (id, job_id, status, trigger_type, execution_mode, created_at, updated_at)
VALUES
  ('inst-dev-api-hello-pending', 'job-dev-api-hello', 'pending', 'api', 'single', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('inst-dev-heartbeat-succeeded', 'job-dev-fixed-rate-heartbeat', 'succeeded', 'fixed_rate', 'single', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  job_id = excluded.job_id,
  status = excluded.status,
  trigger_type = excluded.trigger_type,
  execution_mode = excluded.execution_mode,
  updated_at = excluded.updated_at;

INSERT INTO dispatch_queue (id, job_instance_id, workflow_node_instance_id, priority, run_after, status, attempt, lease_owner, lease_until, fencing_token, worker_selector, created_at, updated_at)
VALUES ('queue-dev-api-hello', 'inst-dev-api-hello-pending', NULL, 100, '2026-01-01T00:00:00Z', 'pending', 0, NULL, NULL, NULL, 'demo.echo', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  job_instance_id = excluded.job_instance_id,
  workflow_node_instance_id = excluded.workflow_node_instance_id,
  priority = excluded.priority,
  run_after = excluded.run_after,
  status = excluded.status,
  attempt = excluded.attempt,
  lease_owner = excluded.lease_owner,
  lease_until = excluded.lease_until,
  fencing_token = excluded.fencing_token,
  worker_selector = excluded.worker_selector,
  updated_at = excluded.updated_at;

INSERT INTO job_instance_attempts (id, instance_id, worker_id, status, created_at, updated_at)
VALUES ('attempt-dev-heartbeat-1', 'inst-dev-heartbeat-succeeded', 'worker-dev-1', 'succeeded', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  instance_id = excluded.instance_id,
  worker_id = excluded.worker_id,
  status = excluded.status,
  updated_at = excluded.updated_at;

INSERT INTO job_instance_logs (id, instance_id, worker_id, level, message, sequence, created_at)
VALUES
  ('log-dev-heartbeat-1', 'inst-dev-heartbeat-succeeded', 'worker-dev-1', 'info', 'dev heartbeat started', 1, '2026-01-01T00:00:01Z'),
  ('log-dev-heartbeat-2', 'inst-dev-heartbeat-succeeded', 'worker-dev-1', 'info', 'dev heartbeat completed', 2, '2026-01-01T00:00:02Z')
ON CONFLICT(id) DO UPDATE SET
  instance_id = excluded.instance_id,
  worker_id = excluded.worker_id,
  level = excluded.level,
  message = excluded.message,
  sequence = excluded.sequence,
  created_at = excluded.created_at;

INSERT INTO scripts (id, name, language, version, content, status, released_version_id, released_version_number, timeout_seconds, max_memory_bytes, allow_network, allowed_env_vars, policy_json, created_by, created_at, updated_at)
VALUES
  ('script-dev-shell-hello', 'dev-shell-hello', 'shell', '1.0.0', '#!/usr/bin/env sh
echo "hello from tikeo dev seed: ${TIKEO_DEV_MESSAGE:-ok}"
', 'approved', 'script-version-dev-shell-hello-1', 1, 10, 67108864, 0, '["TIKEO_DEV_MESSAGE"]', '{"network":{"enabled":false},"filesystem":{"read_only_paths":[],"writable_paths":[]},"resources":{"timeout_ms":10000,"max_memory_bytes":67108864},"env_vars":["TIKEO_DEV_MESSAGE"]}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('script-dev-python-json', 'dev-python-json', 'python', '1.0.0', 'import json
print(json.dumps({"status":"ok","source":"tikeo-dev-seed"}))
', 'approved', 'script-version-dev-python-json-1', 1, 10, 67108864, 0, '[]', '{"network":{"enabled":false},"filesystem":{"read_only_paths":[],"writable_paths":[]},"resources":{"timeout_ms":10000,"max_memory_bytes":67108864},"env_vars":[]}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  name = excluded.name,
  language = excluded.language,
  version = excluded.version,
  content = excluded.content,
  status = excluded.status,
  released_version_id = excluded.released_version_id,
  released_version_number = excluded.released_version_number,
  timeout_seconds = excluded.timeout_seconds,
  max_memory_bytes = excluded.max_memory_bytes,
  allow_network = excluded.allow_network,
  allowed_env_vars = excluded.allowed_env_vars,
  policy_json = excluded.policy_json,
  created_by = excluded.created_by,
  updated_at = excluded.updated_at;

INSERT INTO script_versions (id, script_id, version_number, content, content_sha256, language, status, timeout_seconds, max_memory_bytes, allow_network, allowed_env_vars, policy_json, created_by, created_at)
VALUES
  ('script-version-dev-shell-hello-1', 'script-dev-shell-hello', 1, '#!/usr/bin/env sh
echo "hello from tikeo dev seed: ${TIKEO_DEV_MESSAGE:-ok}"
', '991474538b28fa818388441d7fb96c51ecc3914914bda96f2d3cf480003ada31', 'shell', 'approved', 10, 67108864, 0, '["TIKEO_DEV_MESSAGE"]', '{"network":{"enabled":false},"filesystem":{"read_only_paths":[],"writable_paths":[]},"resources":{"timeout_ms":10000,"max_memory_bytes":67108864},"env_vars":["TIKEO_DEV_MESSAGE"]}', 'usr-admin', '2026-01-01T00:00:00Z'),
  ('script-version-dev-python-json-1', 'script-dev-python-json', 1, 'import json
print(json.dumps({"status":"ok","source":"tikeo-dev-seed"}))
', '08b4ae890c6e0ad4d9ea6d0886ce65478c60ddcc1cefd0ab76f58ba1b3746f09', 'python', 'approved', 10, 67108864, 0, '[]', '{"network":{"enabled":false},"filesystem":{"read_only_paths":[],"writable_paths":[]},"resources":{"timeout_ms":10000,"max_memory_bytes":67108864},"env_vars":[]}', 'usr-admin', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  script_id = excluded.script_id,
  version_number = excluded.version_number,
  content = excluded.content,
  content_sha256 = excluded.content_sha256,
  language = excluded.language,
  status = excluded.status,
  timeout_seconds = excluded.timeout_seconds,
  max_memory_bytes = excluded.max_memory_bytes,
  allow_network = excluded.allow_network,
  allowed_env_vars = excluded.allowed_env_vars,
  policy_json = excluded.policy_json,
  created_by = excluded.created_by,
  created_at = excluded.created_at;


-- Script language examples for local UI/API validation. These mirror the Web create/edit language enum.
INSERT INTO scripts (id, name, language, version, content, status, released_version_id, released_version_number, timeout_seconds, max_memory_bytes, allow_network, allowed_env_vars, policy_json, created_by, created_at, updated_at)
VALUES
  ('script-dev-shell-example', 'dev-shell-script-example', 'shell', '1.0.0', '#!/usr/bin/env sh
set -eu
echo "tikeo shell script example ok"
', 'approved', 'script-version-dev-shell-example-1', 1, 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('script-dev-python-example', 'dev-python-script-example', 'python', '1.0.0', 'import json
print(json.dumps({"language": "python", "status": "ok"}))
', 'approved', 'script-version-dev-python-example-1', 1, 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('script-dev-javascript-example', 'dev-javascript-script-example', 'javascript', '1.0.0', 'const result = { language: "javascript", status: "ok" };
console.log(JSON.stringify(result));
', 'approved', 'script-version-dev-javascript-example-1', 1, 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('script-dev-typescript-example', 'dev-typescript-script-example', 'typescript', '1.0.0', 'type Result = { language: string; status: string };
const result: Result = { language: "typescript", status: "ok" };
console.log(JSON.stringify(result));
', 'approved', 'script-version-dev-typescript-example-1', 1, 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('script-dev-powershell-example', 'dev-powershell-script-example', 'powershell', '1.0.0', '$result = @{ language = "powershell"; status = "ok" } | ConvertTo-Json -Compress
Write-Output $result
', 'approved', 'script-version-dev-powershell-example-1', 1, 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('script-dev-rhai-example', 'dev-rhai-script-example', 'rhai', '1.0.0', 'let result = "rhai script example ok";
print(result);
', 'approved', 'script-version-dev-rhai-example-1', 1, 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('script-dev-rhai-object-example', 'dev-rhai-object-script-example', 'rhai', '1.0.0', 'let result = #{ language: "rhai", status: "ok", "case": "manual-acceptance" };
print(result);
', 'approved', 'script-version-dev-rhai-object-example-1', 1, 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  name = excluded.name,
  language = excluded.language,
  version = excluded.version,
  content = excluded.content,
  status = excluded.status,
  released_version_id = excluded.released_version_id,
  released_version_number = excluded.released_version_number,
  timeout_seconds = excluded.timeout_seconds,
  max_memory_bytes = excluded.max_memory_bytes,
  allow_network = excluded.allow_network,
  allowed_env_vars = excluded.allowed_env_vars,
  policy_json = excluded.policy_json,
  created_by = excluded.created_by,
  updated_at = excluded.updated_at;

INSERT INTO script_versions (id, script_id, version_number, content, content_sha256, language, status, timeout_seconds, max_memory_bytes, allow_network, allowed_env_vars, policy_json, created_by, created_at)
VALUES
  ('script-version-dev-shell-example-1', 'script-dev-shell-example', 1, '#!/usr/bin/env sh
set -eu
echo "tikeo shell script example ok"
', '127d5da7417c18bf3d9567da168de6718bef6c78edbe00c6c50efafd4e27f845', 'shell', 'approved', 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z'),
  ('script-version-dev-python-example-1', 'script-dev-python-example', 1, 'import json
print(json.dumps({"language": "python", "status": "ok"}))
', '0c528eb311f36c396eb445a4cb5bd6a2f9266d706498d128c5a08eba03995b68', 'python', 'approved', 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z'),
  ('script-version-dev-javascript-example-1', 'script-dev-javascript-example', 1, 'const result = { language: "javascript", status: "ok" };
console.log(JSON.stringify(result));
', '128b0ec123c1626fa72e31aa0f2f6b3b59163070005e8a191e5033cbfe30393f', 'javascript', 'approved', 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z'),
  ('script-version-dev-typescript-example-1', 'script-dev-typescript-example', 1, 'type Result = { language: string; status: string };
const result: Result = { language: "typescript", status: "ok" };
console.log(JSON.stringify(result));
', '3bc42bcfb49053fb4004bb655d17ce7854c3b83ecb016f790f368ade5567f8ea', 'typescript', 'approved', 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z'),
  ('script-version-dev-powershell-example-1', 'script-dev-powershell-example', 1, '$result = @{ language = "powershell"; status = "ok" } | ConvertTo-Json -Compress
Write-Output $result
', '737738c0056b58812b5f2849766b0d00f526d35a1c07f5c5fc6b802f56c06097', 'powershell', 'approved', 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z'),
  ('script-version-dev-rhai-example-1', 'script-dev-rhai-example', 1, 'let result = "rhai script example ok";
print(result);
', 'c5fbea054ea991344bda993da92f06498e28822e57f8e6c51c77d694f1d4b6c9', 'rhai', 'approved', 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z'),
  ('script-version-dev-rhai-object-example-1', 'script-dev-rhai-object-example', 1, 'let result = #{ language: "rhai", status: "ok", "case": "manual-acceptance" };
print(result);
', '146ca4d31bdd3bdb2cf87ef600ee7fec70f839c081fbca0cd621eac290169306', 'rhai', 'approved', 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  script_id = excluded.script_id,
  version_number = excluded.version_number,
  content = excluded.content,
  content_sha256 = excluded.content_sha256,
  language = excluded.language,
  status = excluded.status,
  timeout_seconds = excluded.timeout_seconds,
  max_memory_bytes = excluded.max_memory_bytes,
  allow_network = excluded.allow_network,
  allowed_env_vars = excluded.allowed_env_vars,
  policy_json = excluded.policy_json,
  created_by = excluded.created_by,
  created_at = excluded.created_at;

INSERT INTO jobs (id, namespace_id, app_id, name, schedule_type, schedule_expr, processor_name, misfire_policy, enabled, canary_percent, canary_policy_json, retry_policy_json, created_at, updated_at, script_id)
VALUES
  ('job-dev-script-shell-example', 'ns-dev-alpha', 'app-dev-alpha-orders', 'dev-shell-script-job', 'api', NULL, NULL, 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-shell-example'),
  ('job-dev-script-python-example', 'ns-dev-alpha', 'app-dev-alpha-orders', 'dev-python-script-job', 'api', NULL, NULL, 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-python-example'),
  ('job-dev-script-javascript-example', 'ns-dev-alpha', 'app-dev-alpha-orders', 'dev-javascript-script-job', 'api', NULL, NULL, 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-javascript-example'),
  ('job-dev-script-typescript-example', 'ns-dev-alpha', 'app-dev-alpha-orders', 'dev-typescript-script-job', 'api', NULL, NULL, 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-typescript-example'),
  ('job-dev-script-powershell-example', 'ns-dev-alpha', 'app-dev-alpha-orders', 'dev-powershell-script-job', 'api', NULL, NULL, 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-powershell-example'),
  ('job-dev-script-rhai-example', 'ns-dev-alpha', 'app-dev-alpha-orders', 'dev-rhai-script-job', 'api', NULL, NULL, 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-rhai-example'),
  ('job-dev-script-rhai-object-example', 'ns-dev-alpha', 'app-dev-alpha-orders', 'dev-rhai-object-script-job', 'api', NULL, NULL, 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-rhai-object-example')
ON CONFLICT(id) DO UPDATE SET
  namespace_id = excluded.namespace_id,
  app_id = excluded.app_id,
  name = excluded.name,
  schedule_type = excluded.schedule_type,
  schedule_expr = excluded.schedule_expr,
  processor_name = excluded.processor_name,
  misfire_policy = excluded.misfire_policy,
  enabled = excluded.enabled,
  canary_percent = excluded.canary_percent,
  canary_policy_json = excluded.canary_policy_json,
  retry_policy_json = excluded.retry_policy_json,
  script_id = excluded.script_id,
  updated_at = excluded.updated_at;

INSERT INTO workflows (id, name, definition, status, created_by, created_at, updated_at)
VALUES ('wf-dev-basic-pipeline', 'dev-basic-pipeline', '{"nodes":[{"key":"hello","name":"API hello","kind":"job","job_id":"job-dev-api-hello","processor_name":null,"child_workflow_id":null,"map_items":null,"config":null},{"key":"report","name":"Minute report","kind":"job","job_id":"job-dev-cron-minute-report","processor_name":null,"child_workflow_id":null,"map_items":null,"config":null}],"edges":[{"from":"hello","to":"report","condition":"on_success"}]}', 'active', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  name = excluded.name,
  definition = excluded.definition,
  status = excluded.status,
  created_by = excluded.created_by,
  updated_at = excluded.updated_at;

DELETE FROM workflow_nodes
WHERE workflow_id = 'wf-dev-basic-pipeline'
  AND node_key IN ('hello', 'report')
  AND id NOT IN ('wfn-dev-basic-hello', 'wfn-dev-basic-report');

DELETE FROM workflow_edges
WHERE workflow_id = 'wf-dev-basic-pipeline'
  AND from_node_key = 'hello'
  AND to_node_key = 'report'
  AND id <> 'wfe-dev-basic-hello-report';

INSERT INTO workflow_nodes (id, workflow_id, node_key, name, kind, job_id, processor_name, config, created_at)
VALUES
  ('wfn-dev-basic-hello', 'wf-dev-basic-pipeline', 'hello', 'API hello', 'job', 'job-dev-api-hello', NULL, NULL, '2026-01-01T00:00:00Z'),
  ('wfn-dev-basic-report', 'wf-dev-basic-pipeline', 'report', 'Minute report', 'job', 'job-dev-cron-minute-report', NULL, NULL, '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  workflow_id = excluded.workflow_id,
  node_key = excluded.node_key,
  name = excluded.name,
  kind = excluded.kind,
  job_id = excluded.job_id,
  processor_name = excluded.processor_name,
  config = excluded.config,
  created_at = excluded.created_at;

INSERT INTO workflow_edges (id, workflow_id, from_node_key, to_node_key, condition, created_at)
VALUES ('wfe-dev-basic-hello-report', 'wf-dev-basic-pipeline', 'hello', 'report', 'on_success', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  workflow_id = excluded.workflow_id,
  from_node_key = excluded.from_node_key,
  to_node_key = excluded.to_node_key,
  condition = excluded.condition,
  created_at = excluded.created_at;


-- Notification Center failure/success/status demo data for card rendering and public console passthrough.
-- These rows are intentionally persisted seed data, not provider metadata: after applying the seed,
-- the Notification Center page and /public/instances/{id}/console can be opened immediately.
INSERT INTO jobs (id, namespace_id, app_id, name, schedule_type, schedule_expr, processor_name, misfire_policy, enabled, canary_percent, canary_policy_json, retry_policy_json, created_at, updated_at)
VALUES
  ('job-dev-notify-exception', 'ns-dev-alpha', 'app-dev-alpha-orders', 'AutoGenerateStockPdfRecordAfterDateTask', 'api', NULL, 'demo.exception', 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-06-11T17:35:17+08:00', '2026-06-11T17:35:57+08:00'),
  ('job-dev-notify-success', 'ns-dev-alpha', 'app-dev-alpha-orders', 'RebuildCustomerStatementTask', 'api', NULL, 'demo.echo', 'ignore', 1, 0, '{"metricsGateEnabled":false,"minimumSamples":5,"evaluationWindow":20,"maxFailureRate":0.5,"autoRollback":true}', '{"enabled":true,"maxAttempts":3,"initialDelaySeconds":5,"backoffMultiplier":2,"maxDelaySeconds":60}', '2026-06-11T17:40:00+08:00', '2026-06-11T17:40:12+08:00')
ON CONFLICT(id) DO UPDATE SET
  namespace_id = excluded.namespace_id,
  app_id = excluded.app_id,
  name = excluded.name,
  schedule_type = excluded.schedule_type,
  schedule_expr = excluded.schedule_expr,
  processor_name = excluded.processor_name,
  misfire_policy = excluded.misfire_policy,
  enabled = excluded.enabled,
  canary_percent = excluded.canary_percent,
  canary_policy_json = excluded.canary_policy_json,
  retry_policy_json = excluded.retry_policy_json,
  updated_at = excluded.updated_at;

INSERT INTO job_instances (id, job_id, status, trigger_type, execution_mode, result_worker_id, result_success, result_message, result_completed_at, created_at, updated_at)
VALUES
  ('inst-dev-notify-feishu-failed', 'job-dev-notify-exception', 'failed', 'api', 'single', '172.16.103.25:9999', 0, '参数不能为空 should not be empty', '2026-06-11T17:35:57+08:00', '2026-06-11T17:35:17+08:00', '2026-06-11T17:35:57+08:00'),
  ('inst-dev-notify-feishu-succeeded', 'job-dev-notify-success', 'succeeded', 'api', 'single', '172.16.103.26:9999', 1, 'statement rebuilt successfully', '2026-06-11T17:40:12+08:00', '2026-06-11T17:40:00+08:00', '2026-06-11T17:40:12+08:00'),
  ('inst-dev-notify-feishu-running', 'job-dev-notify-success', 'succeeded', 'api', 'single', '172.16.103.27:9999', 1, 'statement rebuilt after progress notification', '2026-06-11T17:46:12+08:00', '2026-06-11T17:45:00+08:00', '2026-06-11T17:46:12+08:00')
ON CONFLICT(id) DO UPDATE SET
  job_id = excluded.job_id,
  status = excluded.status,
  trigger_type = excluded.trigger_type,
  execution_mode = excluded.execution_mode,
  result_worker_id = excluded.result_worker_id,
  result_success = excluded.result_success,
  result_message = excluded.result_message,
  result_completed_at = excluded.result_completed_at,
  created_at = excluded.created_at,
  updated_at = excluded.updated_at;

INSERT INTO job_instance_attempts (id, instance_id, worker_id, status, result_success, result_message, result_completed_at, created_at, updated_at)
VALUES
  ('attempt-dev-notify-feishu-failed-1', 'inst-dev-notify-feishu-failed', '172.16.103.25:9999', 'failed', 0, '参数不能为空 should not be empty', '2026-06-11T17:35:57+08:00', '2026-06-11T17:35:17+08:00', '2026-06-11T17:35:57+08:00'),
  ('attempt-dev-notify-feishu-succeeded-1', 'inst-dev-notify-feishu-succeeded', '172.16.103.26:9999', 'succeeded', 1, 'statement rebuilt successfully', '2026-06-11T17:40:12+08:00', '2026-06-11T17:40:00+08:00', '2026-06-11T17:40:12+08:00'),
  ('attempt-dev-notify-feishu-running-1', 'inst-dev-notify-feishu-running', '172.16.103.27:9999', 'succeeded', 1, 'statement rebuilt after progress notification', '2026-06-11T17:46:12+08:00', '2026-06-11T17:45:00+08:00', '2026-06-11T17:46:12+08:00')
ON CONFLICT(id) DO UPDATE SET
  instance_id = excluded.instance_id,
  worker_id = excluded.worker_id,
  status = excluded.status,
  result_success = excluded.result_success,
  result_message = excluded.result_message,
  result_completed_at = excluded.result_completed_at,
  created_at = excluded.created_at,
  updated_at = excluded.updated_at;

INSERT INTO job_instance_logs (id, instance_id, worker_id, level, message, sequence, created_at)
VALUES
  ('log-dev-notify-feishu-failed-1', 'inst-dev-notify-feishu-failed', '172.16.103.25:9999', 'info', '[demo.exception] accepted payload={"date":"2026-06-11","warehouse":"SH-A"}', 1, '2026-06-11T17:35:18+08:00'),
  ('log-dev-notify-feishu-failed-2', 'inst-dev-notify-feishu-failed', '172.16.103.25:9999', 'error', 'java.lang.IllegalArgumentException: 参数不能为空 should not be empty\n\tat net.tikeo.examples.worker.processor.FailingTaskProcessor.exception(FailingTaskProcessor.java:27)\n\tat net.tikeo.spring.processor.TikeoProcessorAdapter.handle(TikeoProcessorAdapter.java:37)', 2, '2026-06-11T17:35:57+08:00'),
  ('log-dev-notify-feishu-failed-3', 'inst-dev-notify-feishu-failed', '172.16.103.25:9999', 'error', 'authorization=Bearer demo-token-will-be-redacted routingKey:demo-routing-key signingKey=demo-signing-key', 3, '2026-06-11T17:35:58+08:00'),
  ('log-dev-notify-feishu-succeeded-1', 'inst-dev-notify-feishu-succeeded', '172.16.103.26:9999', 'info', '[demo.echo] rebuilt 128 customer statements', 1, '2026-06-11T17:40:10+08:00'),
  ('log-dev-notify-feishu-running-1', 'inst-dev-notify-feishu-running', '172.16.103.27:9999', 'info', '[demo.heartbeat] worker still processing shard 2/4', 1, '2026-06-11T17:45:05+08:00'),
  ('log-dev-notify-feishu-running-2', 'inst-dev-notify-feishu-running', '172.16.103.27:9999', 'info', '[demo.echo] rebuilt 96 customer statements after progress notification', 2, '2026-06-11T17:46:12+08:00')
ON CONFLICT(id) DO UPDATE SET
  instance_id = excluded.instance_id,
  worker_id = excluded.worker_id,
  level = excluded.level,
  message = excluded.message,
  sequence = excluded.sequence,
  created_at = excluded.created_at;

INSERT INTO notification_channels (id, scope_type, namespace, app, worker_pool, name, provider, enabled, config_json, secret_refs_json, target_redacted, safety_policy_json, created_by, updated_by, created_at, updated_at)
VALUES
  ('notif-channel-dev-feishu-interactive', 'app', 'dev-alpha', 'orders', NULL, 'Dev Feishu Job Card', 'feishu', 1, '{"messageType":"interactive","template":{"messageType":"interactive","card":{"config":{"wide_screen_mode":true},"header":{"template":"red","title":{"tag":"plain_text","content":"Tikeo Job 任务通知"}},"elements":[{"tag":"div","text":{"tag":"lark_md","content":"**报警类型**：{{subject}}\n**运行环境**：{{namespace}}\n**应用**：{{app}}\n**任务Handler**：{{jobId}}\n**任务名称**：{{jobName}}\n**触发时间**：{{startedAt}}\n**运行机器**：{{workerId}}\n**执行结果**：{{status}}\n**失败原因**：{{reason}}"}},{"tag":"hr"},{"tag":"action","actions":[{"tag":"button","text":{"tag":"plain_text","content":"查看控制台"},"type":"danger","url":"{{consoleUrl}}"}]}]}}}', '{"url":"https://open.feishu.cn/open-apis/bot/v2/hook/dev-seed-token","signingKey":"dev-seed-signing-key"}', 'https://open.feishu.cn/open-apis/bot/v2/hook/***', NULL, 'dev-seed', 'dev-seed', '2026-06-11T17:35:00+08:00', '2026-06-11T17:35:00+08:00')
ON CONFLICT(id) DO UPDATE SET
  scope_type = excluded.scope_type,
  namespace = excluded.namespace,
  app = excluded.app,
  worker_pool = excluded.worker_pool,
  name = excluded.name,
  provider = excluded.provider,
  enabled = excluded.enabled,
  config_json = excluded.config_json,
  secret_refs_json = excluded.secret_refs_json,
  target_redacted = excluded.target_redacted,
  safety_policy_json = excluded.safety_policy_json,
  updated_by = excluded.updated_by,
  updated_at = excluded.updated_at;

INSERT INTO notification_templates (id, template_key, name, description, provider, message_type, enabled, body_json, variables_json, created_by, updated_by, created_at, updated_at)
VALUES
  ('notif-template-dev-feishu-job-card', 'dev.feishu.job-card', 'Dev Feishu job card', 'Failure/success/status card persisted by scripts/dev-seed.sql for manual Notification Center acceptance.', 'feishu', 'interactive', 1, '{"messageType":"interactive","card":{"config":{"wide_screen_mode":true},"header":{"template":"red","title":{"tag":"plain_text","content":"Tikeo Job 任务通知"}},"elements":[{"tag":"div","text":{"tag":"lark_md","content":"**报警类型**：{{subject}}\n**运行环境**：{{namespace}}\n**应用**：{{app}}\n**任务Handler**：{{jobId}}\n**任务名称**：{{jobName}}\n**触发时间**：{{startedAt}}\n**运行机器**：{{workerId}}\n**执行结果**：{{status}}\n**失败原因**：{{reason}}"}},{"tag":"hr"},{"tag":"action","actions":[{"tag":"button","text":{"tag":"plain_text","content":"查看控制台"},"type":"danger","url":"{{consoleUrl}}"}]}]}}', '{"required":["subject","namespace","app","jobId","jobName","startedAt","workerId","status","reason","consoleUrl"]}', 'dev-seed', 'dev-seed', '2026-06-11T17:35:00+08:00', '2026-06-11T17:35:00+08:00')
ON CONFLICT(id) DO UPDATE SET
  template_key = excluded.template_key,
  name = excluded.name,
  description = excluded.description,
  provider = excluded.provider,
  message_type = excluded.message_type,
  enabled = excluded.enabled,
  body_json = excluded.body_json,
  variables_json = excluded.variables_json,
  updated_by = excluded.updated_by,
  updated_at = excluded.updated_at;

INSERT INTO notification_policies (id, name, enabled, owner_type, owner_id, event_family, event_filter_json, channel_refs_json, template_ref, severity, dedupe_seconds, throttle_json, quiet_hours_json, escalation_json, created_by, updated_by, created_at, updated_at)
VALUES
  ('notif-policy-dev-feishu-job-card', 'Dev Feishu job card acceptance policy', 1, 'job', 'job-dev-notify-exception', 'job_instance', '{"statuses":["failed","succeeded","running"],"eventTypes":["job_instance.failed","job_instance.succeeded","job_instance.running"]}', '[{"channelId":"notif-channel-dev-feishu-interactive"}]', 'dev.feishu.job-card', 'critical', 300, NULL, NULL, NULL, 'dev-seed', 'dev-seed', '2026-06-11T17:35:00+08:00', '2026-06-11T17:35:00+08:00')
ON CONFLICT(id) DO UPDATE SET
  name = excluded.name,
  enabled = excluded.enabled,
  owner_type = excluded.owner_type,
  owner_id = excluded.owner_id,
  event_family = excluded.event_family,
  event_filter_json = excluded.event_filter_json,
  channel_refs_json = excluded.channel_refs_json,
  template_ref = excluded.template_ref,
  severity = excluded.severity,
  dedupe_seconds = excluded.dedupe_seconds,
  throttle_json = excluded.throttle_json,
  quiet_hours_json = excluded.quiet_hours_json,
  escalation_json = excluded.escalation_json,
  updated_by = excluded.updated_by,
  updated_at = excluded.updated_at;

INSERT INTO notification_messages (id, source_type, source_id, policy_id, event_type, resource_type, resource_id, severity, subject, body, payload_json, dedupe_key, trace_id, status, created_at, updated_at)
VALUES
  ('notif-msg-dev-feishu-failed', 'job_instance', 'inst-dev-notify-feishu-failed', 'notif-policy-dev-feishu-job-card', 'job_instance.failed', 'job', 'job-dev-notify-exception', 'critical', '任务执行失败报警', 'Job AutoGenerateStockPdfRecordAfterDateTask instance inst-dev-notify-feishu-failed emitted job_instance.failed: 参数不能为空 should not be empty', '{"eventType":"job_instance.failed","jobId":"job-dev-notify-exception","jobName":"AutoGenerateStockPdfRecordAfterDateTask","resourceType":"job","resourceId":"job-dev-notify-exception","namespace":"dev-alpha","app":"orders","instanceId":"inst-dev-notify-feishu-failed","status":"failed","triggerType":"api","executionMode":"single","startedAt":"2026-06-11T17:35:17+08:00","finishedAt":"2026-06-11T17:35:57+08:00","workerId":"172.16.103.25:9999","operatorType":"system","operatorName":"tikeo","logsUrl":"/public/instances/inst-dev-notify-feishu-failed/console","consoleUrl":"/public/instances/inst-dev-notify-feishu-failed/console","reason":"参数不能为空 should not be empty","severity":"critical","policyId":"notif-policy-dev-feishu-job-card","dedupeKey":"notif-policy-dev-feishu-job-card:inst-dev-notify-feishu-failed:job_instance.failed","job":{"id":"job-dev-notify-exception","name":"AutoGenerateStockPdfRecordAfterDateTask","namespace":"dev-alpha","app":"orders","executionMode":"single"},"instance":{"id":"inst-dev-notify-feishu-failed","status":"failed","triggerType":"api","executionMode":"single","startedAt":"2026-06-11T17:35:17+08:00","finishedAt":"2026-06-11T17:35:57+08:00","workerId":"172.16.103.25:9999"},"operator":{"type":"system","name":"tikeo"},"console":{"url":"/public/instances/inst-dev-notify-feishu-failed/console"},"logs":{"url":"/public/instances/inst-dev-notify-feishu-failed/console","excerpt":null}}', 'notif-policy-dev-feishu-job-card:inst-dev-notify-feishu-failed:job_instance.failed', 'trace-dev-feishu-failed', 'delivered', '2026-06-11T17:35:58+08:00', '2026-06-11T17:35:59+08:00'),
  ('notif-msg-dev-feishu-succeeded', 'job_instance', 'inst-dev-notify-feishu-succeeded', 'notif-policy-dev-feishu-job-card', 'job_instance.succeeded', 'job', 'job-dev-notify-success', 'info', '任务执行成功通知', 'Job RebuildCustomerStatementTask instance inst-dev-notify-feishu-succeeded emitted job_instance.succeeded', '{"eventType":"job_instance.succeeded","jobId":"job-dev-notify-success","jobName":"RebuildCustomerStatementTask","resourceType":"job","resourceId":"job-dev-notify-success","namespace":"dev-alpha","app":"orders","instanceId":"inst-dev-notify-feishu-succeeded","status":"succeeded","triggerType":"api","executionMode":"single","startedAt":"2026-06-11T17:40:00+08:00","finishedAt":"2026-06-11T17:40:12+08:00","workerId":"172.16.103.26:9999","operatorType":"system","operatorName":"tikeo","logsUrl":"/public/instances/inst-dev-notify-feishu-succeeded/console","consoleUrl":"/public/instances/inst-dev-notify-feishu-succeeded/console","reason":"-","severity":"info","policyId":"notif-policy-dev-feishu-job-card","dedupeKey":"notif-policy-dev-feishu-job-card:inst-dev-notify-feishu-succeeded:job_instance.succeeded","job":{"id":"job-dev-notify-success","name":"RebuildCustomerStatementTask","namespace":"dev-alpha","app":"orders","executionMode":"single"},"instance":{"id":"inst-dev-notify-feishu-succeeded","status":"succeeded","triggerType":"api","executionMode":"single","startedAt":"2026-06-11T17:40:00+08:00","finishedAt":"2026-06-11T17:40:12+08:00","workerId":"172.16.103.26:9999"},"operator":{"type":"system","name":"tikeo"},"console":{"url":"/public/instances/inst-dev-notify-feishu-succeeded/console"},"logs":{"url":"/public/instances/inst-dev-notify-feishu-succeeded/console","excerpt":null}}', 'notif-policy-dev-feishu-job-card:inst-dev-notify-feishu-succeeded:job_instance.succeeded', 'trace-dev-feishu-succeeded', 'delivered', '2026-06-11T17:40:13+08:00', '2026-06-11T17:40:14+08:00'),
  ('notif-msg-dev-feishu-running', 'job_instance', 'inst-dev-notify-feishu-running', 'notif-policy-dev-feishu-job-card', 'job_instance.running', 'job', 'job-dev-notify-success', 'info', '任务执行状态通知', 'Job RebuildCustomerStatementTask instance inst-dev-notify-feishu-running emitted job_instance.running', '{"eventType":"job_instance.running","jobId":"job-dev-notify-success","jobName":"RebuildCustomerStatementTask","resourceType":"job","resourceId":"job-dev-notify-success","namespace":"dev-alpha","app":"orders","instanceId":"inst-dev-notify-feishu-running","status":"running","triggerType":"api","executionMode":"single","startedAt":"2026-06-11T17:45:00+08:00","finishedAt":null,"workerId":"172.16.103.27:9999","operatorType":"system","operatorName":"tikeo","logsUrl":"/public/instances/inst-dev-notify-feishu-running/console","consoleUrl":"/public/instances/inst-dev-notify-feishu-running/console","reason":"-","severity":"info","policyId":"notif-policy-dev-feishu-job-card","dedupeKey":"notif-policy-dev-feishu-job-card:inst-dev-notify-feishu-running:job_instance.running","job":{"id":"job-dev-notify-success","name":"RebuildCustomerStatementTask","namespace":"dev-alpha","app":"orders","executionMode":"single"},"instance":{"id":"inst-dev-notify-feishu-running","status":"running","triggerType":"api","executionMode":"single","startedAt":"2026-06-11T17:45:00+08:00","finishedAt":null,"workerId":"172.16.103.27:9999"},"operator":{"type":"system","name":"tikeo"},"console":{"url":"/public/instances/inst-dev-notify-feishu-running/console"},"logs":{"url":"/public/instances/inst-dev-notify-feishu-running/console","excerpt":null}}', 'notif-policy-dev-feishu-job-card:inst-dev-notify-feishu-running:job_instance.running', 'trace-dev-feishu-running', 'pending', '2026-06-11T17:45:06+08:00', '2026-06-11T17:45:06+08:00')
ON CONFLICT(id) DO UPDATE SET
  source_type = excluded.source_type,
  source_id = excluded.source_id,
  policy_id = excluded.policy_id,
  event_type = excluded.event_type,
  resource_type = excluded.resource_type,
  resource_id = excluded.resource_id,
  severity = excluded.severity,
  subject = excluded.subject,
  body = excluded.body,
  payload_json = excluded.payload_json,
  dedupe_key = excluded.dedupe_key,
  trace_id = excluded.trace_id,
  status = excluded.status,
  updated_at = excluded.updated_at;

INSERT INTO notification_delivery_attempts (id, message_id, policy_id, channel_id, provider, target_redacted, attempt, delivered, status_code, error, retry_state, next_retry_at, created_at)
VALUES
  ('notif-attempt-dev-feishu-failed-1', 'notif-msg-dev-feishu-failed', 'notif-policy-dev-feishu-job-card', 'notif-channel-dev-feishu-interactive', 'feishu', 'https://open.feishu.cn/open-apis/bot/v2/hook/***', 1, 1, 200, NULL, 'delivered', NULL, '2026-06-11T17:35:59+08:00'),
  ('notif-attempt-dev-feishu-succeeded-1', 'notif-msg-dev-feishu-succeeded', 'notif-policy-dev-feishu-job-card', 'notif-channel-dev-feishu-interactive', 'feishu', 'https://open.feishu.cn/open-apis/bot/v2/hook/***', 1, 1, 200, NULL, 'delivered', NULL, '2026-06-11T17:40:14+08:00'),
  ('notif-attempt-dev-feishu-running-1', 'notif-msg-dev-feishu-running', 'notif-policy-dev-feishu-job-card', 'notif-channel-dev-feishu-interactive', 'feishu', 'https://open.feishu.cn/open-apis/bot/v2/hook/***', 1, 0, NULL, 'demo seed keeps running/status card pending for retry inspection', 'retry_pending', '2026-06-11T17:50:06+08:00', '2026-06-11T17:45:06+08:00')
ON CONFLICT(id) DO UPDATE SET
  message_id = excluded.message_id,
  policy_id = excluded.policy_id,
  channel_id = excluded.channel_id,
  provider = excluded.provider,
  target_redacted = excluded.target_redacted,
  attempt = excluded.attempt,
  delivered = excluded.delivered,
  status_code = excluded.status_code,
  error = excluded.error,
  retry_state = excluded.retry_state,
  next_retry_at = excluded.next_retry_at,
  created_at = excluded.created_at;

INSERT INTO audit_logs (id, actor, action, resource_type, resource_id, detail, before, after, trace_id, result, failure_reason, ip_address, created_at)
VALUES
  ('audit-dev-seed-jobs', 'dev-seed', 'seed', 'jobs', 'job-dev-api-hello', 'Inserted development jobs and pending queue sample', NULL, NULL, 'trace-dev-seed', 'success', NULL, '127.0.0.1', '2026-01-01T00:00:00Z'),
  ('audit-dev-seed-workflow', 'dev-seed', 'seed', 'workflows', 'wf-dev-basic-pipeline', 'Inserted development workflow sample', NULL, NULL, 'trace-dev-seed', 'success', NULL, '127.0.0.1', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  actor = excluded.actor,
  action = excluded.action,
  resource_type = excluded.resource_type,
  resource_id = excluded.resource_id,
  detail = excluded.detail,
  before = excluded.before,
  after = excluded.after,
  trace_id = excluded.trace_id,
  result = excluded.result,
  failure_reason = excluded.failure_reason,
  ip_address = excluded.ip_address,
  created_at = excluded.created_at;

COMMIT;
