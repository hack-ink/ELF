#!/usr/bin/env python3
"""Validate the exact chat and embedding routes before a benchmark matrix starts."""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any


def endpoint(base: str, resource: str) -> str:
    parsed = urllib.parse.urlsplit(base.rstrip("/"))
    path = parsed.path.rstrip("/")
    if path.endswith(f"/{resource}"):
        pass
    elif path.endswith("/v1"):
        path = f"{path}/{resource}"
    else:
        path = f"{path}/v1/{resource}"
    return urllib.parse.urlunsplit(
        (parsed.scheme, parsed.netloc, path, parsed.query, parsed.fragment)
    )


def request_json(url: str, api_key: str, payload: dict[str, Any]) -> dict[str, Any]:
    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=90) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")[:2000]
        raise RuntimeError(f"provider HTTP {error.code}: {body}") from error


def required(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise KeyError(f"missing required configuration: {name}")
    return value


def sanitized_message(error: BaseException) -> str:
    message = str(error)
    for name in ("BENCHMARK_EMBEDDING_API_KEY", "BENCHMARK_CHAT_API_KEY"):
        value = os.environ.get(name)
        if value:
            message = message.replace(value, "[redacted]")
    return message


def provider_probe(action: Any) -> dict[str, Any]:
    started = time.monotonic()
    try:
        details = action()
        return {
            "classification": "completed",
            "duration_seconds": round(time.monotonic() - started, 3),
            **details,
        }
    except Exception as error:
        return {
            "classification": "provider_failed",
            "message": sanitized_message(error),
            "duration_seconds": round(time.monotonic() - started, 3),
        }


def main() -> int:
    started = time.monotonic()
    try:
        embedding_base = required("BENCHMARK_EMBEDDING_API_BASE")
        embedding_key = required("BENCHMARK_EMBEDDING_API_KEY")
        embedding_model = required("BENCHMARK_EMBEDDING_MODEL")
        embedding_dimensions = int(required("BENCHMARK_EMBEDDING_DIMENSIONS"))
        chat_base = required("BENCHMARK_CHAT_API_BASE")
        chat_key = required("BENCHMARK_CHAT_API_KEY")
        chat_model = required("BENCHMARK_CHAT_MODEL")
        reasoning_effort = required("BENCHMARK_CHAT_REASONING_EFFORT")

        def check_embedding() -> dict[str, Any]:
            response = request_json(
                endpoint(embedding_base, "embeddings"),
                embedding_key,
                {
                    "model": embedding_model,
                    "input": ["ELF benchmark provider preflight"],
                    "dimensions": embedding_dimensions,
                },
            )
            rows = response.get("data")
            vector = rows[0].get("embedding") if isinstance(rows, list) and rows else None
            if not isinstance(vector, list) or len(vector) != embedding_dimensions:
                actual = len(vector) if isinstance(vector, list) else None
                raise RuntimeError(
                    f"embedding route returned dimension {actual}; expected {embedding_dimensions}"
                )
            return {"model": embedding_model, "dimensions": embedding_dimensions}

        def check_chat() -> dict[str, Any]:
            response = request_json(
                endpoint(chat_base, "chat/completions"),
                chat_key,
                {
                    "model": chat_model,
                    "messages": [{"role": "user", "content": "Reply with OK."}],
                    "stream": False,
                    "max_tokens": 128,
                    "reasoning_effort": reasoning_effort,
                },
            )
            choices = response.get("choices")
            if not isinstance(choices, list) or not choices:
                raise RuntimeError("chat route returned no choices")
            return {"model": chat_model, "reasoning_effort": reasoning_effort}

        embedding = provider_probe(check_embedding)
        chat = provider_probe(check_chat)
        classification = (
            "completed"
            if embedding["classification"] == chat["classification"] == "completed"
            else "provider_failed"
        )

        result = {
            "schema": "elf.benchmark_provider_preflight/v1",
            "classification": classification,
            "embedding": embedding,
            "chat": chat,
            "duration_seconds": round(time.monotonic() - started, 3),
        }
        exit_code = 0 if classification == "completed" else 1
    except KeyError as error:
        result = {
            "schema": "elf.benchmark_provider_preflight/v1",
            "classification": "configuration_failed",
            "message": sanitized_message(error),
            "duration_seconds": round(time.monotonic() - started, 3),
        }
        exit_code = 2
    except Exception as error:
        result = {
            "schema": "elf.benchmark_provider_preflight/v1",
            "classification": "provider_failed",
            "message": sanitized_message(error),
            "duration_seconds": round(time.monotonic() - started, 3),
        }
        exit_code = 1
    print(json.dumps(result, ensure_ascii=True, sort_keys=True))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
