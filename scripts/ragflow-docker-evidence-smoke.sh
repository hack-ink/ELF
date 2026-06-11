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
	"$(dirname "${REPORT_MD}")"

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

required_command() {
	local cmd="$1"
	if ! command -v "${cmd}" >/dev/null 2>&1; then
		echo "Missing ${cmd}; cannot write RAGFlow smoke artifacts." >&2
		exit 1
	fi
}

optional_command_status() {
	local cmd="$1"
	if command -v "${cmd}" >/dev/null 2>&1; then
		printf 'available'
	else
		printf 'missing'
	fi
}

relative_path() {
	local path="$1"
	if [[ "${path}" == "${ROOT_DIR}/"* ]]; then
		printf '%s' "${path#"${ROOT_DIR}/"}"
	else
		printf '%s' "${path}"
	fi
}

json_status() {
	local status="$1"
	case "${status}" in
		real | mocked | unsupported | blocked | incomplete | wrong_result | lifecycle_fail | pass | not_encoded)
			printf '%s' "${status}"
			;;
		*)
			printf 'incomplete'
			;;
	esac
}

capture_docker_info() {
	if docker info --format '{{json .}}' >"${DOCKER_INFO}" 2>"${ARTIFACT_DIR}/docker-info.stderr"; then
		return 0
	fi

	jq -n --rawfile stderr "${ARTIFACT_DIR}/docker-info.stderr" '{
		error: "docker_info_failed",
		stderr: $stderr
	}' >"${DOCKER_INFO}"
	return 1
}

capture_disk_info() {
	docker system df >"${DOCKER_DF}" 2>/dev/null || true
}

capture_vm_max_map_count() {
	if VM_MAX_MAP_COUNT="$(sysctl -n vm.max_map_count 2>/dev/null)"; then
		if [[ "${VM_MAX_MAP_COUNT}" =~ ^[0-9]+$ ]] && [[ "${VM_MAX_MAP_COUNT}" -ge 262144 ]]; then
			VM_MAX_MAP_COUNT_STATUS="pass"
		elif [[ "${VM_MAX_MAP_COUNT}" =~ ^[0-9]+$ ]]; then
			VM_MAX_MAP_COUNT_STATUS="blocked"
		else
			VM_MAX_MAP_COUNT_STATUS="not_observed"
		fi
	else
		VM_MAX_MAP_COUNT=""
		VM_MAX_MAP_COUNT_STATUS="not_observed"
	fi
}

capture_image_info() {
	if [[ "${PULL_IMAGE}" == "1" && "${ACCEPT_RESOURCE_ENVELOPE}" == "1" ]]; then
		docker pull "${RAGFLOW_IMAGE}" >"${ARTIFACT_DIR}/docker-pull.log" 2>&1 || true
	fi

	if docker image inspect "${RAGFLOW_IMAGE}" >"${IMAGE_INSPECT}" 2>/dev/null; then
		IMAGE_PRESENT="true"
		IMAGE_SIZE_BYTES="$(jq -r '.[0].Size // ""' "${IMAGE_INSPECT}")"
	else
		printf '[]\n' >"${IMAGE_INSPECT}"
	fi
}

update_env_var() {
	local file="$1"
	local key="$2"
	local value="$3"

	if grep -q "^${key}=" "${file}"; then
		sed -i.bak "s|^${key}=.*|${key}=${value}|" "${file}"
	else
		printf '\n%s=%s\n' "${key}" "${value}" >>"${file}"
	fi
}

prepare_official_ragflow_repo() {
	local repo_dir="${WORK_DIR}/ragflow"

	if [[ ! -d "${repo_dir}/.git" ]]; then
		rm -rf "${repo_dir}"
		git clone --depth 1 --branch "${RAGFLOW_REF}" "${RAGFLOW_REPO_URL}" "${repo_dir}" \
			>"${ARTIFACT_DIR}/ragflow-git-clone.log" 2>&1
	else
		git -C "${repo_dir}" fetch --depth 1 origin "${RAGFLOW_REF}" \
			>"${ARTIFACT_DIR}/ragflow-git-fetch.log" 2>&1
		git -C "${repo_dir}" checkout -f FETCH_HEAD \
			>"${ARTIFACT_DIR}/ragflow-git-checkout.log" 2>&1
	fi

	update_env_var "${repo_dir}/docker/.env" "DEVICE" "${CPU_GPU_MODE}"
	update_env_var "${repo_dir}/docker/.env" "SVR_WEB_HTTP_PORT" "${ELF_RAGFLOW_WEB_HTTP_PORT:-18080}"
	update_env_var "${repo_dir}/docker/.env" "SVR_WEB_HTTPS_PORT" "${ELF_RAGFLOW_WEB_HTTPS_PORT:-18443}"
	update_env_var "${repo_dir}/docker/.env" "SVR_HTTP_PORT" "${API_PORT}"
	update_env_var "${repo_dir}/docker/.env" "ADMIN_SVR_HTTP_PORT" "${ELF_RAGFLOW_ADMIN_PORT:-19381}"
	update_env_var "${repo_dir}/docker/.env" "SVR_MCP_PORT" "${ELF_RAGFLOW_MCP_PORT:-19382}"
	update_env_var "${repo_dir}/docker/.env" "GO_HTTP_PORT" "${ELF_RAGFLOW_GO_HTTP_PORT:-19384}"
	update_env_var "${repo_dir}/docker/.env" "GO_ADMIN_PORT" "${ELF_RAGFLOW_GO_ADMIN_PORT:-19383}"
	update_env_var "${repo_dir}/docker/.env" "EXPOSE_MYSQL_PORT" "${ELF_RAGFLOW_MYSQL_PORT:-13306}"
	update_env_var "${repo_dir}/docker/.env" "MINIO_CONSOLE_PORT" "${ELF_RAGFLOW_MINIO_CONSOLE_PORT:-19001}"
	update_env_var "${repo_dir}/docker/.env" "MINIO_PORT" "${ELF_RAGFLOW_MINIO_PORT:-19000}"
	update_env_var "${repo_dir}/docker/.env" "REDIS_PORT" "${ELF_RAGFLOW_REDIS_PORT:-16379}"
	update_env_var "${repo_dir}/docker/.env" "ES_PORT" "${ELF_RAGFLOW_ES_PORT:-11200}"
	update_env_var "${repo_dir}/docker/.env" "OS_PORT" "${ELF_RAGFLOW_OS_PORT:-11201}"
	update_env_var "${repo_dir}/docker/.env" "RAGFLOW_IMAGE" "${RAGFLOW_IMAGE}"

	printf '%s' "${repo_dir}"
}

