# RAGFlow Docker evidence smoke helper functions.
# Sourced by scripts/ragflow-docker-evidence-smoke.sh.

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
