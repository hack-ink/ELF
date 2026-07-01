elf_timeout_seconds() {
  if [[ -n "${ELF_BASELINE_ELF_TIMEOUT_SECONDS:-}" ]]; then
    echo "${ELF_BASELINE_ELF_TIMEOUT_SECONDS}"
    return
  fi

  case "${CORPUS_PROFILE}" in
    backfill | large)
      echo 3600
      ;;
    stress)
      echo 1800
      ;;
    *)
      echo 1200
      ;;
  esac
}

ensure_adapter_metadata() {
  local project="$1"
  local adapter_path="${REPORT_DIR}/${project}-adapter.json"

  if [[ -s "${adapter_path}" ]] && jq -e . "${adapter_path}" >/dev/null 2>&1; then
    return
  fi

  jq -nc \
    --arg project "${project}" \
    '{
      schema: "elf.live_baseline.adapter_metadata/v1",
      project: $project,
      storage: {
        status: "incomplete",
        detail: "Adapter metadata was not declared by the project runner."
      },
      behaviors: {}
    }' >"${adapter_path}"
}

typed_status_from_result() {
  local result_path="$1"

  jq -r '
    .check_summary as $summary
    | if ($summary.wrong_result // 0) > 0 then "wrong_result"
      elif ($summary.lifecycle_fail // 0) > 0 then "lifecycle_fail"
      elif ($summary.blocked // 0) > 0 then "blocked"
      elif ($summary.incomplete // 0) > 0 then "incomplete"
      elif ($summary.not_encoded // 0) > 0 then "not_encoded"
      else "pass"
      end
  ' "${result_path}"
}

typed_status_reason() {
  local project="$1"
  local status="$2"

  case "${status}" in
    pass)
      if [[ "${project}" == "mem0" ]]; then
        echo "mem0 SDK same-corpus retrieval and every encoded SDK behavior check passed; OpenMemory export-helper setup probe is reported separately in adapter.behaviors.openmemory_ui_export and tmp/live-baseline/mem0-openmemory-ui-export.json"
      else
        echo "${project} same-corpus retrieval and every encoded behavior check passed"
      fi
      ;;
    wrong_result)
      echo "${project} ran but returned the wrong same-corpus result or missed expected evidence"
      ;;
    lifecycle_fail)
      echo "${project} same-corpus retrieval passed, but one or more lifecycle checks failed"
      ;;
    blocked)
      echo "${project} same-corpus retrieval passed, but one or more lifecycle checks are blocked by missing durable runtime, credentials, or host integration"
      ;;
    incomplete)
      echo "${project} setup or a declared behavior check could not complete in the Docker runner"
      ;;
    not_encoded)
      echo "${project} same-corpus retrieval passed, but one or more capability checks are not encoded"
      ;;
    *)
      echo "${project} produced unrecognized benchmark status ${status}"
      ;;
  esac
}