run_with_timeout_if_available() {
	local seconds="$1"
	shift

	if command -v timeout >/dev/null 2>&1; then
		timeout "${seconds}" "$@"
	else
		"$@"
	fi
}

start_ragflow_stack() {
	local repo_dir="$1"
	local started_at ended_at
	started_at="$(date +%s)"

	if (
		cd "${repo_dir}/docker"
		run_with_timeout_if_available "${COMPOSE_TIMEOUT_SECONDS}" \
			docker compose -p "${COMPOSE_PROJECT}" -f docker-compose.yml up -d
	) >"${COMPOSE_UP_LOG}" 2>&1; then
		STARTED="true"
		SETUP_STATUS="pass"
		FAILURE_CLASS=""
		FAILURE_REASON=""
	else
		SETUP_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		FAILURE_CLASS="ragflow_compose_start_failed"
		FAILURE_REASON="Official RAGFlow Docker Compose did not start successfully; see compose-up.log in the artifact directory."
	fi

	ended_at="$(date +%s)"
	STARTUP_TIME_MS="$(((ended_at - started_at) * 1000))"
}

wait_for_ragflow_api() {
	local attempt code

	for attempt in $(seq 1 "${STARTUP_ATTEMPTS}"); do
		code="$(curl -sS -o /dev/null -w '%{http_code}' "${API_BASE}/api/v1/system/healthz" 2>/dev/null || true)"
		jq -nc --argjson attempt "${attempt}" --arg code "${code}" --arg url "${API_BASE}/api/v1/system/healthz" '{
			attempt: $attempt,
			url: $url,
			http_code: $code
		}' >>"${STARTUP_ATTEMPTS_JSONL}"

		if [[ "${code}" == "200" ]]; then
			return 0
		fi

		sleep "${STARTUP_INTERVAL_SECONDS}"
	done

	return 1
}

api_json_request() {
	local method="$1"
	local path="$2"
	local request_file="$3"
	local response_file="$4"
	local stderr_file="${response_file}.stderr"
	local code

	code="$(curl -sS -X "${method}" \
		-o "${response_file}" \
		-w '%{http_code}' \
		-H 'Content-Type: application/json' \
		-H "Authorization: Bearer ${API_KEY}" \
		--data-binary @"${request_file}" \
		"${API_BASE}${path}" 2>"${stderr_file}" || true)"

	jq -n --arg code "${code}" --rawfile stderr "${stderr_file}" '{
		http_code: $code,
		stderr: $stderr
	}' >"${response_file}.meta.json"

	[[ "${code}" =~ ^2 ]]
}

response_code_ok() {
	local response_file="$1"

	jq -e '(.code? == 0) or (.id? != null) or (.data? != null)' "${response_file}" >/dev/null 2>&1
}

extract_id() {
	local response_file="$1"
	jq -r '
		.data.id
		// .data[0].id
		// .data.document_id
		// .data.chunk_id
		// .id
		// empty
	' "${response_file}"
}

