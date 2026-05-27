-- Development integration seed data for tikee.
--
-- Usage:
--   1. Start the dev server once so migrations create the schema:
--      ./scripts/dev.sh
--   2. Apply this script to the SQLite dev database:
--      sqlite3 tikee-dev.db < scripts/dev-seed.sql
--
-- The script is idempotent: stable ids are upserted, so it can be re-run safely.
-- It is intended for local development only, not production bootstrapping.

BEGIN TRANSACTION;

INSERT INTO namespaces (id, name, created_at, updated_at)
VALUES ('ns-dev-default', 'default', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  name = excluded.name,
  updated_at = excluded.updated_at;

INSERT INTO apps (id, namespace_id, name, created_at, updated_at)
VALUES ('app-dev-observability', 'ns-dev-default', 'observability-demo', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  namespace_id = excluded.namespace_id,
  name = excluded.name,
  updated_at = excluded.updated_at;

INSERT INTO apps (id, namespace_id, name, created_at, updated_at)
VALUES ('app-dev-default', 'ns-dev-default', 'default', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  namespace_id = excluded.namespace_id,
  name = excluded.name,
  updated_at = excluded.updated_at;

INSERT INTO users (id, username, password, role, created_at)
VALUES
  ('usr-dev-operator', 'dev_operator', '$2b$10$vslUa5GAP.Mk3s4PPclu..miTj/beUTaSCR/HSZdfPVXmhA/7lmpm', 'operator', '2026-01-01T00:00:00Z'),
  ('usr-dev-viewer', 'dev_viewer', '$2b$10$vslUa5GAP.Mk3s4PPclu..miTj/beUTaSCR/HSZdfPVXmhA/7lmpm', 'viewer', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  username = excluded.username,
  password = excluded.password,
  role = excluded.role;

INSERT INTO jobs (id, namespace_id, app_id, name, schedule_type, schedule_expr, processor_name, enabled, created_at, updated_at)
VALUES
  ('job-dev-api-hello', 'ns-dev-default', 'app-dev-default', 'api-hello', 'api', NULL, 'demo.echo', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('job-dev-fixed-rate-heartbeat', 'ns-dev-default', 'app-dev-default', 'fixed-rate-heartbeat', 'fixed_rate', '30s', 'demo.heartbeat', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('job-dev-cron-minute-report', 'ns-dev-default', 'app-dev-default', 'cron-minute-report', 'cron', '0/30 * * * * * *', 'demo.report', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
ON CONFLICT(id) DO UPDATE SET
  namespace_id = excluded.namespace_id,
  app_id = excluded.app_id,
  name = excluded.name,
  schedule_type = excluded.schedule_type,
  schedule_expr = excluded.schedule_expr,
  processor_name = excluded.processor_name,
  enabled = excluded.enabled,
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
echo "hello from tikee dev seed: ${TIKEE_DEV_MESSAGE:-ok}"
', 'approved', 'script-version-dev-shell-hello-1', 1, 10, 67108864, 0, '["TIKEE_DEV_MESSAGE"]', '{"network":{"enabled":false},"filesystem":{"read_only_paths":[],"writable_paths":[]},"resources":{"timeout_ms":10000,"max_memory_bytes":67108864},"env_vars":["TIKEE_DEV_MESSAGE"]}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
  ('script-dev-python-json', 'dev-python-json', 'python', '1.0.0', 'import json
print(json.dumps({"status":"ok","source":"tikee-dev-seed"}))
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
echo "hello from tikee dev seed: ${TIKEE_DEV_MESSAGE:-ok}"
', '991474538b28fa818388441d7fb96c51ecc3914914bda96f2d3cf480003ada31', 'shell', 'approved', 10, 67108864, 0, '["TIKEE_DEV_MESSAGE"]', '{"network":{"enabled":false},"filesystem":{"read_only_paths":[],"writable_paths":[]},"resources":{"timeout_ms":10000,"max_memory_bytes":67108864},"env_vars":["TIKEE_DEV_MESSAGE"]}', 'usr-admin', '2026-01-01T00:00:00Z'),
  ('script-version-dev-python-json-1', 'script-dev-python-json', 1, 'import json
print(json.dumps({"status":"ok","source":"tikee-dev-seed"}))
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
echo "tikee shell script example ok"
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
', 'approved', 'script-version-dev-rhai-example-1', 1, 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
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
echo "tikee shell script example ok"
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
', 'c5fbea054ea991344bda993da92f06498e28822e57f8e6c51c77d694f1d4b6c9', 'rhai', 'approved', 10, 67108864, 0, '[]', '{"resources":{"timeout_ms":10000,"max_memory_bytes":67108864,"max_output_bytes":1048576},"network":{"enabled":false,"allowed_hosts":[]},"filesystem":{"read_only_paths":[],"writable_paths":[]},"secrets":{"refs":[]},"env_vars":[],"sandbox":{"backend":"auto"}}', 'usr-admin', '2026-01-01T00:00:00Z')
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

INSERT INTO jobs (id, namespace_id, app_id, name, schedule_type, schedule_expr, processor_name, enabled, created_at, updated_at, script_id)
VALUES
  ('job-dev-script-shell-example', 'ns-dev-default', 'app-dev-default', 'dev-shell-script-job', 'api', NULL, NULL, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-shell-example'),
  ('job-dev-script-python-example', 'ns-dev-default', 'app-dev-default', 'dev-python-script-job', 'api', NULL, NULL, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-python-example'),
  ('job-dev-script-javascript-example', 'ns-dev-default', 'app-dev-default', 'dev-javascript-script-job', 'api', NULL, NULL, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-javascript-example'),
  ('job-dev-script-typescript-example', 'ns-dev-default', 'app-dev-default', 'dev-typescript-script-job', 'api', NULL, NULL, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-typescript-example'),
  ('job-dev-script-powershell-example', 'ns-dev-default', 'app-dev-default', 'dev-powershell-script-job', 'api', NULL, NULL, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-powershell-example'),
  ('job-dev-script-rhai-example', 'ns-dev-default', 'app-dev-default', 'dev-rhai-script-job', 'api', NULL, NULL, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'script-dev-rhai-example')
ON CONFLICT(id) DO UPDATE SET
  namespace_id = excluded.namespace_id,
  app_id = excluded.app_id,
  name = excluded.name,
  schedule_type = excluded.schedule_type,
  schedule_expr = excluded.schedule_expr,
  processor_name = excluded.processor_name,
  enabled = excluded.enabled,
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
