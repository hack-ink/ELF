#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${ELF_RAGFLOW_SMOKE_ARTIFACT_DIR:-${ROOT_DIR}/tmp/real-world-memory/ragflow-smoke}"
OUT="${ELF_RAGFLOW_SMOKE_OUT:-${ARTIFACT_DIR}/ragflow-smoke.json}"
MANIFEST_OUT="${ELF_RAGFLOW_SMOKE_MANIFEST_OUT:-${ARTIFACT_DIR}/memory_projects_manifest.ragflow-smoke.json}"
SUMMARY_OUT="${ELF_RAGFLOW_SMOKE_SUMMARY_OUT:-${ARTIFACT_DIR}/summary.json}"
FIXTURE_DIR="${ELF_RAGFLOW_SMOKE_FIXTURE_DIR:-${ARTIFACT_DIR}/ragflow-fixtures}"
FIXTURE_PATH="${ELF_RAGFLOW_SMOKE_FIXTURE_PATH:-${FIXTURE_DIR}/retrieval/ragflow_evidence_smoke.json}"
REPORT_JSON="${ELF_RAGFLOW_SMOKE_REPORT_JSON:-${ARTIFACT_DIR}/ragflow-report.json}"
REPORT_MD="${ELF_RAGFLOW_SMOKE_REPORT_MD:-${ARTIFACT_DIR}/ragflow-report.md}"
SCORED_BENCHMARK="${ELF_RAGFLOW_SMOKE_SCORED_BENCHMARK:-${ARTIFACT_DIR}/scored-benchmark.json}"
WORK_DIR="${ELF_RAGFLOW_SMOKE_WORK_DIR:-${ARTIFACT_DIR}/work}"
RAGFLOW_REPO_URL="${ELF_RAGFLOW_REPO_URL:-https://github.com/infiniflow/ragflow.git}"
RAGFLOW_REF="${ELF_RAGFLOW_REF:-v0.25.6}"
RAGFLOW_IMAGE="${ELF_RAGFLOW_IMAGE:-infiniflow/ragflow:v0.25.6}"
COMPOSE_PROJECT="${ELF_RAGFLOW_COMPOSE_PROJECT:-elf-ragflow-smoke}"
START_RAGFLOW="${ELF_RAGFLOW_SMOKE_START:-0}"
ACCEPT_RESOURCE_ENVELOPE="${ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE:-0}"
ALLOW_ARM="${ELF_RAGFLOW_SMOKE_ALLOW_ARM:-0}"
PULL_IMAGE="${ELF_RAGFLOW_SMOKE_PULL_IMAGE:-0}"
CLEANUP="${ELF_RAGFLOW_SMOKE_CLEANUP:-1}"
CPU_GPU_MODE="${ELF_RAGFLOW_SMOKE_DEVICE:-cpu}"
API_PORT="${ELF_RAGFLOW_API_PORT:-19380}"
API_BASE="${ELF_RAGFLOW_API_BASE:-http://127.0.0.1:${API_PORT}}"
API_KEY="${ELF_RAGFLOW_API_KEY:-${RAGFLOW_API_KEY:-}}"
STARTUP_ATTEMPTS="${ELF_RAGFLOW_SMOKE_STARTUP_ATTEMPTS:-60}"
STARTUP_INTERVAL_SECONDS="${ELF_RAGFLOW_SMOKE_STARTUP_INTERVAL_SECONDS:-5}"
COMPOSE_TIMEOUT_SECONDS="${ELF_RAGFLOW_SMOKE_COMPOSE_TIMEOUT_SECONDS:-1800}"
RUN_ID="${ELF_RAGFLOW_SMOKE_RUN_ID:-ragflow-docker-smoke-$(date -u +%Y%m%d%H%M%S)}"
EVIDENCE_ID="ragflow-smoke-anchor"
DOCUMENT_NAME="${RUN_ID}.txt"
EVIDENCE_TOKEN="ELF_RAGFLOW_SMOKE_TOKEN_${RUN_ID}"
CORPUS_TEXT="RAGFlow smoke evidence ${EVIDENCE_TOKEN}: the ELF adapter maps returned reference chunks to the ragflow-smoke-anchor evidence id."

