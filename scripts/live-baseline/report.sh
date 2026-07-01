json_record() {
  local project="$1"
  local repo="$2"
  local head="$3"
  local status="$4"
  local retrieval_status="$5"
  local reason="$6"
  local log_path="$7"
  local command_summary="$8"
  local finished_at
  local elapsed_seconds
  local checks_path
  local adapter_path
  finished_at="$(date +%s)"
  elapsed_seconds=0
  if [[ -n "${CURRENT_PROJECT_STARTED_AT}" ]]; then
    elapsed_seconds=$((finished_at - CURRENT_PROJECT_STARTED_AT))
  fi
  checks_path="${REPORT_DIR}/${project}-checks.json"
  adapter_path="${REPORT_DIR}/${project}-adapter.json"
  ensure_adapter_metadata "${project}"

  if [[ -s "${checks_path}" ]] && jq -e '.checks and .check_summary' "${checks_path}" >/dev/null 2>&1; then
    jq -nc \
      --arg project "${project}" \
      --arg repo "${repo}" \
      --arg head "${head}" \
      --arg status "${status}" \
      --arg retrieval_status "${retrieval_status}" \
      --arg reason "${reason}" \
      --arg log_path "${log_path}" \
      --arg command_summary "${command_summary}" \
      --argjson elapsed_seconds "${elapsed_seconds}" \
      --slurpfile adapter "${adapter_path}" \
      --slurpfile checks "${checks_path}" \
      '{
        project: $project,
        repo: $repo,
        head: $head,
        status: $status,
        retrieval_status: $retrieval_status,
        reason: $reason,
        log_path: $log_path,
        command_summary: $command_summary,
        elapsed_seconds: $elapsed_seconds,
        adapter: $adapter[0],
        embedding: ($checks[0].embedding // null),
        cost_proxy: ($checks[0].cost_proxy // null),
        query_summary: ($checks[0].query_summary // null),
        queries: ($checks[0].queries // null),
        backfill: ($checks[0].backfill // null),
        resource_envelope: ([$checks[0].checks[]? | select(.name == "resource_envelope") | .evidence][0] // null),
        ops_cases: ($checks[0].ops_cases // null),
        check_summary: $checks[0].check_summary,
        checks: $checks[0].checks
      }' >>"${RECORDS}"
  else
    jq -nc \
      --arg project "${project}" \
      --arg repo "${repo}" \
      --arg head "${head}" \
      --arg status "${status}" \
      --arg retrieval_status "${retrieval_status}" \
      --arg reason "${reason}" \
      --arg log_path "${log_path}" \
      --arg command_summary "${command_summary}" \
      --argjson elapsed_seconds "${elapsed_seconds}" \
      --slurpfile adapter "${adapter_path}" \
      '
        def check_status:
          if $status == "pass" and $retrieval_status == "retrieval_pass" then "pass"
          elif $status == "wrong_result" then "wrong_result"
          elif $status == "lifecycle_fail" then "lifecycle_fail"
          elif $status == "blocked" then "blocked"
          elif $status == "not_encoded" then "not_encoded"
          elif $status == "incomplete" then "incomplete"
          elif $retrieval_status == "retrieval_pass" then "pass"
          else "incomplete"
          end;
        def is_fail:
          check_status == "wrong_result" or check_status == "lifecycle_fail";
      {
        project: $project,
        repo: $repo,
        head: $head,
        status: $status,
        retrieval_status: $retrieval_status,
        reason: $reason,
        log_path: $log_path,
        command_summary: $command_summary,
        elapsed_seconds: $elapsed_seconds,
        query_summary: null,
        queries: null,
        backfill: null,
        cost_proxy: null,
        resource_envelope: null,
        ops_cases: null,
        adapter: $adapter[0],
        check_summary: {
          total: 1,
          pass: (if check_status == "pass" then 1 else 0 end),
          fail: (if is_fail then 1 else 0 end),
          wrong_result: (if check_status == "wrong_result" then 1 else 0 end),
          lifecycle_fail: (if check_status == "lifecycle_fail" then 1 else 0 end),
          incomplete: (if check_status == "incomplete" then 1 else 0 end),
          blocked: (if check_status == "blocked" then 1 else 0 end),
          not_encoded: (if check_status == "not_encoded" then 1 else 0 end)
        },
        checks: [
          {
            name: "same_corpus_retrieval",
            status: check_status,
            reason: $reason,
            evidence: {
              retrieval_status: $retrieval_status,
              log_path: $log_path,
              command_summary: $command_summary
            }
          }
        ]
      }' >>"${RECORDS}"
  fi
}

run_cmd() {
  local label="$1"
  local timeout_seconds="$2"
  local log_path="$3"
  shift 3

  {
    echo "## ${label}"
    echo "## started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "## command=$*"
  } >>"${log_path}"

  if timeout "${timeout_seconds}" bash -lc "$*" >>"${log_path}" 2>&1; then
    echo "## exit=0" >>"${log_path}"
    return 0
  fi

  local code
  code=$?
  echo "## exit=${code}" >>"${log_path}"
  return "${code}"
}

clone_project() {
  local project="$1"
  local repo="$2"
  local log_path="$3"
  local target="${REPOS_DIR}/${project}"

  if run_cmd "${project}: clone" 180 "${log_path}" "git clone --depth 1 '${repo}' '${target}'"; then
    git -C "${target}" rev-parse HEAD
    return 0
  fi

  echo "clone_failed"
  return 1
}

prepare_project_corpus() {
  local project="$1"
  local target="${WORK_DIR}/corpus-${project}"

  rm -rf "${target}"
  mkdir -p "${target}"
  cp -R "${CORPUS_DIR}/." "${target}/"
  echo "${target}"
}

finish_report() {
  jq -s \
    --arg schema "elf.live_baseline.report/v1" \
    --arg run_id "${RUN_ID}" \
    --arg project_filter "${PROJECT_FILTER}" \
    --arg corpus_profile "${CORPUS_PROFILE}" \
    --arg corpus_track "${CORPUS_TRACK}" \
    --arg corpus_path "${CORPUS_PATH_DESCRIPTION}" \
    --arg corpus_manifest_id "${CORPUS_MANIFEST_ID}" \
    --argjson document_count "${DOCUMENT_COUNT}" \
    --argjson query_count "${QUERY_COUNT}" \
    --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    '
      def failure_status:
        . == "wrong_result" or . == "lifecycle_fail";
    {
      schema: $schema,
      run_id: $run_id,
      generated_at: $generated_at,
      docker_only: true,
      project_filter: $project_filter,
      corpus: {
        profile: $corpus_profile,
        track: $corpus_track,
        manifest_id: (if $corpus_manifest_id == "" then null else $corpus_manifest_id end),
        document_count: $document_count,
        query_count: $query_count,
        path: $corpus_path,
        query_file: "tmp/live-baseline/queries.json"
      },
      verdict: (
        if length == 0 then "incomplete"
        elif any(.[]; .status | failure_status) then "fail"
        elif any(.[]; .status == "blocked") then "blocked"
        elif any(.[]; .status == "incomplete") then "incomplete"
        elif any(.[]; .status == "not_encoded") then "incomplete"
        elif all(.[]; .status == "pass" and .retrieval_status == "retrieval_pass") then "pass"
        else "incomplete"
        end
      ),
      summary: {
        total: length,
        pass: ([.[] | select(.status == "pass")] | length),
        fail: ([.[] | select(.status | failure_status)] | length),
        wrong_result: ([.[] | select(.status == "wrong_result")] | length),
        lifecycle_fail: ([.[] | select(.status == "lifecycle_fail")] | length),
        incomplete: ([.[] | select(.status == "incomplete")] | length),
        blocked: ([.[] | select(.status == "blocked")] | length),
        not_encoded: ([.[] | select(.status == "not_encoded")] | length)
      },
      same_corpus_summary: {
        total: length,
        pass: ([.[] | select(.retrieval_status == "retrieval_pass")] | length),
        fail: ([.[] | select(.retrieval_status == "retrieval_wrong_result")] | length),
        wrong_result: ([.[] | select(.retrieval_status == "retrieval_wrong_result")] | length),
        lifecycle_fail: 0,
        incomplete: ([.[] | select(.retrieval_status != "retrieval_pass" and .status == "incomplete")] | length),
        blocked: ([.[] | select(.retrieval_status != "retrieval_pass" and .status == "blocked")] | length),
        not_encoded: ([.[] | select(.retrieval_status != "retrieval_pass" and .status == "not_encoded")] | length)
      },
      full_check_summary: {
        total: ([.[] | .check_summary.total // 0] | add // 0),
        pass: ([.[] | .check_summary.pass // 0] | add // 0),
        fail: ([.[] | .check_summary.fail // 0] | add // 0),
        wrong_result: ([.[] | .check_summary.wrong_result // 0] | add // 0),
        lifecycle_fail: ([.[] | .check_summary.lifecycle_fail // 0] | add // 0),
        incomplete: ([.[] | .check_summary.incomplete // 0] | add // 0),
        blocked: ([.[] | .check_summary.blocked // 0] | add // 0),
        not_encoded: ([.[] | .check_summary.not_encoded // 0] | add // 0)
      },
      wrong_result_count: ([.[] | .query_summary.wrong_result_count // .query_summary.fail // 0] | add // 0),
      latency_ms: {
        total: ([.[] | .query_summary.latency_ms_total // 0] | add // 0),
        mean: (
          [.[] | select(.query_summary != null) | .query_summary.latency_ms_mean // 0] as $means
          | if ($means | length) == 0 then 0 else (($means | add) / ($means | length)) end
        ),
        p50: (
          [.[] | select(.query_summary != null) | .query_summary.latency_ms_p50 // 0] as $values
          | if ($values | length) == 0 then 0 else (($values | add) / ($values | length)) end
        ),
        p95: (
          [.[] | select(.query_summary != null) | .query_summary.latency_ms_p95 // 0] as $values
          | if ($values | length) == 0 then 0 else (($values | add) / ($values | length)) end
        ),
        p99: (
          [.[] | select(.query_summary != null) | .query_summary.latency_ms_p99 // 0] as $values
          | if ($values | length) == 0 then 0 else (($values | add) / ($values | length)) end
        ),
        max: ([.[] | .query_summary.latency_ms_max // 0] | max // 0)
      },
      cost_proxy: {
        projects: [.[] | select(.cost_proxy != null) | {project, cost_proxy}],
        estimated_usd: ([.[] | .cost_proxy.estimated_usd? // empty] | add // null),
        estimated_input_tokens: ([.[] | .cost_proxy.estimated_input_tokens // 0] | add // 0)
      },
      resource_usage: {
        projects: [.[] | select(.resource_envelope != null) | {project, resource_envelope}]
      },
      ops_cases: [.[] | select(.ops_cases != null) | {project, cases: .ops_cases}],
      projects: .
    }' "${RECORDS}" >"${REPORT}"
}
