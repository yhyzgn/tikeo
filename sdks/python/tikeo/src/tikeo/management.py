"""Management API helpers for SDK-side app-scoped jobs."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Any
from urllib.parse import quote

import requests

from .errors import ManagementRequestError

API_KEY_HEADER = "x-tikeo-api-key"


@dataclass(slots=True)
class JobRetryPolicy:
    enabled: bool = True
    max_attempts: int = 3
    initial_delay_seconds: int = 5
    backoff_multiplier: int = 2
    max_delay_seconds: int = 60

    def to_json(self) -> dict[str, Any]:
        return {
            "enabled": self.enabled,
            "maxAttempts": self.max_attempts,
            "initialDelaySeconds": self.initial_delay_seconds,
            "backoffMultiplier": self.backoff_multiplier,
            "maxDelaySeconds": self.max_delay_seconds,
        }


def default_job_retry_policy() -> JobRetryPolicy:
    return JobRetryPolicy()


@dataclass(slots=True)
class JobDefinition:
    id: str = ""
    namespace: str = ""
    app: str = ""
    name: str = ""
    schedule_type: str = ""
    schedule_expr: str | None = None
    processor_name: str | None = None
    processor_type: str | None = None
    script_id: str | None = None
    enabled: bool = True
    retry_policy: JobRetryPolicy | None = None


@dataclass(slots=True)
class JobInstance:
    id: str = ""
    job_id: str = ""
    status: str = ""
    trigger_type: str = ""
    execution_mode: str = ""
    created_at: str = ""
    updated_at: str = ""


@dataclass(slots=True)
class CreateJobRequest:
    name: str
    schedule_type: str = "api"
    schedule_expr: str | None = None
    processor_name: str | None = None
    processor_type: str | None = None
    script_id: str | None = None
    enabled: bool = True
    retry_policy: JobRetryPolicy | None = None


@dataclass(slots=True)
class BroadcastSelectorRequest:
    tags: list[str] | None = None
    region: str | None = None
    cluster: str | None = None
    labels: dict[str, str] | None = None

    def to_json(self) -> dict[str, Any]:
        payload = {
            "tags": self.tags,
            "region": self.region,
            "cluster": self.cluster,
            "labels": self.labels,
        }
        return {key: value for key, value in payload.items() if value is not None}


@dataclass(slots=True)
class TriggerJobRequest:
    trigger_type: str = "api"
    execution_mode: str = "single"
    broadcast_selector: BroadcastSelectorRequest | None = None

    def to_json(self) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "triggerType": self.trigger_type,
            "executionMode": self.execution_mode,
        }
        if self.broadcast_selector is not None:
            payload["broadcastSelector"] = self.broadcast_selector.to_json()
        return payload


def api_trigger() -> TriggerJobRequest:
    return TriggerJobRequest()


def broadcast_api_trigger(selector: BroadcastSelectorRequest | None = None) -> TriggerJobRequest:
    return TriggerJobRequest(execution_mode="broadcast", broadcast_selector=selector)


def api_job(name: str, processor_name: str) -> CreateJobRequest:
    return CreateJobRequest(name=name, processor_name=processor_name, retry_policy=default_job_retry_policy())


def plugin_api_job(name: str, processor_type: str, processor_name: str) -> CreateJobRequest:
    return CreateJobRequest(name=name, processor_type=processor_type, processor_name=processor_name, retry_policy=default_job_retry_policy())


def script_api_job(name: str, script_id: str) -> CreateJobRequest:
    return CreateJobRequest(name=name, script_id=script_id, retry_policy=default_job_retry_policy())


class ManagementClient:
    def __init__(self, endpoint: str, api_key: str, namespace: str = "default", app: str = "default") -> None:
        self.endpoint = endpoint.strip().rstrip("/")
        self.api_key = api_key
        self.namespace = namespace.strip() or "default"
        self.app = app.strip() or "default"
        self.session = requests.Session()
        self.session.headers.update({"accept": "application/json", API_KEY_HEADER: api_key})

    def list_jobs(self) -> list[JobDefinition]:
        data = self._send("GET", "/jobs")
        items = data.get("items", []) if isinstance(data, dict) else []
        return [self._parse_job(item) for item in items if item.get("namespace") == self.namespace and item.get("app") == self.app]

    def create_job(self, request: CreateJobRequest) -> JobDefinition:
        payload = {
            "namespace": self.namespace,
            "app": self.app,
            "name": request.name,
            "scheduleType": request.schedule_type,
            "scheduleExpr": request.schedule_expr,
            "processorName": request.processor_name,
            "processorType": request.processor_type,
            "scriptId": request.script_id,
            "enabled": request.enabled,
            "retryPolicy": request.retry_policy.to_json() if request.retry_policy else None,
        }
        payload = {key: value for key, value in payload.items() if value is not None}
        return self._parse_job(self._send("POST", "/jobs", payload))

    def trigger_job(self, job_id: str, request: TriggerJobRequest | None = None) -> JobInstance:
        body = (request or api_trigger()).to_json()
        return self._parse_instance(self._send("POST", f"/jobs/{quote(job_id, safe='')}:trigger", body))

    def _send(self, method: str, path: str, body: dict[str, Any] | None = None) -> Any:
        response = self.session.request(method, f"{self.endpoint}/api/v1{path}", json=body, timeout=30)
        try:
            envelope = response.json()
        except ValueError as exc:
            raise ManagementRequestError(f"tikeo management response was not JSON: {response.status_code}") from exc
        if response.status_code < 200 or response.status_code >= 300 or envelope.get("code") != 0:
            raise ManagementRequestError(f"tikeo management request failed: status={response.status_code} message={envelope.get('message', '')}")
        if envelope.get("data") is None:
            raise ManagementRequestError("tikeo management response data was null")
        return envelope["data"]

    @staticmethod
    def _parse_job(data: dict[str, Any]) -> JobDefinition:
        retry = data.get("retryPolicy")
        return JobDefinition(
            id=data.get("id", ""),
            namespace=data.get("namespace", ""),
            app=data.get("app", ""),
            name=data.get("name", ""),
            schedule_type=data.get("scheduleType", ""),
            schedule_expr=data.get("scheduleExpr"),
            processor_name=data.get("processorName"),
            processor_type=data.get("processorType"),
            script_id=data.get("scriptId"),
            enabled=bool(data.get("enabled", True)),
            retry_policy=JobRetryPolicy(
                enabled=retry.get("enabled", True),
                max_attempts=retry.get("maxAttempts", 3),
                initial_delay_seconds=retry.get("initialDelaySeconds", 5),
                backoff_multiplier=retry.get("backoffMultiplier", 2),
                max_delay_seconds=retry.get("maxDelaySeconds", 60),
            ) if isinstance(retry, dict) else None,
        )

    @staticmethod
    def _parse_instance(data: dict[str, Any]) -> JobInstance:
        return JobInstance(
            id=data.get("id", ""),
            job_id=data.get("jobId", ""),
            status=data.get("status", ""),
            trigger_type=data.get("triggerType", ""),
            execution_mode=data.get("executionMode", ""),
            created_at=data.get("createdAt", ""),
            updated_at=data.get("updatedAt", ""),
        )
