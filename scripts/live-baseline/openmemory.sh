probe_mem0_openmemory_ui_export() {
  local project_repo="$1"
  local sdk_result_path="$2"
  local out_path="$3"
  local log_path="$4"
  local openmemory_dir="${project_repo}/openmemory"
  local export_script="${openmemory_dir}/backup-scripts/export_openmemory.sh"
  local ui_package="${openmemory_dir}/ui/package.json"
  local compose_file="${openmemory_dir}/docker-compose.yml"
  local readme_path="${openmemory_dir}/README.md"
  local run_script="${openmemory_dir}/run.sh"
  local api_env_example="${openmemory_dir}/api/.env.example"
  local attempt_log="${REPORT_DIR}/mem0-openmemory-export-attempt.log"
  local validation_path="${REPORT_DIR}/mem0-openmemory-export-validation.json"
  local export_user_id="${ELF_MEM0_OPENMEMORY_EXPORT_USER_ID:-elf-history-user}"
  local export_container="${ELF_MEM0_OPENMEMORY_EXPORT_CONTAINER:-openmemory-openmemory-mcp-1}"
  local export_zip="${project_repo}/memories_export_${export_user_id}.zip"
  local command_display="timeout 30 bash openmemory/backup-scripts/export_openmemory.sh --user-id ${export_user_id} --container ${export_container}"
  local sdk_get_all_status
  local export_exit_code=0
  local openmemory_tree_present=false
  local ui_package_present=false
  local compose_present=false
  local export_script_present=false
  local sunsetting_notice_present=false
  local requires_api_key=false
  local requires_docker_compose=false
  local export_requires_running_container=false
  local status="blocked"
  local comparison_outcome="blocked"
  local reason_code="OPENMEMORY_CONTAINER_NOT_RUNNING"
  local reason="OpenMemory export-helper setup probe could not run because no OpenMemory product container is available in the Docker baseline runner."
  local next_action="Add a dedicated OpenMemory Docker Compose profile that imports the generated mem0 corpus into the OpenMemory app database, starts the API/UI with explicit local or provider configuration, then rerun the export helper and validate the exported memories."
  local output_excerpt=""
  local validation_json="{}"

  sdk_get_all_status="$(jq -r '[.checks[]? | select(.name == "local_get_all_export_readback") | .status][0] // "missing"' "${sdk_result_path}" 2>/dev/null || echo "missing")"

  [[ -d "${openmemory_dir}" ]] && openmemory_tree_present=true
  [[ -f "${ui_package}" ]] && ui_package_present=true
  [[ -f "${compose_file}" ]] && compose_present=true
  [[ -f "${export_script}" ]] && export_script_present=true
  if [[ -f "${readme_path}" ]] && grep -qi "sunsetting notice" "${readme_path}"; then
    sunsetting_notice_present=true
  fi
  if grep -q "OPENAI_API_KEY" "${run_script}" "${api_env_example}" 2>/dev/null; then
    requires_api_key=true
  fi
  if [[ -f "${run_script}" ]] && grep -q "docker compose" "${run_script}"; then
    requires_docker_compose=true
  fi
  if [[ -f "${export_script}" ]] && grep -q "docker ps" "${export_script}"; then
    export_requires_running_container=true
  fi

  : >"${attempt_log}"
  rm -f "${validation_path}" "${export_zip}"
  if [[ "${openmemory_tree_present}" != "true" ]]; then
    status="unsupported"
    reason_code="OPENMEMORY_TREE_MISSING"
    reason="The cloned mem0 repository does not contain the OpenMemory product tree, so no export-helper setup probe path is available in this revision."
  elif [[ "${export_script_present}" != "true" ]]; then
    status="unsupported"
    reason_code="OPENMEMORY_EXPORT_SCRIPT_MISSING"
    reason="The OpenMemory tree is present, but its export helper is missing, so the runner cannot attempt export-helper setup readback."
  else
    set +e
    (
      cd "${project_repo}"
      timeout 30 bash openmemory/backup-scripts/export_openmemory.sh \
        --user-id "${export_user_id}" \
        --container "${export_container}"
    ) >"${attempt_log}" 2>&1
    export_exit_code=$?
    set -e
    output_excerpt="$(head -c 4000 "${attempt_log}" || true)"

    if [[ "${export_exit_code}" -eq 0 && -s "${export_zip}" ]]; then
      python3 - "${export_zip}" "${validation_path}" <<'PY'
import json
import sys
import zipfile
from pathlib import Path

zip_path = Path(sys.argv[1])
out_path = Path(sys.argv[2])
result = {
    "zip_present": zip_path.is_file(),
    "zip_path": str(zip_path),
    "memories_json_present": False,
    "has_current_preference": False,
    "omits_other_scope": False,
    "error": None,
}

try:
    with zipfile.ZipFile(zip_path) as archive:
        result["members"] = archive.namelist()
        if "memories.json" in archive.namelist():
            result["memories_json_present"] = True
            payload = archive.read("memories.json").decode("utf-8", "replace")
            lowered = payload.lower()
            result["has_current_preference"] = (
                "concise" in lowered and "evidence-linked" in lowered
            )
            result["omits_other_scope"] = "long-form chinese" not in lowered
except Exception as exc:
    result["error"] = repr(exc)

out_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
PY
      validation_json="$(cat "${validation_path}")"
      if jq -e '.has_current_preference == true and .omits_other_scope == true' "${validation_path}" >/dev/null; then
        status="pass"
        reason_code="OPENMEMORY_EXPORT_READBACK_MATCHED"
        reason="OpenMemory export produced a zip containing the current scoped preference and omitting the other scope."
        next_action="Keep OpenMemory export-helper readback as a separate product-UX scenario from SDK get_all and rerun after any OpenMemory setup change."
      else
        status="blocked"
        reason_code="OPENMEMORY_EXPORT_MISSING_SAME_CORPUS"
        reason="OpenMemory export ran, but the exported product data did not prove readback of the same local mem0 SDK corpus."
      fi
    elif [[ "${export_exit_code}" -eq 124 ]]; then
      status="blocked"
      reason_code="OPENMEMORY_EXPORT_TIMEOUT"
      reason="OpenMemory export did not complete within the bounded 30-second probe."
    elif grep -qi "docker.*command not found\|docker: not found\|docker not found" "${attempt_log}"; then
      status="blocked"
      reason_code="DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER"
      reason="The OpenMemory export helper requires Docker access, but Docker is not available inside the baseline-runner container."
    elif grep -qi "Container .*not found/running" "${attempt_log}"; then
      status="blocked"
      reason_code="OPENMEMORY_CONTAINER_NOT_RUNNING"
      reason="The OpenMemory export helper requires a running OpenMemory product container, but the baseline runner only starts the mem0 SDK path."
    else
      status="blocked"
      reason_code="OPENMEMORY_EXPORT_COMMAND_FAILED"
      reason="The OpenMemory export helper failed before export-helper readback could be validated."
    fi
  fi

  case "${status}" in
    pass)
      comparison_outcome="not_tested"
      ;;
    blocked)
      comparison_outcome="blocked"
      ;;
    unsupported)
      comparison_outcome="non_goal"
      ;;
    *)
      comparison_outcome="not_tested"
      ;;
  esac

  jq -nc \
    --arg schema "elf.live_baseline.openmemory_ui_export_probe/v1" \
    --arg run_id "${RUN_ID}" \
    --arg project "mem0/OpenMemory" \
    --arg scenario_id "openmemory_ui_export_readback" \
    --arg status "${status}" \
    --arg comparison_outcome "${comparison_outcome}" \
    --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg sdk_result_artifact "tmp/live-baseline/mem0-search.json" \
    --arg sdk_get_all_status "${sdk_get_all_status}" \
    --arg export_user_id "${export_user_id}" \
    --arg export_container "${export_container}" \
    --arg command "${command_display}" \
    --arg log_artifact "tmp/live-baseline/mem0-openmemory-export-attempt.log" \
    --arg output_excerpt "${output_excerpt}" \
    --arg reason_code "${reason_code}" \
    --arg reason "${reason}" \
    --arg next_action "${next_action}" \
    --argjson exit_code "${export_exit_code}" \
    --argjson openmemory_tree_present "${openmemory_tree_present}" \
    --argjson ui_package_present "${ui_package_present}" \
    --argjson compose_present "${compose_present}" \
    --argjson export_script_present "${export_script_present}" \
    --argjson sunsetting_notice_present "${sunsetting_notice_present}" \
    --argjson requires_api_key "${requires_api_key}" \
    --argjson requires_docker_compose "${requires_docker_compose}" \
    --argjson export_requires_running_container "${export_requires_running_container}" \
    --argjson validation "${validation_json}" \
    '{
      schema: $schema,
      run_id: $run_id,
      project: $project,
      scenario_id: $scenario_id,
      status: $status,
      comparison_outcome: $comparison_outcome,
      generated_at: $generated_at,
      same_corpus: {
        sdk_result_artifact: $sdk_result_artifact,
        sdk_get_all_check_status: $sdk_get_all_status,
        sdk_history_filters: {
          user_id: "elf-history-user",
          agent_id: "elf-history-agent",
          run_id: "elf-project"
        },
        sdk_get_all_is_ui_export_evidence: false
      },
      openmemory_surface: {
        tree_present: $openmemory_tree_present,
        ui_package_present: $ui_package_present,
        compose_file_present: $compose_present,
        export_script_present: $export_script_present,
        sunsetting_notice_present: $sunsetting_notice_present,
        requires_openai_api_key: $requires_api_key,
        requires_docker_compose: $requires_docker_compose,
        export_requires_running_container: $export_requires_running_container,
        default_export_container: $export_container
      },
      attempt: {
        command: $command,
        exit_code: $exit_code,
        log_artifact: $log_artifact,
        output_excerpt: $output_excerpt
      },
      export_validation: $validation,
      classification: {
        status: $status,
        reason_code: $reason_code,
        reason: $reason,
        next_action: $next_action
      },
      claim_boundary: {
        hosted_platform_claim: false,
        optional_graph_memory_enabled: false,
        sdk_get_all_is_ui_export_evidence: false
      }
    }' >"${out_path}"

  jq \
    --arg status "${status}" \
    --arg artifact "tmp/live-baseline/mem0-openmemory-ui-export.json" \
    '.behaviors.openmemory_ui_export.status = $status
      | .behaviors.openmemory_ui_export.surface =
        ("bounded OpenMemory export-helper setup probe recorded at " + $artifact + "; SDK get_all remains separate")' \
    "${REPORT_DIR}/mem0-adapter.json" >"${REPORT_DIR}/mem0-adapter.json.tmp"
  mv "${REPORT_DIR}/mem0-adapter.json.tmp" "${REPORT_DIR}/mem0-adapter.json"
  {
    echo "OpenMemory UI/export probe status: ${status}"
    echo "Reason code: ${reason_code}"
    echo "Next action: ${next_action}"
  } >>"${log_path}"
}
