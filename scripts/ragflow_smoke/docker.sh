# RAGFlow Docker evidence smoke helper functions.
# Sourced by scripts/ragflow-docker-evidence-smoke.sh.

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