mkdir -p \
	"${ARTIFACT_DIR}" \
	"${WORK_DIR}" \
	"$(dirname "${OUT}")" \
	"$(dirname "${MANIFEST_OUT}")" \
	"$(dirname "${SUMMARY_OUT}")" \
	"$(dirname "${FIXTURE_PATH}")" \
	"$(dirname "${REPORT_JSON}")" \
	"$(dirname "${REPORT_MD}")" \
	"$(dirname "${SCORED_BENCHMARK}")"

rm -f "${OUT}" "${MANIFEST_OUT}" "${SUMMARY_OUT}" "${REPORT_JSON}" "${REPORT_MD}" "${SCORED_BENCHMARK}"

DOCKER_INFO="${ARTIFACT_DIR}/docker-info.json"
IMAGE_INSPECT="${ARTIFACT_DIR}/ragflow-image-inspect.json"
STARTUP_ATTEMPTS_JSONL="${ARTIFACT_DIR}/startup-attempts.jsonl"
DATASET_REQUEST="${ARTIFACT_DIR}/dataset-create-request.json"
DATASET_RESPONSE="${ARTIFACT_DIR}/dataset-create-response.json"
DOCUMENT_REQUEST="${ARTIFACT_DIR}/document-create-request.json"
DOCUMENT_RESPONSE="${ARTIFACT_DIR}/document-create-response.json"
CHUNK_REQUEST="${ARTIFACT_DIR}/chunk-create-request.json"
CHUNK_RESPONSE="${ARTIFACT_DIR}/chunk-create-response.json"
RETRIEVAL_REQUEST="${ARTIFACT_DIR}/retrieval-request.json"
RETRIEVAL_RESPONSE="${ARTIFACT_DIR}/retrieval-response.json"
REFERENCE_MAPPING="${ARTIFACT_DIR}/reference-mapping.json"
DOCKER_DF="${ARTIFACT_DIR}/docker-system-df.txt"
COMPOSE_UP_LOG="${ARTIFACT_DIR}/compose-up.log"
COMPOSE_DOWN_LOG="${ARTIFACT_DIR}/compose-down.log"

printf '[]\n' >"${IMAGE_INSPECT}"
printf '[]\n' >"${REFERENCE_MAPPING}"
for json_file in \
	"${DATASET_REQUEST}" \
	"${DATASET_RESPONSE}" \
	"${DOCUMENT_REQUEST}" \
	"${DOCUMENT_RESPONSE}" \
	"${CHUNK_REQUEST}" \
	"${CHUNK_RESPONSE}" \
	"${RETRIEVAL_REQUEST}" \
	"${RETRIEVAL_RESPONSE}"; do
	printf 'null\n' >"${json_file}"
done
: >"${STARTUP_ATTEMPTS_JSONL}"
: >"${DOCKER_DF}"
: >"${COMPOSE_UP_LOG}"
: >"${COMPOSE_DOWN_LOG}"

SETUP_STATUS="blocked"
RUN_STATUS="not_encoded"
RESULT_STATUS="blocked"
OVERALL_STATUS="blocked"
EVIDENCE_CLASS="research_gate"
FAILURE_CLASS="resource_confirmation_required"
FAILURE_REASON="RAGFlow startup is resource-heavy; set ELF_RAGFLOW_SMOKE_START=1 and ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE=1 to run the official Docker Compose stack."
STARTUP_TIME_MS=""
STARTED="false"
DATASET_ID=""
DOCUMENT_ID=""
CHUNK_ID=""
VM_MAX_MAP_COUNT=""
VM_MAX_MAP_COUNT_STATUS="not_observed"
VM_MAX_MAP_COUNT_ACTION="not_changed"
IMAGE_PRESENT="false"
IMAGE_SIZE_BYTES=""
HOST_GLOBAL_INSTALLS_REQUIRED="false"
DATASET_STEP_STATUS="not_encoded"
DOCUMENT_STEP_STATUS="not_encoded"
CHUNK_STEP_STATUS="not_encoded"
RETRIEVAL_STEP_STATUS="not_encoded"