run_api_smoke() {
	local dataset_name="${RUN_ID}"

	jq -n --arg name "${dataset_name}" '{
		name: $name,
		description: "Generated public ELF RAGFlow Docker evidence smoke corpus.",
		permission: "me",
		chunk_method: "manual",
		parser_config: {"raptor": {"use_raptor": false}}
	}' >"${DATASET_REQUEST}"

	if api_json_request POST "/api/v1/datasets" "${DATASET_REQUEST}" "${DATASET_RESPONSE}" \
		&& response_code_ok "${DATASET_RESPONSE}"; then
		DATASET_STEP_STATUS="pass"
		DATASET_ID="$(extract_id "${DATASET_RESPONSE}")"
	else
		DATASET_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_dataset_create_failed"
		FAILURE_REASON="RAGFlow dataset creation did not return a successful response."
		return 0
	fi

	if [[ -z "${DATASET_ID}" ]]; then
		DATASET_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_dataset_id_missing"
		FAILURE_REASON="RAGFlow dataset creation succeeded but no dataset id was found in the response."
		return 0
	fi

	jq -n --arg name "${DOCUMENT_NAME}" '{name: $name}' >"${DOCUMENT_REQUEST}"

	if api_json_request POST "/api/v1/datasets/${DATASET_ID}/documents?type=empty" \
		"${DOCUMENT_REQUEST}" "${DOCUMENT_RESPONSE}" \
		&& response_code_ok "${DOCUMENT_RESPONSE}"; then
		DOCUMENT_STEP_STATUS="pass"
		DOCUMENT_ID="$(extract_id "${DOCUMENT_RESPONSE}")"
	else
		DOCUMENT_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_document_create_failed"
		FAILURE_REASON="RAGFlow empty document creation did not return a successful response."
		return 0
	fi

	if [[ -z "${DOCUMENT_ID}" ]]; then
		DOCUMENT_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_document_id_missing"
		FAILURE_REASON="RAGFlow empty document creation succeeded but no document id was found in the response."
		return 0
	fi

	jq -n \
		--arg content "${CORPUS_TEXT}" \
		--arg token "${EVIDENCE_TOKEN}" \
		'{
			content: $content,
			important_keywords: [$token],
			questions: ["Which evidence token should map to ragflow-smoke-anchor?"]
		}' >"${CHUNK_REQUEST}"

	if api_json_request POST "/api/v1/datasets/${DATASET_ID}/documents/${DOCUMENT_ID}/chunks" \
		"${CHUNK_REQUEST}" "${CHUNK_RESPONSE}" \
		&& response_code_ok "${CHUNK_RESPONSE}"; then
		CHUNK_STEP_STATUS="pass"
		CHUNK_ID="$(extract_id "${CHUNK_RESPONSE}")"
	else
		CHUNK_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_chunk_create_failed"
		FAILURE_REASON="RAGFlow chunk creation did not return a successful response."
		return 0
	fi

	jq -n \
		--arg question "Which RAGFlow smoke evidence token maps to ragflow-smoke-anchor?" \
		--arg dataset_id "${DATASET_ID}" \
		--arg document_id "${DOCUMENT_ID}" \
		'{
			question: $question,
			dataset_ids: [$dataset_id],
			document_ids: [$document_id],
			page: 1,
			page_size: 5,
			similarity_threshold: 0.0,
			vector_similarity_weight: 0.0,
			top_k: 5,
			keyword: true,
			highlight: false
		}' >"${RETRIEVAL_REQUEST}"

	if api_json_request POST "/api/v1/retrieval" "${RETRIEVAL_REQUEST}" "${RETRIEVAL_RESPONSE}" \
		&& response_code_ok "${RETRIEVAL_RESPONSE}"; then
		RETRIEVAL_STEP_STATUS="pass"
	else
		RETRIEVAL_STEP_STATUS="incomplete"
		RUN_STATUS="incomplete"
		RESULT_STATUS="incomplete"
		OVERALL_STATUS="incomplete"
		FAILURE_CLASS="ragflow_retrieval_failed"
		FAILURE_REASON="RAGFlow retrieval did not return a successful response."
		return 0
	fi

	jq \
		--arg evidence_id "${EVIDENCE_ID}" \
		--arg token "${EVIDENCE_TOKEN}" \
		--arg document_name "${DOCUMENT_NAME}" '
		def chunk_array:
			if (.data.chunks? | type) == "array" then .data.chunks
			elif (.reference.chunks? | type) == "array" then .reference.chunks
			else [] end;
		chunk_array
		| map({
			chunk_id: (.id // .chunk_id // ""),
			content: (.content // .content_with_weight // ""),
			document_id: (.document_id // .doc_id // ""),
			document_name: (.document_name // .document_keyword // .doc_name // .docnm_kwd // ""),
			dataset_id: (.dataset_id // .kb_id // ""),
			positions: (.positions // []),
			similarity: (.similarity // null),
			vector_similarity: (.vector_similarity // null),
			term_similarity: (.term_similarity // null),
			evidence_ids: (
				if (((.content // .content_with_weight // "") | contains($token))
					or ((.document_name // .document_keyword // .doc_name // .docnm_kwd // "") == $document_name))
				then [$evidence_id]
				else []
				end
			),
			mapping_status: (
				if ((.content // .content_with_weight // "") | contains($token)) then "matched_content"
				elif ((.document_name // .document_keyword // .doc_name // .docnm_kwd // "") == $document_name) then "matched_document"
				else "unmatched"
				end
			)
		})' "${RETRIEVAL_RESPONSE}" >"${REFERENCE_MAPPING}"

	RUN_STATUS="pass"
	EVIDENCE_CLASS="live_real_world"

	if jq -e --arg evidence_id "${EVIDENCE_ID}" '
		length > 0 and any(.[]; (.evidence_ids // []) | index($evidence_id))
	' "${REFERENCE_MAPPING}" >/dev/null; then
		RESULT_STATUS="pass"
		OVERALL_STATUS="pass"
		FAILURE_CLASS=""
		FAILURE_REASON=""
	else
		RESULT_STATUS="wrong_result"
		OVERALL_STATUS="wrong_result"
		FAILURE_CLASS="ragflow_reference_mapping_missing"
		FAILURE_REASON="RAGFlow retrieval returned chunks but none mapped to the generated evidence id."
	fi
}

cleanup_stack() {
	local repo_dir="${WORK_DIR}/ragflow"

	if [[ "${STARTED}" != "true" || "${CLEANUP}" != "1" || ! -d "${repo_dir}/docker" ]]; then
		return 0
	fi

	(
		cd "${repo_dir}/docker"
		docker compose -p "${COMPOSE_PROJECT}" -f docker-compose.yml down -v
	) >"${COMPOSE_DOWN_LOG}" 2>&1 || true
}

write_artifact() {
	local generated_at out_rel manifest_rel fixture_rel report_json_rel report_md_rel docker_status git_status curl_status jq_status
	generated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
	out_rel="$(relative_path "${OUT}")"
	manifest_rel="$(relative_path "${MANIFEST_OUT}")"
	fixture_rel="$(relative_path "${FIXTURE_PATH}")"
	report_json_rel="$(relative_path "${REPORT_JSON}")"
	report_md_rel="$(relative_path "${REPORT_MD}")"
	docker_status="$(optional_command_status docker)"
	git_status="$(optional_command_status git)"
	curl_status="$(optional_command_status curl)"
	jq_status="$(optional_command_status jq)"

	jq -n \
		--arg schema "elf.ragflow_docker_evidence_smoke/v1" \
		--arg run_id "${RUN_ID}" \
		--arg generated_at "${generated_at}" \
		--arg adapter_id "ragflow_docker_evidence_smoke" \
		--arg evidence_class "${EVIDENCE_CLASS}" \
		--arg overall_status "$(json_status "${OVERALL_STATUS}")" \
		--arg setup_status "$(json_status "${SETUP_STATUS}")" \
		--arg run_status "$(json_status "${RUN_STATUS}")" \
		--arg result_status "$(json_status "${RESULT_STATUS}")" \
		--arg failure_class "${FAILURE_CLASS}" \
		--arg failure_reason "${FAILURE_REASON}" \
		--arg out_rel "${out_rel}" \
		--arg manifest_rel "${manifest_rel}" \
		--arg fixture_rel "${fixture_rel}" \
		--arg report_json_rel "${report_json_rel}" \
		--arg report_md_rel "${report_md_rel}" \
		--arg artifact_dir "$(relative_path "${ARTIFACT_DIR}")" \
		--arg work_dir "$(relative_path "${WORK_DIR}")" \
		--arg repo_url "${RAGFLOW_REPO_URL}" \
		--arg ragflow_ref "${RAGFLOW_REF}" \
		--arg ragflow_image "${RAGFLOW_IMAGE}" \
		--arg compose_project "${COMPOSE_PROJECT}" \
		--arg cpu_gpu_mode "${CPU_GPU_MODE}" \
		--arg start_enabled "${START_RAGFLOW}" \
		--arg accept_resource_envelope "${ACCEPT_RESOURCE_ENVELOPE}" \
		--arg allow_arm "${ALLOW_ARM}" \
		--arg pull_image "${PULL_IMAGE}" \
		--arg cleanup "${CLEANUP}" \
		--arg api_base "${API_BASE}" \
		--arg api_key_provided "$([[ -n "${API_KEY}" ]] && printf true || printf false)" \
		--arg startup_time_ms "${STARTUP_TIME_MS}" \
		--arg started "${STARTED}" \
		--arg startup_attempt_count "${STARTUP_ATTEMPTS}" \
		--arg startup_interval_seconds "${STARTUP_INTERVAL_SECONDS}" \
		--arg compose_timeout_seconds "${COMPOSE_TIMEOUT_SECONDS}" \
		--arg evidence_id "${EVIDENCE_ID}" \
		--arg document_name "${DOCUMENT_NAME}" \
		--arg evidence_token "${EVIDENCE_TOKEN}" \
		--arg corpus_text "${CORPUS_TEXT}" \
		--arg dataset_id "${DATASET_ID}" \
		--arg document_id "${DOCUMENT_ID}" \
		--arg chunk_id "${CHUNK_ID}" \
		--arg vm_max_map_count "${VM_MAX_MAP_COUNT}" \
		--arg vm_max_map_count_status "${VM_MAX_MAP_COUNT_STATUS}" \
		--arg vm_max_map_count_action "${VM_MAX_MAP_COUNT_ACTION}" \
		--arg image_present "${IMAGE_PRESENT}" \
		--arg image_size_bytes "${IMAGE_SIZE_BYTES}" \
		--arg host_global_installs_required "${HOST_GLOBAL_INSTALLS_REQUIRED}" \
		--arg docker_status "${docker_status}" \
		--arg git_status "${git_status}" \
		--arg curl_status "${curl_status}" \
		--arg jq_status "${jq_status}" \
		--arg dataset_step_status "$(json_status "${DATASET_STEP_STATUS}")" \
		--arg document_step_status "$(json_status "${DOCUMENT_STEP_STATUS}")" \
		--arg chunk_step_status "$(json_status "${CHUNK_STEP_STATUS}")" \
		--arg retrieval_step_status "$(json_status "${RETRIEVAL_STEP_STATUS}")" \
		--slurpfile docker_info "${DOCKER_INFO}" \
		--slurpfile image_inspect "${IMAGE_INSPECT}" \
		--slurpfile reference_mapping "${REFERENCE_MAPPING}" \
		--rawfile docker_df "${DOCKER_DF}" \
		--rawfile compose_up_log "${COMPOSE_UP_LOG}" \
		--rawfile compose_down_log "${COMPOSE_DOWN_LOG}" \
		--slurpfile dataset_response "${DATASET_RESPONSE}" \
		--slurpfile document_response "${DOCUMENT_RESPONSE}" \
		--slurpfile chunk_response "${CHUNK_RESPONSE}" \
		--slurpfile retrieval_response "${RETRIEVAL_RESPONSE}" \
		--slurpfile startup_attempts <(jq -s '.' "${STARTUP_ATTEMPTS_JSONL}") \
		'{
			schema: $schema,
			run_id: $run_id,
			generated_at: $generated_at,
			adapter_id: $adapter_id,
			evidence_class: $evidence_class,
			overall_status: $overall_status,
			no_quality_claim: true,
			failure: (
				if $failure_class == "" then null
				else {
					class: $failure_class,
					reason: $failure_reason
				}
				end
			),
			artifacts: {
				smoke: $out_rel,
				external_adapter_manifest: $manifest_rel,
				generated_fixture: $fixture_rel,
				scored_report_json: $report_json_rel,
				scored_report_markdown: $report_md_rel,
				artifact_dir: $artifact_dir,
				work_dir: $work_dir
			},
			upstream: {
				repository: $repo_url,
				ref: $ragflow_ref,
				quickstart: "https://ragflow.io/docs/",
				http_api_reference: "https://raw.githubusercontent.com/infiniflow/ragflow/main/docs/references/http_api_reference.md",
				api_key_guide: "https://ragflow.io/docs/acquire_ragflow_api_key"
			},
			docker_boundary: {
				status: $setup_status,
				official_compose_path: "ragflow/docker/docker-compose.yml",
				compose_project: $compose_project,
				image: $ragflow_image,
				device: $cpu_gpu_mode,
				start_enabled: ($start_enabled == "1"),
				resource_envelope_accepted: ($accept_resource_envelope == "1"),
				allow_arm: ($allow_arm == "1"),
				pull_image_requested: ($pull_image == "1"),
				cleanup_requested: ($cleanup == "1"),
				host_global_installs_required: ($host_global_installs_required == "true"),
				tooling: {
					docker: $docker_status,
					git: $git_status,
					curl: $curl_status,
					jq: $jq_status
				}
			},
			setup: {
				status: $setup_status,
				command: "cargo make ragflow-docker-smoke",
				live_command: "ELF_RAGFLOW_SMOKE_START=1 ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE=1 cargo make ragflow-docker-smoke",
				started: ($started == "true"),
				startup_time_ms: (if $startup_time_ms == "" then null else ($startup_time_ms | tonumber) end),
				vm_max_map_count: {
					status: $vm_max_map_count_status,
					observed: (if $vm_max_map_count == "" then null else $vm_max_map_count end),
					required_min: 262144,
					action: $vm_max_map_count_action
				},
				image: {
					present: ($image_present == "true"),
					size_bytes: (if $image_size_bytes == "" then null else ($image_size_bytes | tonumber) end),
					official_compressed_size_note: "RAGFlow quickstart lists the stable image at about 2 GB compressed.",
					official_expanded_size_note: "RAGFlow quickstart says the image expands to about 7 GB once unpacked.",
					inspect: ($image_inspect[0] // [])
				},
				resource_envelope: {
					official_min_cpu_cores: 4,
					official_min_ram_gb: 16,
					official_min_disk_gb: 50,
					docker_info: ($docker_info[0] // {}),
					docker_system_df: $docker_df
				},
				provider_boundaries: {
					ragflow_api_base: $api_base,
					ragflow_api_key_provided: ($api_key_provided == "true"),
					operator_owned_provider_credentials_used: false,
					private_corpus_used: false,
					generated_public_corpus_only: true,
					external_llm_quality_scoring_claimed: false
				},
				retry_behavior: {
					startup_poll_attempts_configured: ($startup_attempt_count | tonumber),
					startup_interval_seconds: ($startup_interval_seconds | tonumber),
					compose_timeout_seconds: ($compose_timeout_seconds | tonumber),
					startup_attempts: ($startup_attempts[0] // [])
				},
				log_excerpt: {
					compose_up: ($compose_up_log | split("\n") | .[0:40]),
					compose_down: ($compose_down_log | split("\n") | .[0:20])
				}
			},
			corpus: {
				profile: "generated_public",
				evidence_id: $evidence_id,
				document_name: $document_name,
				evidence_token: $evidence_token,
				text: $corpus_text,
				dataset_id: (if $dataset_id == "" then null else $dataset_id end),
				document_id: (if $document_id == "" then null else $document_id end),
				chunk_id: (if $chunk_id == "" then null else $chunk_id end)
			},
			run: {
				status: $run_status,
				steps: {
					dataset_creation: {
						status: $dataset_step_status,
						request_artifact: "dataset-create-request.json",
						response_artifact: "dataset-create-response.json",
						response: ($dataset_response[0] // null)
					},
					document_creation: {
						status: $document_step_status,
						request_artifact: "document-create-request.json",
						response_artifact: "document-create-response.json",
						response: ($document_response[0] // null)
					},
					chunk_ingest: {
						status: $chunk_step_status,
						request_artifact: "chunk-create-request.json",
						response_artifact: "chunk-create-response.json",
						response: ($chunk_response[0] // null)
					},
					retrieval_query: {
						status: $retrieval_step_status,
						request_artifact: "retrieval-request.json",
						response_artifact: "retrieval-response.json",
						response: ($retrieval_response[0] // null)
					}
				}
			},
			result: {
				status: $result_status,
				evidence: "RAGFlow retrieval reference chunks are mapped to real_world_job evidence ids when content or document metadata matches the generated public corpus.",
				reference_chunk_count: (($reference_mapping[0] // []) | length),
				mapped_reference_chunk_count: (($reference_mapping[0] // []) | map(select((.evidence_ids // []) | length > 0)) | length)
			},
			evidence_mapping: {
				expected_evidence_ids: [$evidence_id],
				reference_chunks: ($reference_mapping[0] // []),
				field_mapping: {
					"id": "chunk_id",
					"document_id": "document_id",
					"document_name_or_document_keyword": "document_name",
					"dataset_id_or_kb_id": "dataset_id",
					"content_or_content_with_weight": "content",
					"positions": "positions",
					"similarity": "similarity",
					"vector_similarity": "vector_similarity",
					"term_similarity": "term_similarity"
				}
			}
		}' >"${OUT}"
}

write_manifest() {
	local generated_at out_rel manifest_rel retrieval_suite_status production_ops_status capability_retrieval_status capability_setup_status
	generated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
	out_rel="$(relative_path "${OUT}")"
	manifest_rel="$(relative_path "${MANIFEST_OUT}")"
	retrieval_suite_status="$(json_status "${RESULT_STATUS}")"
	capability_retrieval_status="$(json_status "${RESULT_STATUS}")"
	capability_setup_status="$(json_status "${SETUP_STATUS}")"
	production_ops_status="not_encoded"

	jq -n \
		--arg generated_at "${generated_at}" \
		--arg manifest_id "ragflow-docker-evidence-smoke-${RUN_ID}" \
		--arg out_rel "${out_rel}" \
		--arg manifest_rel "${manifest_rel}" \
		--arg evidence_class "${EVIDENCE_CLASS}" \
		--arg overall_status "$(json_status "${OVERALL_STATUS}")" \
		--arg setup_status "$(json_status "${SETUP_STATUS}")" \
		--arg run_status "$(json_status "${RUN_STATUS}")" \
		--arg result_status "$(json_status "${RESULT_STATUS}")" \
		--arg retrieval_suite_status "${retrieval_suite_status}" \
		--arg production_ops_status "${production_ops_status}" \
		--arg capability_setup_status "${capability_setup_status}" \
		--arg capability_retrieval_status "${capability_retrieval_status}" \
		--arg ragflow_image "${RAGFLOW_IMAGE}" \
		--arg cpu_gpu_mode "${CPU_GPU_MODE}" \
		--arg failure_reason "${FAILURE_REASON}" \
		--arg host_global_installs_required "${HOST_GLOBAL_INSTALLS_REQUIRED}" \
		'{
			schema: "elf.real_world_external_adapter_manifest/v1",
			manifest_id: $manifest_id,
			docker_isolation: {
				default: true,
				compose_file: "official RAGFlow docker/docker-compose.yml",
				runner: "scripts/ragflow-docker-evidence-smoke.sh",
				artifact_dir: "tmp/real-world-memory/ragflow-smoke",
				host_global_installs_required: ($host_global_installs_required == "true"),
				notes: [
					"Generated by the RAGFlow evidence-smoke script at " + $generated_at + ".",
					"The smoke uses a generated public corpus and does not use private corpus or operator-owned provider credentials."
				]
			},
			adapters: [
				{
					adapter_id: "ragflow_docker_evidence_smoke",
					project: "RAGFlow",
					adapter_kind: "docker_service_evidence_smoke",
					evidence_class: $evidence_class,
					docker_default: true,
					host_global_installs_required: ($host_global_installs_required == "true"),
					overall_status: $overall_status,
					setup: {
						status: $setup_status,
						evidence: "Official RAGFlow Docker Compose boundary and resource envelope were evaluated for the tiny evidence smoke.",
						command: "cargo make ragflow-docker-smoke",
						artifact: $out_rel
					},
					run: {
						status: $run_status,
						evidence: "The smoke attempts dataset creation, empty-document corpus ingest, chunk insert, retrieval query, and reference chunk extraction.",
						command: "ELF_RAGFLOW_SMOKE_START=1 ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE=1 cargo make ragflow-docker-smoke",
						artifact: $out_rel
					},
					result: {
						status: $result_status,
						evidence: (
							if $failure_reason == "" then "Returned RAGFlow reference chunks were mapped to generated real_world_job evidence ids for the smoke only."
							else $failure_reason
							end
						),
						artifact: $out_rel
					},
					capabilities: [
						{
							capability: "official_docker_service_boundary",
							status: $capability_setup_status,
							evidence: "The script uses the official RAGFlow Docker Compose setup and records image, disk, startup, CPU/GPU, and vm.max_map_count evidence."
						},
						{
							capability: "dataset_or_chunk_ingest",
							status: $run_status,
							evidence: "The live path creates a generated public dataset, empty document, and chunk before querying."
						},
						{
							capability: "retrieval_reference_mapping",
							status: $capability_retrieval_status,
							evidence: "The script maps returned chunk id, document id, document name, dataset id, positions, and similarity fields to benchmark evidence ids."
						},
						{
							capability: "quality_or_scale_claim",
							status: "not_encoded",
							evidence: "The smoke does not run broad RAGFlow quality scoring, scale tests, private corpora, or comparative ranking claims."
						}
					],
					suites: [
						{
							suite_id: "retrieval",
							status: $retrieval_suite_status,
							evidence: "Only the generated-public RAGFlow evidence-smoke retrieval path is represented."
						},
						{
							suite_id: "production_ops",
							status: $production_ops_status,
							evidence: "Resource envelope evidence is recorded, but no production-ops suite scoring is encoded."
						},
						{
							suite_id: "knowledge_compilation",
							status: "not_encoded",
							evidence: "RAGFlow page or knowledge-compilation behavior is not part of this smoke."
						}
					],
					evidence: [
						{
							kind: "artifact",
							ref: $out_rel,
							status: $result_status
						},
						{
							kind: "manifest",
							ref: $manifest_rel,
							status: $overall_status
						},
						{
							kind: "source",
							ref: "https://ragflow.io/docs/",
							status: "real"
						},
						{
							kind: "source",
							ref: "https://raw.githubusercontent.com/infiniflow/ragflow/main/docs/references/http_api_reference.md",
							status: "real"
						}
					],
					execution_metadata: {
						sources: [
							{
								label: "RAGFlow quickstart",
								url: "https://ragflow.io/docs/",
								evidence: "Official Docker startup, resource envelope, vm.max_map_count, and provider configuration guidance."
							},
							{
								label: "RAGFlow HTTP API reference",
								url: "https://raw.githubusercontent.com/infiniflow/ragflow/main/docs/references/http_api_reference.md",
								evidence: "Official dataset, document, chunk, retrieval, and reference-chunk field contract."
							}
						],
						setup_path: "Run the official RAGFlow Docker Compose stack with generated public corpus only.",
						runtime_boundary: "Official RAGFlow Docker Compose service boundary; no host-global RAGFlow install.",
						resource_expectation: (
							"RAGFlow image " + $ragflow_image + ", CPU/GPU mode " + $cpu_gpu_mode + ", official minimums 4 CPU cores, 16 GB RAM, 50 GB disk, and vm.max_map_count >= 262144."
						),
						retry_guidance: [
							"Default command records a typed blocked preflight unless resource-heavy startup is explicitly enabled.",
							"Set ELF_RAGFLOW_SMOKE_START=1 and ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE=1 for a live Docker startup attempt.",
							"Provide only a local self-hosted RAGFlow API key; do not use private corpora or operator-owned model provider credentials for this smoke."
						],
						research_depth: "D2 feasibility plus XY-885 evidence-smoke implementation; generated artifact decides live evidence class."
					},
					notes: [
						"This adapter record is generated by a smoke artifact and must not be generalized into broad RAGFlow quality evidence.",
						"Failure before query output remains typed as blocked, incomplete, or not_encoded."
					]
				}
			]
		}' >"${MANIFEST_OUT}"
}

write_fixture() {
	local result_status reason
	result_status="$(json_status "${RESULT_STATUS}")"
	reason="${FAILURE_REASON}"

	jq -n \
		--arg run_id "${RUN_ID}" \
		--arg evidence_id "${EVIDENCE_ID}" \
		--arg evidence_token "${EVIDENCE_TOKEN}" \
		--arg corpus_text "${CORPUS_TEXT}" \
		--arg result_status "${result_status}" \
		--arg failure_reason "${reason}" \
		'{
			schema: "elf.real_world_job/v1",
			job_id: "ragflow-evidence-smoke-001",
			suite: "retrieval",
			title: "Map RAGFlow reference chunks to generated evidence",
			corpus: {
				corpus_id: "ragflow-generated-public-smoke",
				profile: "generated_public",
				items: [
					{
						evidence_id: $evidence_id,
						kind: "document",
						text: $corpus_text,
						source_ref: {
							schema: "source_ref/v1",
							resolver: "ragflow_smoke/v1",
							ref: {
								run_id: $run_id,
								evidence_token: $evidence_token
							}
						},
						created_at: "2026-06-10T00:00:00Z"
					}
				],
				adapter_response: {
					adapter_id: "ragflow_docker_evidence_smoke",
					answer: {
						content: (
							if $result_status == "pass" then
								"RAGFlow returned reference chunks that map to the generated ragflow-smoke-anchor evidence id."
							else
								""
							end
						),
						claims: (
							if $result_status == "pass" then
								[
									{
										claim_id: "ragflow_reference_mapping",
										text: "RAGFlow reference chunks map to the generated ragflow-smoke-anchor evidence id.",
										evidence_ids: [$evidence_id],
										confidence: "derived_from_ragflow_reference_chunk_mapping"
									}
								]
							else
								[]
							end
						),
						evidence_ids: (if $result_status == "pass" then [$evidence_id] else [] end),
						latency_ms: 0.0,
						cost: {
							currency: "USD",
							amount: 0.0,
							input_tokens: 0,
							output_tokens: 0
						}
					}
				}
			},
			timeline: [
				{
					event_id: "ragflow-smoke-corpus-generated",
					ts: "2026-06-10T00:00:00Z",
					actor: "system",
					action: "generated_public_corpus",
					evidence_ids: [$evidence_id],
					summary: "The RAGFlow smoke generated a tiny public corpus for reference chunk mapping."
				}
			],
			prompt: {
				role: "user",
				content: "Which RAGFlow smoke evidence token maps to the generated reference chunk?",
				job_mode: "answer",
				constraints: ["cite_evidence", "avoid_broad_quality_claims"]
			},
			expected_answer: {
				must_include: [
					{
						claim_id: "ragflow_reference_mapping",
						text: "RAGFlow reference chunks map to the generated ragflow-smoke-anchor evidence id."
					}
				],
				must_not_include: ["RAGFlow passed a broad graph/RAG quality benchmark."],
				evidence_links: {
					ragflow_reference_mapping: [$evidence_id]
				},
				answer_type: "direct_answer",
				accepted_alternates: [],
				requires_caveat: true,
				requires_refusal: false
			},
			required_evidence: [
				{
					evidence_id: $evidence_id,
					claim_id: "ragflow_reference_mapping",
					requirement: "cite",
					quote: "ragflow-smoke-anchor evidence id"
				}
			],
			negative_traps: [],
			scoring_rubric: {
				dimensions: {
					answer_correctness: {
						weight: 0.3,
						max_points: 1.0,
						criteria: "States the generated evidence mapping without broad quality claims."
					},
					evidence_grounding: {
						weight: 0.45,
						max_points: 1.0,
						criteria: "Maps returned RAGFlow reference chunks to the generated evidence id."
					},
					trap_avoidance: {
						weight: 0.15,
						max_points: 1.0,
						criteria: "Does not claim broad RAGFlow quality from the tiny smoke."
					},
					latency_resource: {
						weight: 0.1,
						max_points: 1.0,
						criteria: "Records setup, resource, provider, and reference-mapping boundaries."
					}
				},
				pass_threshold: 0.75,
				hard_fail_rules: []
			},
			allowed_uncertainty: {
				can_answer_unknown: false,
				acceptable_phrases: ["tiny generated corpus", "reference chunk smoke only"],
				fallback_action: "state_blocker"
			},
			operator_debug: null,
			encoding: {},
			memory_evolution: null,
			tags: ["external_adapter", "generated_public", "ragflow", "no_live_claim"]
		}
		| if ["blocked", "incomplete", "not_encoded"] | index($result_status) then
			.encoding = {status: $result_status, reason: $failure_reason}
		else
			.
		end' >"${FIXTURE_PATH}"
}

write_scored_report() {
	(
		cd "${ROOT_DIR}"
		cargo run -p elf-eval --bin real_world_job_benchmark -- run \
			--fixtures "${FIXTURE_PATH}" \
			--out "${REPORT_JSON}" \
			--run-id real-world-memory-live-ragflow \
			--adapter-id ragflow_docker_evidence_smoke \
			--adapter-name "RAGFlow Docker evidence smoke adapter" \
			--adapter-behavior docker_service_evidence_smoke \
			--adapter-storage-status "$(json_status "${SETUP_STATUS}")" \
			--adapter-runtime-status "$(json_status "${OVERALL_STATUS}")" \
			--adapter-notes "Generated by the RAGFlow Docker evidence smoke; pass or wrong_result requires reference chunks mapped to generated evidence ids, while resource/setup/API-key limits remain typed." \
			--external-adapter-manifest "${MANIFEST_OUT}"
		cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
			--report "${REPORT_JSON}" \
			--out "${REPORT_MD}"
	)
}

write_summary() {
	jq -n \
		--slurpfile materialization "${OUT}" \
		--slurpfile manifest "${MANIFEST_OUT}" \
		--slurpfile report "${REPORT_JSON}" \
		'{
			schema: "elf.ragflow_docker_smoke_summary/v1",
			generated_at: (now | todateiso8601),
			adapter_id: "ragflow_docker_evidence_smoke",
			evidence_class: $materialization[0].evidence_class,
			materialization: $materialization[0],
			manifest: {
				json: ($materialization[0].artifacts.external_adapter_manifest // "tmp/real-world-memory/ragflow-smoke/memory_projects_manifest.ragflow-smoke.json"),
				summary: $manifest[0].adapters[0].overall_status,
				suites: $manifest[0].adapters[0].suites
			},
			report: {
				json: ($materialization[0].artifacts.scored_report_json // "tmp/real-world-memory/ragflow-smoke/ragflow-report.json"),
				markdown: ($materialization[0].artifacts.scored_report_markdown // "tmp/real-world-memory/ragflow-smoke/ragflow-report.md"),
				summary: $report[0].summary,
				suites: $report[0].suites
			}
		}' >"${SUMMARY_OUT}"
}

write_outputs() {
	write_artifact
	write_manifest
	write_fixture
	write_scored_report
	write_summary
	echo "RAGFlow smoke artifact: ${OUT}"
	echo "RAGFlow smoke manifest: ${MANIFEST_OUT}"
	echo "RAGFlow smoke report: ${REPORT_JSON}"
	echo "RAGFlow smoke summary: ${SUMMARY_OUT}"
}

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
