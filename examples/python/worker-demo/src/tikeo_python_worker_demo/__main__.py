from __future__ import annotations

import json
import logging
from dataclasses import asdict
import os
import time
from typing import Iterable

import tikeo

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")


def main() -> None:
    config = tikeo.local_config(env_or("TIKEO_WORKER_ENDPOINT", "http://127.0.0.1:9998"), env_or("TIKEO_WORKER_CLIENT_INSTANCE_ID", "python-worker-demo-local"))
    config.namespace = env_or("TIKEO_WORKER_NAMESPACE", "dev-alpha")
    config.app = env_or("TIKEO_WORKER_APP", "orders")
    config.cluster = env_or("TIKEO_WORKER_CLUSTER", "local")
    config.region = env_or("TIKEO_WORKER_REGION", "local")
    config.add_tag("python")
    config.add_tag("manual-demo")
    for processor in csv_or("TIKEO_WORKER_SDK_PROCESSORS", "demo.echo,demo.context,demo.bytes,demo.heartbeat,demo.fail,demo.exception"):
        config.add_sdk_processor(processor)
    config.labels["worker_pool"] = env_or("TIKEO_WORKER_POOL", "python-blue")
    if enabled_by_default("TIKEO_ENABLE_PLUGIN_SQL"):
        config.add_plugin_processor(env_or("TIKEO_PLUGIN_SQL_TYPE", "sql"), env_or("TIKEO_PLUGIN_SQL_PROCESSOR", "billing.sql-sync"))
        config.labels["plugin_sql"] = "enabled"

    scripts = configure_scripts(config)
    client = tikeo.Client(config)
    registration = client.registration()
    print("python worker demo configured: " + json.dumps(asdict(registration), indent=2))

    if enabled("TIKEO_MANAGEMENT_CREATE_EXAMPLES"):
        mgmt = tikeo.ManagementClient(env_or("TIKEO_HTTP_URL", "http://127.0.0.1:8080"), os.environ.get("TIKEO_API_KEY", ""), config.namespace, config.app)
        for job in [tikeo.api_job("python-echo-api", "demo.echo"), tikeo.plugin_api_job("python-sql-sync-api", "sql", "billing.sql-sync")]:
            try:
                created = mgmt.create_job(job)
                instance = mgmt.trigger_job(created.id)
                logging.info("created and triggered job %s/%s %s instance=%s trigger_type=%s", created.namespace, created.app, created.name, instance.id, instance.trigger_type)
            except Exception as exc:
                logging.warning("create job %s failed: %s", job.name, exc)

    if dry_run_enabled():
        client.start_dry_run(process_task)
        heartbeat = client.next_heartbeat("dry-run-worker", "dry-run-fence", 1)
        print(f"dry_run_heartbeat_sequence={heartbeat.sequence}")
        return

    oneshot = enabled("TIKEO_WORKER_ONESHOT")
    while True:
        try:
            session = client.connect()
            stop = session.start_heartbeat()
            logging.info("python worker connected: worker_id=%s generation=%s lease_seconds=%s", session.worker_id, session.generation, session.lease_seconds)
            try:
                while True:
                    outcome = session.process_next(process_task, scripts)
                    logging.info("processed task success=%s message=%s", outcome.success, outcome.message)
                    if oneshot:
                        return
                    time.sleep(0.05)
            finally:
                stop.set()
                session.close()
        except Exception as exc:
            logging.warning("worker tunnel ended, reconnecting: %s", exc)
            time.sleep(2)


def configure_scripts(config: tikeo.WorkerConfig) -> tikeo.ScriptRunnerRegistry:
    scripts = tikeo.ScriptRunnerRegistry()
    resolver = tikeo.SandboxToolResolver(state_dir=env_or("TIKEO_WORKER_STATE_DIR", ""), auto_install=not disabled("TIKEO_SANDBOX_AUTO_INSTALL"))
    for language in csv_or("TIKEO_WORKER_SCRIPT_LANGUAGES", "shell,python,javascript,typescript,powershell,php,groovy,rhai"):
        if disabled("TIKEO_ENABLE_SCRIPT_" + language.upper()):
            continue
        backend = script_sandbox_backend(language)
        try:
            if backend == "srt":
                srt, srt_ok = resolver.resolve_srt()
                rg, rg_ok = resolver.resolve_ripgrep()
                interpreter, interpreter_ok = resolve_srt_interpreter(language, resolver)
                if srt_ok and rg_ok and interpreter_ok:
                    scripts.register(tikeo.SrtScriptRunner(language, srt, interpreter, sandbox_tool_path_entries(srt, rg, interpreter, resolver)))
                    logging.info("script runner %s registered backend=srt interpreter=%s", language, interpreter)
                    continue
                logging.warning("script runner %s skipped: srt_ok=%s rg_ok=%s interpreter_ok=%s interpreter=%s", language, srt_ok, rg_ok, interpreter_ok, interpreter)
            elif backend in {"deno", "v8"}:
                deno, ok = resolver.resolve_deno()
                if ok:
                    scripts.register(tikeo.DenoScriptRunner(language, deno))
                    logging.info("script runner %s registered backend=deno runtime=%s", language, deno)
                    continue
                logging.warning("script runner %s skipped: deno unavailable runtime=%s", language, deno)
            elif backend in {"docker", "podman"}:
                scripts.register(tikeo.ContainerScriptRunner(language, backend, script_image(language)))
                logging.info("script runner %s registered backend=%s", language, backend)
                continue
            elif enabled("TIKEO_ENABLE_LOCAL_SCRIPT_" + language.upper()):
                scripts.register(tikeo.LocalCommandScriptRunner(language, "custom"))
                logging.info("script runner %s registered backend=custom", language)
                continue
        except Exception as exc:
            logging.warning("script runner %s skipped: %s", language, exc)
        if enabled("TIKEO_ENABLE_UNAVAILABLE_SCRIPT_ADAPTERS"):
            scripts.register(tikeo.UnavailableScriptRunner(language, backend, f"{backend} sandbox backend is unavailable; auto requires SRT+rg for native scripts and Deno for JavaScript/TypeScript"))
    scripts.add_capabilities(config)
    return scripts


