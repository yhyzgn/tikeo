#!/usr/bin/env python3
"""Synchronize a Docker Hub repository overview from a Markdown file."""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

API_ROOT = "https://hub.docker.com/v2"


def request_json(method: str, url: str, payload: dict[str, object] | None = None, token: str | None = None) -> dict[str, object]:
    data = None
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"JWT {token}"
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            body = response.read().decode("utf-8")
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"Docker Hub API {method} {url} failed with HTTP {error.code}: {detail}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Docker Hub API {method} {url} failed: {error}") from error
    return json.loads(body) if body else {}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True, help="Docker Hub repository, for example yhyzgn/tikeo-docs")
    parser.add_argument("--readme", required=True, type=Path, help="Markdown overview file")
    parser.add_argument("--description", required=True, help="Short Docker Hub repository description")
    args = parser.parse_args()

    username = os.environ.get("DOCKERHUB_USERNAME")
    password = os.environ.get("DOCKERHUB_TOKEN")
    if not username or not password:
        raise RuntimeError("DOCKERHUB_USERNAME and DOCKERHUB_TOKEN must be set")

    namespace, _, repo = args.repository.partition("/")
    if not namespace or not repo:
        raise RuntimeError("--repository must use namespace/name format")

    overview = args.readme.read_text(encoding="utf-8")
    if not overview.strip():
        raise RuntimeError(f"overview file is empty: {args.readme}")
    description_bytes = len(args.description.encode("utf-8"))
    if description_bytes > 100:
        raise RuntimeError(f"Docker Hub short description must be at most 100 bytes, got {description_bytes}")

    login = request_json("POST", f"{API_ROOT}/users/login/", {"username": username, "password": password})
    token = login.get("token")
    if not isinstance(token, str) or not token:
        raise RuntimeError("Docker Hub login response did not include a token")

    request_json(
        "PATCH",
        f"{API_ROOT}/repositories/{namespace}/{repo}/",
        {"description": args.description, "full_description": overview},
        token,
    )
    print(f"Updated Docker Hub overview for {args.repository} from {args.readme}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:  # noqa: BLE001 - CLI should surface concise failure context.
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
