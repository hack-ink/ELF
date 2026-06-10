#!/usr/bin/env python3
"""Small OpenAI-compatible mock provider for LightRAG Docker smokes."""

from __future__ import annotations

import hashlib
import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any


EMBEDDING_DIM = int(os.environ.get("ELF_LIGHTRAG_MOCK_EMBEDDING_DIM", "64"))
HOST = os.environ.get("ELF_LIGHTRAG_MOCK_HOST", "0.0.0.0")
PORT = int(os.environ.get("ELF_LIGHTRAG_MOCK_PORT", "8080"))


def _read_json(handler: BaseHTTPRequestHandler) -> dict[str, Any]:
    length = int(handler.headers.get("content-length", "0"))
    if length == 0:
        return {}
    raw = handler.rfile.read(length)
    return json.loads(raw.decode("utf-8"))


def _write_json(handler: BaseHTTPRequestHandler, status: int, payload: dict[str, Any]) -> None:
    body = json.dumps(payload).encode("utf-8")
    handler.send_response(status)
    handler.send_header("content-type", "application/json")
    handler.send_header("content-length", str(len(body)))
    handler.end_headers()
    handler.wfile.write(body)


def _embedding(text: str) -> list[float]:
    vector = [0.0] * EMBEDDING_DIM
    for term in "".join(ch.lower() if ch.isalnum() else " " for ch in text).split():
        if len(term) < 2:
            continue
        digest = hashlib.blake2b(term.encode("utf-8"), digest_size=8).digest()
        index = int.from_bytes(digest[:4], "little") % EMBEDDING_DIM
        vector[index] += 1.0
    norm = sum(value * value for value in vector) ** 0.5
    if norm > 0:
        vector = [value / norm for value in vector]
    return vector


def _chat_completion(request: dict[str, Any]) -> dict[str, Any]:
    content = (
        '{"entities":[],"relationships":[],"summary":"No graph facts extracted by '
        'the local LightRAG smoke provider."}'
    )
    return {
        "id": "elf-lightrag-mock-chat",
        "object": "chat.completion",
        "model": request.get("model", "elf-lightrag-mock"),
        "choices": [
            {
                "index": 0,
                "finish_reason": "stop",
                "message": {"role": "assistant", "content": content},
            }
        ],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
    }


def _embeddings(request: dict[str, Any]) -> dict[str, Any]:
    inputs = request.get("input", [])
    if isinstance(inputs, str):
        inputs = [inputs]
    return {
        "object": "list",
        "model": request.get("model", "elf-lightrag-mock-embedding"),
        "data": [
            {"object": "embedding", "index": index, "embedding": _embedding(str(text))}
            for index, text in enumerate(inputs)
        ],
        "usage": {"prompt_tokens": 0, "total_tokens": 0},
    }


def _rerank(request: dict[str, Any]) -> dict[str, Any]:
    documents = request.get("documents", [])
    if not isinstance(documents, list):
        documents = []
    return {
        "id": "elf-lightrag-mock-rerank",
        "results": [
            {"index": index, "relevance_score": 1.0 / (index + 1)}
            for index, _document in enumerate(documents)
        ],
    }


class Handler(BaseHTTPRequestHandler):
    """HTTP handler for the mock provider."""

    def do_GET(self) -> None:
        if self.path in {"/health", "/v1/health"}:
            _write_json(self, 200, {"status": "ok"})
            return
        _write_json(self, 404, {"error": "not_found"})

    def do_POST(self) -> None:
        try:
            request = _read_json(self)
            if self.path.endswith("/chat/completions"):
                _write_json(self, 200, _chat_completion(request))
            elif self.path.endswith("/embeddings"):
                _write_json(self, 200, _embeddings(request))
            elif self.path.endswith("/rerank") or self.path == "/rerank":
                _write_json(self, 200, _rerank(request))
            else:
                _write_json(self, 404, {"error": "not_found", "path": self.path})
        except Exception as exc:  # noqa: BLE001
            _write_json(self, 500, {"error": "mock_provider_error", "detail": str(exc)})

    def log_message(self, format: str, *args: Any) -> None:
        return


if __name__ == "__main__":
    server = ThreadingHTTPServer((HOST, PORT), Handler)
    server.serve_forever()