def process_task(task: tikeo.TaskContext) -> tikeo.TaskOutcome:
    logging.info("[python-worker] processor=%s instance=%s payload_bytes=%s", task.processor_name, task.instance_id, len(task.payload))
    payload = task.payload.decode(errors="replace")
    match task.processor_name or "demo.echo":
        case "" | "demo.echo":
            logging.info("[demo.echo] payload=%r", payload)
            return tikeo.TaskOutcome(True, "python demo echo processed")
        case "demo.context":
            logging.info("[demo.context] jobId=%s instanceId=%s", task.job_id, task.instance_id)
            return tikeo.TaskOutcome(True, f"python demo context processed instance={task.instance_id}")
        case "demo.bytes":
            logging.info("[demo.bytes] payload=%r length=%s", payload, len(task.payload))
            return tikeo.TaskOutcome(True, f"python demo bytes processed payload_bytes={len(task.payload)}")
        case "demo.heartbeat":
            logging.info("[demo.heartbeat] tick jobId=%s instanceId=%s", task.job_id, task.instance_id)
            return tikeo.TaskOutcome(True, "python demo heartbeat processed")
        case "billing.sql-sync":
            logging.info("[billing.sql-sync] plugin SQL processor received payload=%r", payload)
            return tikeo.TaskOutcome(True, "python demo sql plugin processed")
        case "demo.fail":
            logging.error("[demo.fail] intentional failure payload=%r", payload)
            return tikeo.failed("python demo intentional failure")
        case "demo.exception":
            logging.error("[demo.exception] raising runtime exception payload=%r", payload)
            raise RuntimeError("python demo runtime exception")
        case other:
            logging.error("[python-worker] unsupported processor=%s", other)
            return tikeo.failed("unsupported python demo processor: " + other)


def script_sandbox_backend(language: str) -> str:
    value = os.environ.get("TIKEO_WORKER_SCRIPT_SANDBOX", "").strip()
    if value and value.lower() != "auto":
        return value.lower()
    return "deno" if language.lower() in {"javascript", "js", "typescript", "ts"} else "srt"


def resolve_srt_interpreter(language: str, resolver: tikeo.SandboxToolResolver) -> tuple[str, bool]:
    key = language.strip().lower()
    if key in {"shell", "sh", "bash"}:
        return resolver.resolve_interpreter("sh")
    if key in {"python", "py"}:
        return resolver.resolve_interpreter("python3")
    if key in {"powershell", "pwsh"}:
        return resolver.resolve_powershell()
    if key == "php":
        return resolver.resolve_interpreter("php")
    if key == "groovy":
        return resolver.resolve_interpreter("groovy")
    if key == "rhai":
        return resolver.resolve_rhai()
    return resolver.resolve_interpreter("sh")


def sandbox_tool_path_entries(srt: str, rg: str, interpreter: str, resolver: tikeo.SandboxToolResolver) -> list[str]:
    entries = [parent for parent in [tool_parent(srt), tool_parent(rg), tool_parent(interpreter)] if parent]
    for value, ok in [resolver.resolve_node(), resolver.resolve_npm()]:
        if ok and tool_parent(value):
            entries.append(tool_parent(value))
    return list(dict.fromkeys(entries))


def tool_parent(command: str) -> str:
    return str(os.path.dirname(command)) if os.path.sep in command else ""


def script_image(language: str) -> str:
    images = {
        "shell": env_or("TIKEO_SHELL_IMAGE", "alpine:latest"),
        "python": env_or("TIKEO_PYTHON_IMAGE", "python:alpine"),
        "javascript": env_or("TIKEO_JAVASCRIPT_IMAGE", "denoland/deno:alpine"),
        "typescript": env_or("TIKEO_TYPESCRIPT_IMAGE", "denoland/deno:alpine"),
        "powershell": env_or("TIKEO_POWERSHELL_IMAGE", "mcr.microsoft.com/powershell:latest"),
        "php": env_or("TIKEO_PHP_IMAGE", "php:cli-alpine"),
        "groovy": env_or("TIKEO_GROOVY_IMAGE", "groovy:latest"),
        "rhai": env_or("TIKEO_RHAI_IMAGE", "rhaiscript/rhai:latest"),
    }
    return images.get(tikeo.normalize_script_language(language), "")


def env_or(key: str, fallback: str) -> str:
    return os.environ.get(key, "").strip() or fallback


def csv_or(key: str, fallback: str) -> list[str]:
    value = env_or(key, fallback)
    return [item.strip() for item in value.split(",") if item.strip()]


def enabled_by_default(key: str) -> bool:
    return not disabled(key)


def enabled(key: str) -> bool:
    return os.environ.get(key, "").strip().lower() in {"1", "true", "yes", "on"}


def disabled(key: str) -> bool:
    return os.environ.get(key, "").strip().lower() in {"0", "false", "no", "off"}


def dry_run_enabled() -> bool:
    return enabled("TIKEO_WORKER_DRY_RUN") or disabled("TIKEO_WORKER_CONNECT")


if __name__ == "__main__":
    main()
