project_qmd() {
  local project="qmd"
  local repo="https://github.com/tobi/qmd.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local query_result_path="${REPORT_DIR}/${project}-query.json"
  local status_path="${REPORT_DIR}/${project}-status.txt"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-qmd.mjs"
  local home="${HOME_DIR}/${project}"
  local corpus_path
  local head
  mkdir -p "${home}"
  cat >"${REPORT_DIR}/${project}-adapter.json" <<'JSON'
{
  "schema": "elf.live_baseline.adapter_metadata/v1",
  "project": "qmd",
  "storage": {
    "status": "real",
    "detail": "The adapter uses qmd's local collection, persisted project files, and fresh CLI query processes inside Docker."
  },
  "behaviors": {
    "same_corpus_retrieval": {
      "status": "real",
      "surface": "collection add, update, embed -f, and query --json"
    },
    "update": {
      "status": "real",
      "surface": "rewrite corpus file, rerun qmd update/embed, and query for the replacement marker"
    },
    "delete_or_expire": {
      "status": "real",
      "surface": "delete corpus file, rerun qmd update, and verify deleted evidence is not returned"
    },
    "expire": {
      "status": "unsupported",
      "surface": "qmd file collections support deletion but no TTL/expiry behavior is encoded"
    },
    "cold_start_reload": {
      "status": "real",
      "surface": "fresh qmd query process over the persisted local collection"
    },
    "scale_stress_profile": {
      "status": "real",
      "surface": "Run ELF_BASELINE_PROJECTS=qmd with ELF_BASELINE_PROFILE=scale or stress through cargo make baseline-live-docker."
    }
  }
}
JSON
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install/build" 300 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && (npm ci || npm install --no-audit --no-fund) && npm run build --if-present"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "install/build failed" "${project}.log" "npm install/build"
    return
  fi
  corpus_path="$(prepare_project_corpus "${project}")"

  cat >"${driver_path}" <<'JS'
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const outPath = process.argv[2];
const queriesPath = process.argv[3];
const corpusPath = process.argv[4];
if (!outPath || !queriesPath || !corpusPath) {
  throw new Error("output path, query path, and corpus path are required");
}

const queries = JSON.parse(readFileSync(queriesPath, "utf8")).queries;
const topK = process.env.ELF_BASELINE_TOP_K ?? "10";

function resultMatches(results, query) {
  if (!Array.isArray(results)) return false;
  return results.some((entry) => {
    const entryText = JSON.stringify(entry).toLowerCase();
    const file = String(entry.file ?? "");
    return (
      file.includes(query.expected_doc) &&
      query.expected_terms.every((term) =>
        entryText.includes(String(term).toLowerCase()),
      )
    );
  });
}

function qmdQuery(queryText) {
  const structuredQuery = `lex: ${queryText}\nvec: ${queryText}`;
  const stdout = execFileSync(
    "npx",
    [
      "tsx",
      "src/cli/qmd.ts",
      "query",
      structuredQuery,
      "-c",
      "elfbench",
      "--json",
      "--no-rerank",
      "--min-score",
      "0",
      "-n",
      topK,
    ],
    { encoding: "utf8", env: process.env },
  );
  return JSON.parse(stdout);
}

function runQueryCase(query) {
  const results = qmdQuery(query.query);
  return {
    id: query.id,
    query: query.query,
    expected_doc: query.expected_doc,
    expected_terms: query.expected_terms,
    matched: resultMatches(results, query),
    results,
  };
}

function makeCheck(name, status, reason, evidence) {
  return { name, status, reason, evidence };
}

function summarizeChecks(checks) {
  return {
    total: checks.length,
    pass: checks.filter((check) => check.status === "pass").length,
    fail: checks.filter(
      (check) =>
        check.status === "wrong_result" ||
        check.status === "lifecycle_fail",
    ).length,
    wrong_result: checks.filter((check) => check.status === "wrong_result")
      .length,
    lifecycle_fail: checks.filter((check) => check.status === "lifecycle_fail")
      .length,
    incomplete: checks.filter((check) => check.status === "incomplete").length,
    blocked: checks.filter((check) => check.status === "blocked").length,
    not_encoded: checks.filter((check) => check.status === "not_encoded")
      .length,
  };
}

function runQmd(args) {
  return execFileSync("npx", ["tsx", "src/cli/qmd.ts", ...args], {
    encoding: "utf8",
    env: process.env,
  });
}

function syncCollection({ embed = false } = {}) {
  runQmd(["update"]);
  if (embed) {
    runQmd(["embed", "-f", "-c", "elfbench"]);
  }
}

const queryResults = queries.map((query) => runQueryCase(query));
const pass = queryResults.filter((result) => result.matched).length;
const checks = [
  makeCheck(
    "same_corpus_retrieval",
    pass === queryResults.length ? "pass" : "wrong_result",
    pass === queryResults.length
      ? "qmd structured hybrid query returned expected evidence for every query."
      : "qmd structured hybrid query missed one or more expected results.",
    {
      total: queryResults.length,
      pass,
      fail: queryResults.length - pass,
    },
  ),
];

const authPath = join(corpusPath, "auth-memory.md");
if (!existsSync(authPath)) {
  checks.push(
    makeCheck(
      "update_replaces_note_text",
      "not_encoded",
      "The auth corpus file was missing, so qmd update could not be exercised.",
      { source: "auth-memory.md" },
    ),
  );
} else {
  writeFileSync(
    authPath,
    "# Auth Memory\n\nRotated auth middleware validates JWT tokens with key id `kid-v4` under `RotatedJwtKeyPlan`. It still requires tenant scope `project_shared` for deployment operations after the emergency key rotation.\n",
  );
  syncCollection({ embed: true });
  const updateQuery = {
    id: "lifecycle-update-new-marker",
    query: "Which rotated JWT key id does the auth middleware require?",
    expected_doc: "auth-memory.md",
    expected_terms: ["kid-v4", "RotatedJwtKeyPlan"],
  };
  const updateResults = qmdQuery(updateQuery.query);
  const updateMatched = resultMatches(updateResults, updateQuery);
  const oldMarkerAbsent = updateResults
    .filter((entry) => String(entry.file ?? "").includes("auth-memory.md"))
    .every((entry) => !JSON.stringify(entry).toLowerCase().includes("kid-v3"));
  checks.push(
    makeCheck(
      "update_replaces_note_text",
      updateMatched && oldMarkerAbsent ? "pass" : "lifecycle_fail",
      updateMatched && oldMarkerAbsent
        ? "qmd update/embed returned the new marker and did not return the old marker for the updated file."
        : "qmd update/embed did not cleanly replace the searchable auth file text.",
      {
        source: "auth-memory.md",
        matched_new_marker: updateMatched,
        old_marker_absent: oldMarkerAbsent,
        results: updateResults,
      },
    ),
  );
}

const deleteQuery = queries.find(
  (query) =>
    query.expected_doc !== "auth-memory.md" &&
    query.expected_doc !== "database-memory.md" &&
    existsSync(join(corpusPath, query.expected_doc)),
);
if (!deleteQuery) {
  checks.push(
    makeCheck(
      "delete_suppresses_retrieval",
      "not_encoded",
      "No non-update, non-recovery corpus file was available, so qmd delete could not be exercised.",
      { available_docs: queries.map((query) => query.expected_doc) },
    ),
  );
} else {
  unlinkSync(join(corpusPath, deleteQuery.expected_doc));
  syncCollection();
  const deleteResults = qmdQuery(deleteQuery.query);
  const deletedStillMatched = resultMatches(deleteResults, deleteQuery);
  checks.push(
    makeCheck(
      "delete_suppresses_retrieval",
      deletedStillMatched ? "lifecycle_fail" : "pass",
      deletedStillMatched
        ? "qmd update marked the deleted file removed, but it was still searchable."
        : "qmd update suppressed the deleted file from subsequent search.",
      {
        source: deleteQuery.expected_doc,
        query: deleteQuery,
        deleted_still_matched: deletedStillMatched,
        results: deleteResults,
      },
    ),
  );
}

const recoveryQuery = {
  id: "lifecycle-cold-start-recovery",
  query:
    "The invoice list N+1 query was fixed by eager loading invoice lines through `InvoiceLineBatcher`. Do not reintroduce per-row SQL calls in invoice rendering.",
  expected_doc: "database-memory.md",
  expected_terms: ["InvoiceLineBatcher", "N+1"],
};
const recoveryResults = qmdQuery(recoveryQuery.query);
const recoveryMatched = resultMatches(recoveryResults, recoveryQuery);
checks.push(
  makeCheck(
    "cold_start_recovery_search",
    recoveryMatched ? "pass" : "lifecycle_fail",
    recoveryMatched
      ? "A fresh qmd query process reopened the persisted index and retrieved expected evidence."
      : "A fresh qmd query process did not retrieve expected persisted evidence.",
    {
      expected_doc: recoveryQuery.expected_doc,
      matched: recoveryMatched,
      results: recoveryResults,
    },
  ),
);

const checkSummary = summarizeChecks(checks);
writeFileSync(
  outPath,
  JSON.stringify(
    {
      schema: "elf.live_baseline.qmd_result/v1",
      summary: {
        total: queryResults.length,
        pass,
        fail: queryResults.length - pass,
      },
      check_summary: checkSummary,
      checks,
      queries: queryResults,
    },
    null,
    2,
  ),
);
JS

  if run_cmd "${project}: embedded retrieval" 900 "${log_path}" \
    "export HOME='${home}'; export XDG_CACHE_HOME='/root/.cache'; export QMD_FORCE_CPU=1; cd '${REPOS_DIR}/${project}' && npx tsx src/cli/qmd.ts collection add '${corpus_path}' --name elfbench && npx tsx src/cli/qmd.ts update && npx tsx src/cli/qmd.ts embed -f -c elfbench && npx tsx src/cli/qmd.ts status > '${status_path}' && node '${driver_path}' '${query_result_path}' '${REPORT_DIR}/queries.json' '${corpus_path}'"; then
    if jq -e '.checks and .check_summary' "${query_result_path}" >/dev/null 2>&1; then
      jq '{check_summary, checks}' "${query_result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.qmd_result/v1" and
      .summary.total == $query_count
    ' "${query_result_path}" >/dev/null; then
      local typed_status
      local retrieval_status
      typed_status="$(typed_status_from_result "${query_result_path}")"
      if jq -e '.summary.fail == 0' "${query_result_path}" >/dev/null; then
        retrieval_status="retrieval_pass"
      else
        retrieval_status="retrieval_wrong_result"
      fi
      json_record "${project}" "${repo}" "${head}" "${typed_status}" "${retrieval_status}" "$(typed_status_reason "${project}" "${typed_status}")" "${project}.log" "collection add; update; embed -f; query --json"
    elif ! rg -q "Embedded [1-9][0-9]* chunks" "${log_path}"; then
      json_record "${project}" "${repo}" "${head}" "incomplete" "embedding_required" "qmd indexed the corpus, but no successful embedding completion was observed" "${project}.log" "collection add; update; embed -f; query --json"
    elif ! jq -e '.schema == "elf.live_baseline.qmd_result/v1"' "${query_result_path}" >/dev/null 2>&1; then
      json_record "${project}" "${repo}" "${head}" "incomplete" "invalid_json_result" "qmd query command completed, but did not produce parseable JSON results" "${project}.log" "collection add; update; embed -f; search/query --json"
    else
      json_record "${project}" "${repo}" "${head}" "wrong_result" "retrieval_wrong_result" "qmd embedded retrieval ran but did not return expected evidence" "${project}.log" "collection add; update; embed -f; search/query --json"
    fi
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "qmd install passed but embedded retrieval command failed" "${project}.log" "collection add; update; embed -f; search/query --json"
}
