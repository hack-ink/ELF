#!/usr/bin/env bash
set -euo pipefail

profile="${1:-}"
if [ -z "$profile" ]; then
	echo "usage: scripts/real-world-docker.sh <profile>" >&2
	exit 2
fi

case "$profile" in
job-operator-ux-live-adapters)
	docker compose -f docker-compose.baseline.yml run --build --rm \
		-e ELF_OPERATOR_DEBUG_LIVE_REPORT_DIR \
		-e ELF_OPERATOR_DEBUG_LIVE_FIXTURES \
		-e ELF_OPERATOR_DEBUG_LIVE_WORK_DIR \
		-e ELF_OPERATOR_DEBUG_QMD_DIR \
		baseline-runner bash scripts/real-world-operator-debug-live-adapters.sh
	;;
memory-live-consolidation)
	docker compose -f docker-compose.baseline.yml run --build --rm \
		-e ELF_CONSOLIDATION_LIVE_REPORT_DIR \
		-e ELF_CONSOLIDATION_LIVE_FIXTURES \
		baseline-runner bash scripts/real-world-consolidation-live-adapter.sh
	;;
memory-live-knowledge)
	docker compose -f docker-compose.baseline.yml run --build --rm \
		-e ELF_KNOWLEDGE_LIVE_REPORT_DIR \
		-e ELF_KNOWLEDGE_LIVE_FIXTURES \
		baseline-runner bash scripts/real-world-knowledge-live-adapter.sh
	;;
memory-service-native-dreaming)
	docker compose -f docker-compose.baseline.yml run --build --rm \
		-e ELF_DREAMING_SERVICE_NATIVE_REPORT_DIR \
		-e ELF_DREAMING_SERVICE_NATIVE_FIXTURES \
		baseline-runner bash scripts/real-world-dreaming-service-native.sh
	;;
memory-live-adapters)
	lightrag_start="$(printenv ELF_LIGHTRAG_CONTEXT_START || true)"
	graphiti_start="$(printenv ELF_GRAPHITI_ZEP_SMOKE_START || true)"
	status=0
	if [ "$lightrag_start" = "1" ]; then
		docker compose -f docker-compose.baseline.yml --profile lightrag up -d lightrag
	fi
	if [ "$graphiti_start" = "1" ]; then
		docker compose -f docker-compose.baseline.yml --profile graphiti-zep up -d graphiti-falkordb
	fi
	docker compose -f docker-compose.baseline.yml run --build --rm \
		-e ELF_REAL_WORLD_LIVE_ENABLE_RAGFLOW \
		-e ELF_REAL_WORLD_LIVE_ENABLE_LIGHTRAG \
		-e ELF_REAL_WORLD_LIVE_ENABLE_GRAPHRAG \
		-e ELF_REAL_WORLD_LIVE_ENABLE_GRAPHITI_ZEP \
		-e ELF_REAL_WORLD_LIVE_ENABLE_GRAPHIFY \
		-e ELF_RAGFLOW_SMOKE_START \
		-e ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE \
		-e ELF_RAGFLOW_SMOKE_ALLOW_ARM \
		-e ELF_RAGFLOW_SMOKE_PULL_IMAGE \
		-e ELF_RAGFLOW_SMOKE_CLEANUP \
		-e ELF_RAGFLOW_SMOKE_DEVICE \
		-e ELF_RAGFLOW_API_PORT \
		-e ELF_RAGFLOW_API_BASE \
		-e ELF_RAGFLOW_API_KEY \
		-e RAGFLOW_API_KEY \
		-e ELF_RAGFLOW_SMOKE_STARTUP_ATTEMPTS \
		-e ELF_RAGFLOW_SMOKE_STARTUP_INTERVAL_SECONDS \
		-e ELF_RAGFLOW_SMOKE_COMPOSE_TIMEOUT_SECONDS \
		-e ELF_RAGFLOW_REPO_URL \
		-e ELF_RAGFLOW_REF \
		-e ELF_RAGFLOW_IMAGE \
		-e ELF_RAGFLOW_COMPOSE_PROJECT \
		-e ELF_LIGHTRAG_CONTEXT_START \
		-e ELF_LIGHTRAG_API_BASE \
		-e ELF_LIGHTRAG_ADAPTER_ID \
		-e ELF_LIGHTRAG_ADAPTER_NAME \
		-e ELF_LIGHTRAG_STARTUP_ATTEMPTS \
		-e ELF_LIGHTRAG_STARTUP_INTERVAL_SECONDS \
		-e ELF_LIGHTRAG_INDEX_ATTEMPTS \
		-e ELF_LIGHTRAG_INDEX_INTERVAL_SECONDS \
		-e ELF_GRAPHRAG_SMOKE_RUN \
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
		-e ELF_GRAPHITI_ZEP_SMOKE_START \
		-e ELF_GRAPHITI_ZEP_SMOKE_RUN \
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
		-e ELF_GRAPHIFY_SMOKE_RUN \
		-e ELF_GRAPHIFY_SMOKE_WORK_DIR \
		-e ELF_GRAPHIFY_SMOKE_INSTALL \
		-e ELF_GRAPHIFY_PACKAGE \
		-e ELF_GRAPHIFY_REF \
		-e ELF_GRAPHIFY_TIMEOUT_SECONDS \
		-e ELF_GRAPHIFY_QUERY_BUDGET \
		baseline-runner bash scripts/real-world-live-adapters.sh || status=$?
	if [ "$lightrag_start" = "1" ]; then
		docker compose -f docker-compose.baseline.yml --profile lightrag stop lightrag lightrag-mock-provider >/dev/null 2>&1 || true
	fi
	if [ "$graphiti_start" = "1" ]; then
		docker compose -f docker-compose.baseline.yml --profile graphiti-zep stop graphiti-falkordb >/dev/null 2>&1 || true
	fi
	exit "$status"
	;;
*)
	echo "unknown real-world Docker profile: $profile" >&2
	exit 2
	;;
esac
