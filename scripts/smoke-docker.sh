#!/usr/bin/env bash
set -euo pipefail

smoke="${1:-}"
if [ -z "$smoke" ]; then
	echo "usage: scripts/smoke-docker.sh <smoke>" >&2
	exit 2
fi

case "$smoke" in
graphify-docker-graph-report)
	docker compose -f docker-compose.baseline.yml run --build --rm \
		-e ELF_GRAPHIFY_SMOKE_RUN \
		-e ELF_GRAPHIFY_SMOKE_REPORT_DIR \
		-e ELF_GRAPHIFY_SMOKE_WORK_DIR \
		-e ELF_GRAPHIFY_SMOKE_INSTALL \
		-e ELF_GRAPHIFY_PACKAGE \
		-e ELF_GRAPHIFY_REF \
		-e ELF_GRAPHIFY_TIMEOUT_SECONDS \
		-e ELF_GRAPHIFY_QUERY_BUDGET \
		baseline-runner python3 scripts/graphify-docker-graph-report-smoke.py
	;;
graphiti-zep-docker-temporal)
	start="$(printenv ELF_GRAPHITI_ZEP_SMOKE_START || true)"
	status=0
	if [ "$start" = "1" ]; then
		docker compose -f docker-compose.baseline.yml --profile graphiti-zep up -d graphiti-falkordb
	fi
	docker compose -f docker-compose.baseline.yml run --build --rm \
		-e ELF_GRAPHITI_ZEP_SMOKE_RUN \
		-e ELF_GRAPHITI_ZEP_SMOKE_REPORT_DIR \
		-e ELF_GRAPHITI_ZEP_SMOKE_WORK_DIR \
		-e ELF_GRAPHITI_ZEP_SMOKE_INSTALL \
		-e ELF_GRAPHITI_ZEP_VERSION \
		-e ELF_GRAPHITI_ZEP_PACKAGE \
		-e ELF_GRAPHITI_ZEP_REF \
		-e ELF_GRAPHITI_ZEP_API_BASE \
		-e ELF_GRAPHITI_ZEP_API_KEY \
		-e ELF_GRAPHITI_ZEP_LLM_MODEL \
		-e ELF_GRAPHITI_ZEP_EMBEDDING_MODEL \
		-e ELF_GRAPHITI_ZEP_FALKORDB_HOST \
		-e ELF_GRAPHITI_ZEP_FALKORDB_PORT \
		-e ELF_GRAPHITI_ZEP_FALKORDB_DATABASE \
		-e ELF_GRAPHITI_ZEP_TIMEOUT_SECONDS \
		-e ELF_GRAPHITI_ZEP_STARTUP_ATTEMPTS \
		-e ELF_GRAPHITI_ZEP_STARTUP_INTERVAL_SECONDS \
		baseline-runner python3 scripts/graphiti-zep-docker-temporal-smoke.py || status=$?
	if [ "$start" = "1" ]; then
		docker compose -f docker-compose.baseline.yml --profile graphiti-zep stop graphiti-falkordb >/dev/null 2>&1 || true
	fi
	exit "$status"
	;;
graphrag-docker)
	docker compose -f docker-compose.baseline.yml run --build --rm \
		-e ELF_GRAPHRAG_SMOKE_RUN \
		-e ELF_GRAPHRAG_SMOKE_REPORT_DIR \
		-e ELF_GRAPHRAG_SMOKE_WORK_DIR \
		-e ELF_GRAPHRAG_SMOKE_INSTALL \
		-e ELF_GRAPHRAG_VERSION \
		-e ELF_GRAPHRAG_PACKAGE \
		-e ELF_GRAPHRAG_REF \
		-e ELF_GRAPHRAG_CHAT_MODEL \
		-e ELF_GRAPHRAG_EMBEDDING_MODEL \
		-e ELF_GRAPHRAG_API_BASE \
		-e ELF_GRAPHRAG_API_KEY \
		-e ELF_GRAPHRAG_INDEX_METHOD \
		-e ELF_GRAPHRAG_QUERY_METHOD \
		-e ELF_GRAPHRAG_TIMEOUT_SECONDS \
		-e ELF_GRAPHRAG_MAX_DOCS \
		-e ELF_GRAPHRAG_MAX_INPUT_CHARS \
		baseline-runner python3 scripts/graphrag-docker-smoke.py
	;;
lightrag-docker-context)
	start="$(printenv ELF_LIGHTRAG_CONTEXT_START || true)"
	status=0
	if [ "$start" = "1" ]; then
		docker compose -f docker-compose.baseline.yml --profile lightrag up -d lightrag
	fi
	docker compose -f docker-compose.baseline.yml run --build --rm \
		baseline-runner bash scripts/lightrag-docker-context-smoke.sh || status=$?
	if [ "$start" = "1" ]; then
		docker compose -f docker-compose.baseline.yml --profile lightrag stop lightrag lightrag-mock-provider >/dev/null 2>&1 || true
	fi
	exit "$status"
	;;
*)
	echo "unknown smoke: $smoke" >&2
	exit 2
	;;
esac