source "${ROOT_DIR}/scripts/ragflow_smoke/common.sh"
source "${ROOT_DIR}/scripts/ragflow_smoke/docker.sh"
source "${ROOT_DIR}/scripts/ragflow_smoke/api.sh"
source "${ROOT_DIR}/scripts/ragflow_smoke/scoring.sh"
source "${ROOT_DIR}/scripts/ragflow_smoke/materialization.sh"
source "${ROOT_DIR}/scripts/ragflow_smoke/manifest.sh"
source "${ROOT_DIR}/scripts/ragflow_smoke/fixture.sh"
source "${ROOT_DIR}/scripts/ragflow_smoke/summary.sh"

for cmd in jq curl; do
	required_command "${cmd}"
done

if ! command -v docker >/dev/null 2>&1; then
	jq -n '{error: "docker_missing"}' >"${DOCKER_INFO}"
	SETUP_STATUS="incomplete"
	OVERALL_STATUS="incomplete"
	RESULT_STATUS="incomplete"
	FAILURE_CLASS="docker_cli_missing"
	FAILURE_REASON="Docker CLI is required for the RAGFlow evidence smoke."
	write_outputs
	exit 0
fi

if ! capture_docker_info; then
	SETUP_STATUS="incomplete"
	OVERALL_STATUS="incomplete"
	RESULT_STATUS="incomplete"
	FAILURE_CLASS="docker_unavailable"
	FAILURE_REASON="Docker is installed but docker info failed; RAGFlow Docker setup was not attempted."
	write_outputs
	exit 0
fi

capture_disk_info
capture_vm_max_map_count
capture_image_info

ARCH="$(uname -m)"
if [[ "${ARCH}" != "x86_64" && "${ARCH}" != "amd64" && "${ALLOW_ARM}" != "1" ]]; then
	SETUP_STATUS="blocked"
	OVERALL_STATUS="blocked"
	RESULT_STATUS="blocked"
	FAILURE_CLASS="unsupported_ragflow_docker_architecture"
	FAILURE_REASON="Official RAGFlow quickstart supports x86 CPU and Nvidia GPU Docker images; set ELF_RAGFLOW_SMOKE_ALLOW_ARM=1 only for an explicitly built ARM image path."
	write_outputs
	exit 0
fi

if [[ "${START_RAGFLOW}" != "1" ]]; then
	write_outputs
	exit 0
fi

if [[ "${ACCEPT_RESOURCE_ENVELOPE}" != "1" ]]; then
	write_outputs
	exit 0
fi

if ! command -v git >/dev/null 2>&1; then
	SETUP_STATUS="incomplete"
	OVERALL_STATUS="incomplete"
	RESULT_STATUS="incomplete"
	FAILURE_CLASS="git_missing_for_ragflow_source"
	FAILURE_REASON="git is required to fetch the official RAGFlow Docker Compose files for this smoke."
	write_outputs
	exit 0
fi

RAGFLOW_REPO_DIR=""
if RAGFLOW_REPO_DIR="$(prepare_official_ragflow_repo)"; then
	start_ragflow_stack "${RAGFLOW_REPO_DIR}"
else
	SETUP_STATUS="incomplete"
	OVERALL_STATUS="incomplete"
	RESULT_STATUS="incomplete"
	FAILURE_CLASS="ragflow_source_checkout_failed"
	FAILURE_REASON="Failed to fetch the official RAGFlow Docker Compose source."
fi

if [[ "${SETUP_STATUS}" == "pass" ]]; then
	if wait_for_ragflow_api; then
		if [[ -z "${API_KEY}" ]]; then
			RUN_STATUS="blocked"
			RESULT_STATUS="blocked"
			OVERALL_STATUS="blocked"
			FAILURE_CLASS="ragflow_api_key_required"
			FAILURE_REASON="RAGFlow HTTP APIs require a local self-host API key; no private or operator-owned provider credentials were used."
		else
			run_api_smoke
		fi
	else
		SETUP_STATUS="incomplete"
		RUN_STATUS="not_encoded"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_api_startup_timeout"
		FAILURE_REASON="RAGFlow Docker services started but the HTTP API did not become healthy within the configured retry window."
	fi
fi

cleanup_stack
write_outputs
