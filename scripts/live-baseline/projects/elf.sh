project_elf() {
  local project="ELF"
  local repo="local:/workspace"
  local log_path="${REPORT_DIR}/${project}.log"
  local result_path="${REPORT_DIR}/${project}-result.json"
  local head
  cat >"${REPORT_DIR}/${project}-adapter.json" <<'JSON'
{
  "schema": "elf.live_baseline.adapter_metadata/v1",
  "project": "ELF",
  "storage": {
    "status": "real",
    "detail": "Docker-owned Postgres with pgvector is the source of truth and Qdrant is rebuilt from persisted chunk vectors."
  },
  "behaviors": {
    "same_corpus_retrieval": {
      "status": "real",
      "surface": "add_note, worker indexing, Qdrant rebuild, and search_raw over the configured service stores"
    },
    "update": {
      "status": "real",
      "surface": "service update plus worker reindex"
    },
    "delete_or_expire": {
      "status": "real",
      "surface": "service delete plus worker delete propagation"
    },
    "cold_start_reload": {
      "status": "real",
      "surface": "new ElfService over the same Postgres and Qdrant stores"
    },
    "concurrent_write_search": {
      "status": "real",
      "surface": "parallel add_note calls followed by worker indexing and search probes"
    },
    "scale_stress_profile": {
      "status": "real",
      "surface": "profile-selected generated or production corpus size plus soak and resource-envelope checks"
    },
    "soak_profile": {
      "status": "real",
      "surface": "profile-controlled repeated write/search stability window"
    },
    "resource_envelope": {
      "status": "real",
      "surface": "local elapsed-time and RSS envelope check"
    }
  }
}
JSON
  head="${ELF_BASELINE_ELF_HEAD:-}"
  if [[ -z "${head}" ]]; then
    head="$(git -C "${ROOT_DIR}" rev-parse HEAD 2>>"${log_path}" || echo "unknown")"
  fi

  if run_cmd "${project}: same-corpus retrieval" "$(elf_timeout_seconds)" "${log_path}" \
    "cd '${ROOT_DIR}' && cargo run -p elf-eval --bin live_baseline_elf -- --config config/local/elf.docker.toml --corpus '${CORPUS_DIR}' --queries '${REPORT_DIR}/queries.json' --out '${result_path}'"; then
    if [[ -s "${result_path}" ]] && jq -e '.checks and .check_summary' "${result_path}" >/dev/null 2>&1; then
      jq '{embedding, cost_proxy, query_summary: .summary, queries, backfill, ops_cases, check_summary, checks}' "${result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if [[ -s "${result_path}" ]] && jq -e --argjson document_count "${DOCUMENT_COUNT}" --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.elf_result/v1" and
      .status == "pass" and
      .summary.total == $query_count and
      .summary.fail == 0 and
      .check_summary.fail == 0 and
      .check_summary.incomplete == 0 and
      .backfill.source_count == $document_count and
      .backfill.completed_count == $document_count and
      (.backfill.duplicate_source_notes | length) == 0 and
      (
        .backfill.resume.enabled == false or
        (.backfill.resume.interrupted == true and .backfill.resume.resume_attempts >= 2)
      ) and
      (.check_summary.blocked // 0) == 0 and
      (.check_summary.not_encoded // 0) == 0 and
      .indexing.note_count == $document_count and
      .indexing.rebuild_rebuilt_count >= $document_count and
      .indexing.rebuild_error_count == 0
    ' "${result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "pass" "retrieval_pass" \
        "$(jq -r '.reason' "${result_path}")" \
        "${project}.log" "checkpointed add_note backfill; bounded worker outbox indexing; rebuild_qdrant; search_raw; concurrent writes; soak stability; latency/resource/cost proxies"
      return
    fi

    if [[ -s "${result_path}" ]] && jq -e '.schema == "elf.live_baseline.elf_result/v1"' "${result_path}" >/dev/null 2>&1; then
      json_record "${project}" "${repo}" "${head}" "$(jq -r '.status // "incomplete"' "${result_path}")" \
        "$(jq -r '.retrieval_status // "retrieval_failed"' "${result_path}")" \
        "$(jq -r '.reason // "ELF result did not satisfy live baseline pass criteria"' "${result_path}")" \
        "${project}.log" "checkpointed add_note backfill; bounded worker outbox indexing; rebuild_qdrant; search_raw; concurrent writes; soak stability; latency/resource/cost proxies"
      return
    fi

    json_record "${project}" "${repo}" "${head}" "incomplete" "runtime_failed" \
      "ELF command completed but did not write a valid live-baseline result; inspect ELF.log for the runtime error" \
      "${project}.log" "checkpointed add_note backfill; bounded worker outbox indexing; rebuild_qdrant; search_raw; concurrent writes; soak stability; latency/resource/cost proxies"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "runtime_failed" \
    "ELF same-corpus retrieval command failed in Docker" \
    "${project}.log" "checkpointed add_note backfill; bounded worker outbox indexing; rebuild_qdrant; search_raw; concurrent writes; soak stability; latency/resource/cost proxies"
}
