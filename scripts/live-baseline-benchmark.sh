#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_BASELINE_REPORT_DIR:-${ROOT_DIR}/tmp/live-baseline}"
WORK_DIR="${ELF_BASELINE_WORK_DIR:-/bench}"
REPOS_DIR="${WORK_DIR}/repos"
CORPUS_DIR="${WORK_DIR}/corpus"
HOME_DIR="${WORK_DIR}/home"
RECORDS="${REPORT_DIR}/project-records.jsonl"
REPORT="${REPORT_DIR}/live-baseline-report.json"
RUN_ID="${ELF_BASELINE_RUN_ID:-live-baseline-$(date +%Y%m%d%H%M%S)}"
PROJECT_FILTER="${ELF_BASELINE_PROJECTS:-all}"
CORPUS_PROFILE="${ELF_BASELINE_PROFILE:-smoke}"
SCALE_DOC_COUNT="${ELF_BASELINE_SCALE_DOCS:-120}"
STRESS_DOC_COUNT="${ELF_BASELINE_STRESS_DOCS:-480}"
QUERY_TOP_K="${ELF_BASELINE_TOP_K:-10}"
CURRENT_PROJECT_STARTED_AT=""

if [[ ! -f "/.dockerenv" && "${ELF_BASELINE_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run live baseline benchmark outside Docker. Use cargo make baseline-live-docker." >&2
  exit 1
fi

for cmd in bash cargo git jq node npm python3 rg timeout; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in baseline runner." >&2
    exit 1
  fi
done

generate_corpus() {
  python3 - "${CORPUS_PROFILE}" "${SCALE_DOC_COUNT}" "${STRESS_DOC_COUNT}" "${CORPUS_DIR}" "${REPORT_DIR}/queries.json" <<'PY'
import json
import sys
from pathlib import Path

profile, scale_doc_count_raw, stress_doc_count_raw, corpus_dir_raw, queries_path_raw = sys.argv[1:]
corpus_dir = Path(corpus_dir_raw)
queries_path = Path(queries_path_raw)
scale_doc_count = int(scale_doc_count_raw)
stress_doc_count = int(stress_doc_count_raw)

anchors = [
    {
        "name": "auth-memory.md",
        "title": "Auth Memory",
        "body": "The API auth middleware validates JWT tokens with key id `kid-v3`. The middleware rejects tokens older than 15 minutes and requires tenant scope `project_shared` for deployment operations.",
        "query": "Which JWT key id does the auth middleware require?",
        "alternate_query": "Find the auth note that mentions key id kid-v3 and tenant scope.",
        "terms": ["kid-v3", "auth middleware"],
    },
    {
        "name": "database-memory.md",
        "title": "Database Memory",
        "body": "The invoice list N+1 query was fixed by eager loading invoice lines through `InvoiceLineBatcher`. Do not reintroduce per-row SQL calls in invoice rendering.",
        "query": "How was the invoice list N+1 query fixed?",
        "alternate_query": "Find the invoice rendering memory about InvoiceLineBatcher and N+1 prevention.",
        "terms": ["InvoiceLineBatcher", "N+1"],
    },
    {
        "name": "deploy-memory.md",
        "title": "Deploy Memory",
        "body": "Production deploys must run Docker-isolated parity checks first. The cleanup command must remove Postgres, Qdrant, npm, pip, cargo, and target volumes before adoption.",
        "query": "What must be cleaned up after Docker parity checks?",
        "alternate_query": "Find the deploy checklist that mentions Postgres, Qdrant, and cleanup volumes.",
        "terms": ["Postgres", "Qdrant", "volumes"],
    },
    {
        "name": "retention-memory.md",
        "title": "Retention Memory",
        "body": "The retention worker uses `RetentionSweepPlan` before deletion and writes a tombstone ledger entry named `ledger-retain-77` for every expired note.",
        "query": "Which plan does the retention worker use before deletion?",
        "alternate_query": "Find the retention note with ledger-retain-77 tombstone handling.",
        "terms": ["RetentionSweepPlan", "ledger-retain-77"],
    },
    {
        "name": "incident-memory.md",
        "title": "Incident Memory",
        "body": "During canary incidents, `CanaryTraceGate` must stay enabled until the rollback window closes and the release captain records marker `incident-green-42`.",
        "query": "Which gate stays enabled during canary incidents?",
        "alternate_query": "Find the canary incident memory with incident-green-42.",
        "terms": ["CanaryTraceGate", "incident-green-42"],
    },
    {
        "name": "billing-memory.md",
        "title": "Billing Memory",
        "body": "Billing replay uses `UsageAccumulator` with idempotency key `bill-run-42` so duplicate metering events do not create extra invoices.",
        "query": "Which accumulator and idempotency key protect billing replay?",
        "alternate_query": "Find the billing replay note with bill-run-42.",
        "terms": ["UsageAccumulator", "bill-run-42"],
    },
    {
        "name": "search-memory.md",
        "title": "Search Memory",
        "body": "Search fanout routes tenant scoped reads through `SemanticShardRouter`; every shard label must include the prefix `tenant_scope` before merge ranking.",
        "query": "Which router handles tenant scoped search fanout?",
        "alternate_query": "Find the tenant_scope shard routing memory.",
        "terms": ["SemanticShardRouter", "tenant_scope"],
    },
    {
        "name": "recovery-memory.md",
        "title": "Recovery Memory",
        "body": "Disaster recovery requires `SnapshotRestoreFence` and a WAL checkpoint named `wal-green-17` before accepting new writes after restore.",
        "query": "Which fence is required before accepting writes after restore?",
        "alternate_query": "Find the disaster recovery note with wal-green-17.",
        "terms": ["SnapshotRestoreFence", "wal-green-17"],
    },
]

if profile == "smoke":
    docs = anchors[:3]
elif profile in {"scale", "full"}:
    docs = list(anchors)
    target_count = max(scale_doc_count, len(anchors))
elif profile == "stress":
    docs = list(anchors)
    target_count = max(stress_doc_count, len(anchors))
else:
    raise SystemExit(f"unsupported ELF_BASELINE_PROFILE={profile!r}")

if profile in {"scale", "full", "stress"}:
    topics = [
        "scheduler dry run budget window",
        "operator dashboard cache refresh",
        "import packet normalization lane",
        "workspace role synchronization",
        "trace export sampling policy",
        "background compaction checkpoint",
        "local fixture replay validation",
        "notification queue dampening",
    ]
    for idx in range(1, target_count - len(anchors) + 1):
        topic = topics[idx % len(topics)]
        docs.append(
            {
                "name": f"distractor-{idx:03d}.md",
                "title": f"Distractor Memory {idx:03d}",
                "body": (
                    f"This operational note covers {topic}. "
                    f"It intentionally uses ordinary maintenance vocabulary for lane {idx:03d}, "
                    f"checkpoint batch {1000 + idx}, and reviewer group {idx % 9}. "
                    "It should not answer the benchmark needle queries."
                ),
            }
        )

for existing in corpus_dir.glob("*.md"):
    existing.unlink()

for doc in docs:
    (corpus_dir / doc["name"]).write_text(
        f"# {doc['title']}\n\n{doc['body']}\n", encoding="utf-8"
    )

query_docs = anchors[: (3 if profile == "smoke" else len(anchors))]
queries = []
for doc in query_docs:
    base_id = doc["name"].replace("-memory.md", "").replace(".md", "")
    queries.append(
        {
            "id": f"q-{base_id}",
            "query": doc["query"],
            "expected_doc": doc["name"],
            "expected_terms": doc["terms"],
        }
    )
    if profile == "stress":
        queries.append(
            {
                "id": f"q-{base_id}-alt",
                "query": doc["alternate_query"],
                "expected_doc": doc["name"],
                "expected_terms": doc["terms"],
            }
        )

queries_path.write_text(
    json.dumps(
        {
            "schema": "elf.live_baseline.queries/v1",
            "profile": profile,
            "document_count": len(docs),
            "queries": queries,
        },
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY
}

rm -rf "${WORK_DIR}"
mkdir -p "${REPORT_DIR}"
find "${REPORT_DIR}" -maxdepth 1 -type f -delete
mkdir -p "${REPOS_DIR}" "${CORPUS_DIR}" "${HOME_DIR}"
: >"${RECORDS}"

generate_corpus
DOCUMENT_COUNT="$(find "${CORPUS_DIR}" -maxdepth 1 -type f -name '*.md' | wc -l | tr -d ' ')"
QUERY_COUNT="$(jq '.queries | length' "${REPORT_DIR}/queries.json")"

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
  finished_at="$(date +%s)"
  elapsed_seconds=0
  if [[ -n "${CURRENT_PROJECT_STARTED_AT}" ]]; then
    elapsed_seconds=$((finished_at - CURRENT_PROJECT_STARTED_AT))
  fi
  checks_path="${REPORT_DIR}/${project}-checks.json"

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
        embedding: ($checks[0].embedding // null),
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
        check_summary: {
          total: 1,
          pass: (if $retrieval_status == "retrieval_pass" then 1 else 0 end),
          fail: (if $status == "fail" then 1 else 0 end),
          incomplete: (if $retrieval_status != "retrieval_pass" and $status != "fail" then 1 else 0 end)
        },
        checks: [
          {
            name: "same_corpus_retrieval",
            status: (if $retrieval_status == "retrieval_pass" then "pass" elif $status == "fail" then "fail" else "incomplete" end),
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

finish_report() {
  jq -s \
    --arg schema "elf.live_baseline.report/v1" \
    --arg run_id "${RUN_ID}" \
    --arg project_filter "${PROJECT_FILTER}" \
    --arg corpus_profile "${CORPUS_PROFILE}" \
    --argjson document_count "${DOCUMENT_COUNT}" \
    --argjson query_count "${QUERY_COUNT}" \
    --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    '{
      schema: $schema,
      run_id: $run_id,
      generated_at: $generated_at,
      docker_only: true,
      project_filter: $project_filter,
      corpus: {
        profile: $corpus_profile,
        document_count: $document_count,
        query_count: $query_count,
        path: "generated in Docker under /bench/corpus",
        query_file: "tmp/live-baseline/queries.json"
      },
      verdict: (
        if length == 0 then "incomplete"
        elif any(.[]; .status == "fail") then "fail"
        elif all(.[]; .status == "pass" and .retrieval_status == "retrieval_pass") then "pass"
        else "incomplete"
        end
      ),
      summary: {
        total: length,
        pass: ([.[] | select(.status == "pass")] | length),
        fail: ([.[] | select(.status == "fail")] | length),
        incomplete: ([.[] | select(.status == "incomplete")] | length)
      },
      same_corpus_summary: {
        total: length,
        pass: ([.[] | select(.retrieval_status == "retrieval_pass")] | length),
        fail: ([.[] | select(.retrieval_status != "retrieval_pass" and .status == "fail")] | length),
        incomplete: ([.[] | select(.retrieval_status != "retrieval_pass" and .status != "fail")] | length)
      },
      full_check_summary: {
        total: ([.[] | .check_summary.total // 0] | add // 0),
        pass: ([.[] | .check_summary.pass // 0] | add // 0),
        fail: ([.[] | .check_summary.fail // 0] | add // 0),
        incomplete: ([.[] | .check_summary.incomplete // 0] | add // 0)
      },
      projects: .
    }' "${RECORDS}" >"${REPORT}"
}

project_enabled() {
  local project="$1"

  if [[ -z "${PROJECT_FILTER}" || "${PROJECT_FILTER}" == "all" ]]; then
    return 0
  fi

  for selected in ${PROJECT_FILTER//,/ }; do
    if [[ "${selected}" == "${project}" ]]; then
      return 0
    fi
  done

  return 1
}

run_project() {
  local project="$1"
  local fn="$2"

  if project_enabled "${project}"; then
    CURRENT_PROJECT_STARTED_AT="$(date +%s)"
    "${fn}"
    CURRENT_PROJECT_STARTED_AT=""
  fi
}

project_elf() {
  local project="ELF"
  local repo="local:/workspace"
  local log_path="${REPORT_DIR}/${project}.log"
  local result_path="${REPORT_DIR}/${project}-result.json"
  local head
  head="${ELF_BASELINE_ELF_HEAD:-}"
  if [[ -z "${head}" ]]; then
    head="$(git -C "${ROOT_DIR}" rev-parse HEAD 2>>"${log_path}" || echo "unknown")"
  fi

  if run_cmd "${project}: same-corpus retrieval" 1200 "${log_path}" \
    "cd '${ROOT_DIR}' && cargo run -p elf-eval --bin live_baseline_elf -- --config config/local/elf.docker.toml --corpus '${CORPUS_DIR}' --queries '${REPORT_DIR}/queries.json' --out '${result_path}'"; then
    if [[ -s "${result_path}" ]] && jq -e '.checks and .check_summary' "${result_path}" >/dev/null 2>&1; then
      jq '{embedding, check_summary, checks}' "${result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if [[ -s "${result_path}" ]] && jq -e --argjson document_count "${DOCUMENT_COUNT}" --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.elf_result/v1" and
      .status == "pass" and
      .summary.total == $query_count and
      .summary.fail == 0 and
      .check_summary.fail == 0 and
      .check_summary.incomplete == 0 and
      .indexing.note_count == $document_count and
      .indexing.rebuild_rebuilt_count >= $document_count and
      .indexing.rebuild_error_count == 0
    ' "${result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "pass" "retrieval_pass" \
        "$(jq -r '.reason' "${result_path}")" \
        "${project}.log" "add_note; worker outbox indexing; rebuild_qdrant; search_raw; concurrent writes; soak stability"
      return
    fi

    if [[ -s "${result_path}" ]] && jq -e '.schema == "elf.live_baseline.elf_result/v1"' "${result_path}" >/dev/null 2>&1; then
      json_record "${project}" "${repo}" "${head}" "$(jq -r '.status // "fail"' "${result_path}")" \
        "$(jq -r '.retrieval_status // "retrieval_failed"' "${result_path}")" \
        "$(jq -r '.reason // "ELF result did not satisfy live baseline pass criteria"' "${result_path}")" \
        "${project}.log" "add_note; worker outbox indexing; rebuild_qdrant; search_raw; concurrent writes; soak stability"
      return
    fi

    json_record "${project}" "${repo}" "${head}" "fail" "runtime_failed" \
      "ELF command completed but did not write a valid live-baseline result; inspect ELF.log for the runtime error" \
      "${project}.log" "add_note; worker outbox indexing; rebuild_qdrant; search_raw; concurrent writes; soak stability"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "fail" "runtime_failed" \
    "ELF same-corpus retrieval command failed in Docker" \
    "${project}.log" "add_note; worker outbox indexing; rebuild_qdrant; search_raw; concurrent writes; soak stability"
}

project_agentmemory() {
  local project="agentmemory"
  local repo="https://github.com/rohitg00/agentmemory.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local result_path="${REPORT_DIR}/${project}-search.json"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-agentmemory.ts"
  local head
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if run_cmd "${project}: install/build" 300 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && (npm ci || npm install --no-audit --no-fund) && npm run build --if-present"; then
    cat >"${driver_path}" <<'TS'
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { registerRememberFunction } from "./src/functions/remember.js";
import {
  getSearchIndex,
  registerSearchFunction,
  setEmbeddingProvider,
  setVectorIndex,
} from "./src/functions/search.js";

function mockKV() {
  const store = new Map<string, Map<string, unknown>>();
  return {
    get: async <T>(scope: string, key: string): Promise<T | null> =>
      (store.get(scope)?.get(key) as T) ?? null,
    set: async <T>(scope: string, key: string, data: T): Promise<T> => {
      if (!store.has(scope)) store.set(scope, new Map());
      store.get(scope)!.set(key, data);
      return data;
    },
    delete: async (scope: string, key: string): Promise<void> => {
      store.get(scope)?.delete(key);
    },
    list: async <T>(scope: string): Promise<T[]> => {
      const entries = store.get(scope);
      return entries ? (Array.from(entries.values()) as T[]) : [];
    },
  };
}

function mockSdk() {
  const functions = new Map<string, Function>();
  return {
    registerFunction: (idOrOpts: string | { id: string }, handler: Function) => {
      const id = typeof idOrOpts === "string" ? idOrOpts : idOrOpts.id;
      functions.set(id, handler);
    },
    registerTrigger: () => {},
    trigger: async (
      idOrInput: string | { function_id: string; payload: unknown },
      data?: unknown,
    ) => {
      const id = typeof idOrInput === "string" ? idOrInput : idOrInput.function_id;
      const payload = typeof idOrInput === "string" ? data : idOrInput.payload;
      const fn = functions.get(id);
      if (!fn) {
        if (id === "mem::cascade-update") return { success: true };
        throw new Error(`No function: ${id}`);
      }
      return fn(payload);
    },
  };
}

type QueryCase = {
  id: string;
  query: string;
  expected_doc: string;
  expected_terms: string[];
};

const outPath = process.argv[2];
const corpusPath = process.argv[3];
const queriesPath = process.argv[4];
if (!outPath || !corpusPath || !queriesPath) {
  throw new Error("output path, corpus path, and query path are required");
}

const sdk = mockSdk();
const kv = mockKV();
getSearchIndex().clear();
setVectorIndex(null);
setEmbeddingProvider(null);
registerRememberFunction(sdk as never, kv as never);
registerSearchFunction(sdk as never, kv as never);

function plainText(markdown: string): string {
  return markdown
    .split(/\r?\n/)
    .filter((line) => !line.trimStart().startsWith("#"))
    .join(" ")
    .replace(/\s+/g, " ")
    .trim();
}

function conceptsFor(file: string): string[] {
  return file
    .replace(/\.md$/i, "")
    .split(/[^A-Za-z0-9]+/)
    .map((part) => part.toLowerCase())
    .filter(Boolean);
}

function queryMatches(result: unknown, query: QueryCase): boolean {
  const results = (result as { results?: unknown[] }).results ?? [];
  return results.some((entry) => {
    const entryJson = JSON.stringify(entry);
    const entryText = entryJson.toLowerCase();
    const files =
      (entry as { observation?: { files?: string[] } }).observation?.files ?? [];
    return (
      files.includes(query.expected_doc) &&
      query.expected_terms.every((term) =>
        entryText.includes(term.toLowerCase()),
      )
    );
  });
}

function resultEntries(result: unknown): unknown[] {
  return (result as { results?: unknown[] }).results ?? [];
}

function makeCheck(
  name: string,
  status: "pass" | "fail" | "incomplete",
  reason: string,
  evidence: unknown,
) {
  return { name, status, reason, evidence };
}

function summarizeChecks(checks: Array<{ status: string }>) {
  return {
    total: checks.length,
    pass: checks.filter((check) => check.status === "pass").length,
    fail: checks.filter((check) => check.status === "fail").length,
    incomplete: checks.filter((check) => check.status === "incomplete").length,
  };
}

async function runSearch(query: QueryCase) {
  return sdk.trigger("mem::search", {
    query: query.query,
    limit: topK,
    format: "full",
    project: "elfbench",
  });
}

const docs = readdirSync(corpusPath)
  .filter((file) => file.endsWith(".md"))
  .sort()
  .map((file) => ({
    content: plainText(readFileSync(join(corpusPath, file), "utf8")),
    concepts: conceptsFor(file),
    files: [file],
  }));
const queries = JSON.parse(readFileSync(queriesPath, "utf8")).queries as QueryCase[];

const writes = [];
const memoryIdsBySource = new Map<string, string>();
for (const doc of docs) {
  const write = await sdk.trigger("mem::remember", {
    content: doc.content,
    type: "fact",
    concepts: doc.concepts,
    files: doc.files,
    project: "elfbench",
    agentId: "elf-baseline",
  });
  writes.push({ source: doc.files[0], result: write });
  const memoryId = (write as { memory?: { id?: string } }).memory?.id;
  if (memoryId) memoryIdsBySource.set(doc.files[0], memoryId);
}

const queryResults = [];
const topK = Number(process.env.ELF_BASELINE_TOP_K ?? "10");
for (const query of queries) {
  const result = await runSearch(query);
  queryResults.push({
    id: query.id,
    query: query.query,
    expected_doc: query.expected_doc,
    expected_terms: query.expected_terms,
    matched: queryMatches(result, query),
    result,
  });
}

const pass = queryResults.filter((result) => result.matched).length;
const checks = [
  makeCheck(
    "same_corpus_retrieval",
    pass === queryResults.length ? "pass" : "fail",
    pass === queryResults.length
      ? "agentmemory mem::remember/mem::search returned expected evidence for every query."
      : "agentmemory mem::remember/mem::search missed one or more expected results.",
    {
      total: queryResults.length,
      pass,
      fail: queryResults.length - pass,
    },
  ),
];

const authId = memoryIdsBySource.get("auth-memory.md");
if (!authId) {
  checks.push(
    makeCheck(
      "update_replaces_note_text",
      "incomplete",
      "The auth memory id was not returned by mem::remember, so supersede/update could not be exercised.",
      { source: "auth-memory.md" },
    ),
  );
} else {
  const updateRemember = await sdk.trigger("mem::remember", {
    content:
      "The API auth middleware validates JWT tokens with key id `kid-v4` under `RotatedJwtKeyPlan`. The middleware rejects tokens older than 15 minutes and requires tenant scope `project_shared` for deployment operations.",
    type: "fact",
    concepts: conceptsFor("auth-memory.md"),
    files: ["auth-memory.md"],
    project: "elfbench",
    agentId: "elf-baseline",
  });
  const updateQuery: QueryCase = {
    id: "lifecycle-update-new-marker",
    query: "Which rotated JWT key id does the auth middleware require?",
    expected_doc: "auth-memory.md",
    expected_terms: ["kid-v4", "RotatedJwtKeyPlan"],
  };
  const updateResult = await runSearch(updateQuery);
  const updateMatched = queryMatches(updateResult, updateQuery);
  const oldMarkerAbsent = resultEntries(updateResult)
    .filter((entry) => {
      const files =
        (entry as { observation?: { files?: string[] } }).observation?.files ?? [];
      return files.includes("auth-memory.md");
    })
    .every((entry) => !JSON.stringify(entry).toLowerCase().includes("kid-v3"));
  checks.push(
    makeCheck(
      "update_replaces_note_text",
      updateMatched && oldMarkerAbsent ? "pass" : "fail",
      updateMatched && oldMarkerAbsent
        ? "agentmemory mem::remember supersede returned the new marker and did not return the old marker for the updated file."
        : "agentmemory mem::remember supersede did not cleanly replace the searchable auth memory text.",
      {
        memory_id: authId,
        update_result: updateRemember,
        matched_new_marker: updateMatched,
        old_marker_absent: oldMarkerAbsent,
        result: updateResult,
      },
    ),
  );
}

const deleteQuery = queries.find(
  (query) =>
    query.expected_doc !== "auth-memory.md" &&
    query.expected_doc !== "database-memory.md" &&
    memoryIdsBySource.has(query.expected_doc),
);
if (!deleteQuery) {
  checks.push(
    makeCheck(
      "delete_suppresses_retrieval",
      "incomplete",
      "No non-update, non-recovery memory id was available, so mem::forget could not be exercised.",
      { available_sources: Array.from(memoryIdsBySource.keys()).sort() },
    ),
  );
} else {
  const deleteId = memoryIdsBySource.get(deleteQuery.expected_doc)!;
  const deleteResult = await sdk.trigger("mem::forget", { memoryId: deleteId });
  const searchAfterDelete = await runSearch(deleteQuery);
  const deletedStillMatched = queryMatches(searchAfterDelete, deleteQuery);
  checks.push(
    makeCheck(
      "delete_suppresses_retrieval",
      deletedStillMatched ? "fail" : "pass",
      deletedStillMatched
        ? "agentmemory mem::forget returned success but the deleted memory was still searchable."
        : "agentmemory mem::forget suppressed the deleted memory from subsequent search.",
      {
        memory_id: deleteId,
        source: deleteQuery.expected_doc,
        query: deleteQuery,
        delete_result: deleteResult,
        deleted_still_matched: deletedStillMatched,
        result: searchAfterDelete,
      },
    ),
  );
}

checks.push(
  makeCheck(
    "cold_start_recovery_search",
    "incomplete",
    "This adapter runs agentmemory against an in-memory SDK/KV mock; no durable store is available in the harness to prove cold-start recovery.",
    {
      adapter_storage: "mock StateKV Map",
      required_next_step: "wire an agentmemory persistent KV/index path or hosted runtime for restart testing",
    },
  ),
);

const checkSummary = summarizeChecks(checks);

writeFileSync(
  outPath,
  JSON.stringify(
    {
      schema: "elf.live_baseline.agentmemory_result/v1",
      corpus: {
        document_count: docs.length,
        query_count: queries.length,
      },
      writes,
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
TS
    if run_cmd "${project}: same-corpus remember/search" 240 "${log_path}" \
      "cd '${REPOS_DIR}/${project}' && npx tsx '${driver_path}' '${result_path}' '${CORPUS_DIR}' '${REPORT_DIR}/queries.json'"; then
      if jq -e '.checks and .check_summary' "${result_path}" >/dev/null 2>&1; then
        jq '{check_summary, checks}' "${result_path}" >"${REPORT_DIR}/${project}-checks.json"
      fi
      if jq -e --argjson query_count "${QUERY_COUNT}" --argjson document_count "${DOCUMENT_COUNT}" '
        .schema == "elf.live_baseline.agentmemory_result/v1" and
        .corpus.document_count == $document_count and
        .summary.total == $query_count and
        .summary.fail == 0 and
        .check_summary.fail == 0 and
        .check_summary.incomplete == 0
      ' "${result_path}" >/dev/null; then
        json_record "${project}" "${repo}" "${head}" "pass" "retrieval_pass" "agentmemory mem::remember/mem::search found expected evidence and lifecycle checks passed" "${project}.log" "npm install/build; mem::remember/mem::forget/mem::search"
        return
      fi
      if jq -e --argjson query_count "${QUERY_COUNT}" --argjson document_count "${DOCUMENT_COUNT}" '
        .schema == "elf.live_baseline.agentmemory_result/v1" and
        .corpus.document_count == $document_count and
        .summary.total == $query_count and
        .summary.fail == 0 and
        .check_summary.fail == 0
      ' "${result_path}" >/dev/null; then
        json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_pass" "agentmemory same-corpus retrieval passed, but one or more lifecycle checks could not be completed in the in-memory harness" "${project}.log" "npm install/build; mem::remember/mem::forget/mem::search"
        return
      fi
      if jq -e --argjson query_count "${QUERY_COUNT}" --argjson document_count "${DOCUMENT_COUNT}" '
        .schema == "elf.live_baseline.agentmemory_result/v1" and
        .corpus.document_count == $document_count and
        .summary.total == $query_count and
        .summary.fail == 0
      ' "${result_path}" >/dev/null; then
        json_record "${project}" "${repo}" "${head}" "fail" "retrieval_pass" "agentmemory same-corpus retrieval passed, but one or more lifecycle checks failed" "${project}.log" "npm install/build; mem::remember/mem::forget/mem::search"
        return
      fi
      json_record "${project}" "${repo}" "${head}" "fail" "retrieval_wrong_result" "agentmemory same-corpus search ran but did not return expected evidence" "${project}.log" "npm install/build; mem::remember; mem::search"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "agentmemory install/build passed but same-corpus remember/search failed" "${project}.log" "npm install/build; mem::remember; mem::search"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "fail" "not_run" "install/build failed" "${project}.log" "npm install/build"
}

project_qmd() {
  local project="qmd"
  local repo="https://github.com/tobi/qmd.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local query_result_path="${REPORT_DIR}/${project}-query.json"
  local status_path="${REPORT_DIR}/${project}-status.txt"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-qmd.mjs"
  local home="${HOME_DIR}/${project}"
  local head
  mkdir -p "${home}"
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install/build" 300 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && (npm ci || npm install --no-audit --no-fund) && npm run build --if-present"; then
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "install/build failed" "${project}.log" "npm install/build"
    return
  fi

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
    fail: checks.filter((check) => check.status === "fail").length,
    incomplete: checks.filter((check) => check.status === "incomplete").length,
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
    pass === queryResults.length ? "pass" : "fail",
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
      "incomplete",
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
      updateMatched && oldMarkerAbsent ? "pass" : "fail",
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
      "incomplete",
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
      deletedStillMatched ? "fail" : "pass",
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
    recoveryMatched ? "pass" : "fail",
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
    "export HOME='${home}'; export XDG_CACHE_HOME='/root/.cache'; export QMD_FORCE_CPU=1; cd '${REPOS_DIR}/${project}' && npx tsx src/cli/qmd.ts collection add '${CORPUS_DIR}' --name elfbench && npx tsx src/cli/qmd.ts update && npx tsx src/cli/qmd.ts embed -f -c elfbench && npx tsx src/cli/qmd.ts status > '${status_path}' && node '${driver_path}' '${query_result_path}' '${REPORT_DIR}/queries.json' '${CORPUS_DIR}'"; then
    if jq -e '.checks and .check_summary' "${query_result_path}" >/dev/null 2>&1; then
      jq '{check_summary, checks}' "${query_result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.qmd_result/v1" and
      .summary.total == $query_count and
      .summary.fail == 0 and
      .check_summary.fail == 0 and
      .check_summary.incomplete == 0
    ' "${query_result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "pass" "retrieval_pass" "qmd embedded structured hybrid query found expected evidence and lifecycle checks passed" "${project}.log" "collection add; update; embed -f; query --json"
    elif jq -e --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.qmd_result/v1" and
      .summary.total == $query_count and
      .summary.fail == 0
    ' "${query_result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "fail" "retrieval_pass" "qmd same-corpus retrieval passed, but one or more update/delete/recovery checks failed or were incomplete" "${project}.log" "collection add; update; embed -f; query --json"
    elif ! rg -q "Embedded [1-9][0-9]* chunks" "${log_path}"; then
      json_record "${project}" "${repo}" "${head}" "incomplete" "embedding_required" "qmd indexed the corpus, but no successful embedding completion was observed" "${project}.log" "collection add; update; embed -f; query --json"
    elif ! jq -e '.schema == "elf.live_baseline.qmd_result/v1"' "${query_result_path}" >/dev/null 2>&1; then
      json_record "${project}" "${repo}" "${head}" "fail" "invalid_json_result" "qmd query command completed, but did not produce parseable JSON results" "${project}.log" "collection add; update; embed -f; search/query --json"
    else
      json_record "${project}" "${repo}" "${head}" "fail" "retrieval_wrong_result" "qmd embedded retrieval ran but did not return expected evidence" "${project}.log" "collection add; update; embed -f; search/query --json"
    fi
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "qmd install passed but embedded retrieval command failed" "${project}.log" "collection add; update; embed -f; search/query --json"
}

project_memsearch() {
  local project="memsearch"
  local repo="https://github.com/zilliztech/memsearch.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local home="${HOME_DIR}/${project}"
  local result_path="${REPORT_DIR}/${project}-search.json"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-memsearch.py"
  local head
  mkdir -p "${home}"
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install" 420 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && python3 -m venv .venv && .venv/bin/pip install --upgrade pip && .venv/bin/pip install -e '.[local,onnx]'"; then
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "pip install failed" "${project}.log" "pip install -e .[local,onnx]"
    return
  fi

  cat >"${driver_path}" <<'PY'
import json
import os
import subprocess
from pathlib import Path

out_path = Path(os.environ["ELF_MEMSEARCH_RESULT_PATH"])
queries_path = Path(os.environ["ELF_BASELINE_QUERIES_PATH"])
corpus_path = Path(os.environ["ELF_BASELINE_CORPUS_PATH"])
top_k = os.environ.get("ELF_BASELINE_TOP_K", "10")
queries = json.loads(queries_path.read_text())["queries"]


def run_memsearch(args):
    return subprocess.run(
        ["memsearch", *args],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    ).stdout


def index_corpus():
    return run_memsearch(["index", str(corpus_path)])


def search_output(query_text):
    return run_memsearch(["search", query_text, "--top-k", top_k])


def output_matches(output, query):
    lowered = output.lower()
    matched = query["expected_doc"] in output and all(
        term.lower() in lowered for term in query["expected_terms"]
    )
    if not matched:
        matched = all(term.lower() in lowered for term in query["expected_terms"])
    return matched


def make_check(name, status, reason, evidence):
    return {
        "name": name,
        "status": status,
        "reason": reason,
        "evidence": evidence,
    }


def summarize_checks(checks):
    return {
        "total": len(checks),
        "pass": sum(1 for check in checks if check["status"] == "pass"),
        "fail": sum(1 for check in checks if check["status"] == "fail"),
        "incomplete": sum(1 for check in checks if check["status"] == "incomplete"),
    }


query_results = []
for query in queries:
    output = search_output(query["query"])
    matched = output_matches(output, query)
    query_results.append(
        {
            "id": query["id"],
            "query": query["query"],
            "expected_doc": query["expected_doc"],
            "expected_terms": query["expected_terms"],
            "matched": matched,
            "output": output,
        }
    )

pass_count = sum(1 for result in query_results if result["matched"])
checks = [
    make_check(
        "same_corpus_retrieval",
        "pass" if pass_count == len(query_results) else "fail",
        "memsearch search returned expected evidence for every query."
        if pass_count == len(query_results)
        else "memsearch search missed one or more expected results.",
        {
            "total": len(query_results),
            "pass": pass_count,
            "fail": len(query_results) - pass_count,
        },
    )
]

auth_path = corpus_path / "auth-memory.md"
if not auth_path.exists():
    checks.append(
        make_check(
            "update_replaces_note_text",
            "incomplete",
            "The auth corpus file was missing, so memsearch update could not be exercised.",
            {"source": "auth-memory.md"},
        )
    )
else:
    auth_path.write_text(
        "# Auth Memory\n\nRotated auth middleware validates JWT tokens with key id `kid-v4` under `RotatedJwtKeyPlan`. It still requires tenant scope `project_shared` for deployment operations after the emergency key rotation.\n"
    )
    update_index_output = index_corpus()
    update_query = {
        "id": "lifecycle-update-new-marker",
        "query": "Which rotated JWT key id does the auth middleware require?",
        "expected_doc": "auth-memory.md",
        "expected_terms": ["kid-v4", "RotatedJwtKeyPlan"],
    }
    update_output = search_output(update_query["query"])
    update_matched = output_matches(update_output, update_query)
    old_marker_absent = "kid-v3" not in update_output.lower()
    checks.append(
        make_check(
            "update_replaces_note_text",
            "pass" if update_matched and old_marker_absent else "fail",
            "memsearch re-index returned the new marker and did not return the old marker for the updated file."
            if update_matched and old_marker_absent
            else "memsearch re-index did not cleanly replace the searchable auth file text.",
            {
                "source": "auth-memory.md",
                "matched_new_marker": update_matched,
                "old_marker_absent": old_marker_absent,
                "index_output": update_index_output,
                "output": update_output,
            },
        )
    )

delete_query = next(
    (
        query
        for query in queries
        if query["expected_doc"] not in {"auth-memory.md", "database-memory.md"}
        and (corpus_path / query["expected_doc"]).exists()
    ),
    None,
)
if delete_query is None:
    checks.append(
        make_check(
            "delete_suppresses_retrieval",
            "incomplete",
            "No non-update, non-recovery corpus file was available, so memsearch delete could not be exercised.",
            {"available_docs": [query["expected_doc"] for query in queries]},
        )
    )
else:
    (corpus_path / delete_query["expected_doc"]).unlink()
    delete_index_output = index_corpus()
    delete_output = search_output(delete_query["query"])
    deleted_still_matched = output_matches(delete_output, delete_query)
    checks.append(
        make_check(
            "delete_suppresses_retrieval",
            "fail" if deleted_still_matched else "pass",
            "memsearch index removed the deleted file from subsequent search."
            if not deleted_still_matched
            else "memsearch index returned success but the deleted file was still searchable.",
            {
                "source": delete_query["expected_doc"],
                "query": delete_query,
                "deleted_still_matched": deleted_still_matched,
                "index_output": delete_index_output,
                "output": delete_output,
            },
        )
    )

recovery_query = {
    "id": "lifecycle-cold-start-recovery",
    "query": "The invoice list N+1 query was fixed by eager loading invoice lines through `InvoiceLineBatcher`. Do not reintroduce per-row SQL calls in invoice rendering.",
    "expected_doc": "database-memory.md",
    "expected_terms": ["InvoiceLineBatcher", "N+1"],
}
recovery_output = search_output(recovery_query["query"])
recovery_matched = output_matches(recovery_output, recovery_query)
checks.append(
    make_check(
        "cold_start_recovery_search",
        "pass" if recovery_matched else "fail",
        "A fresh memsearch CLI process reopened the local Milvus index and retrieved persisted evidence."
        if recovery_matched
        else "A fresh memsearch CLI process did not retrieve expected persisted evidence.",
        {
            "expected_doc": recovery_query["expected_doc"],
            "matched": recovery_matched,
            "output": recovery_output,
        },
    )
)

check_summary = summarize_checks(checks)
out_path.write_text(
    json.dumps(
        {
            "schema": "elf.live_baseline.memsearch_result/v1",
            "summary": {
                "total": len(query_results),
                "pass": pass_count,
                "fail": len(query_results) - pass_count,
            },
            "check_summary": check_summary,
            "checks": checks,
            "queries": query_results,
        },
        indent=2,
    )
)
PY

  if run_cmd "${project}: cli retrieval attempt" 240 "${log_path}" \
    "export HOME='${home}'; export ELF_MEMSEARCH_RESULT_PATH='${result_path}'; export ELF_BASELINE_QUERIES_PATH='${REPORT_DIR}/queries.json'; export ELF_BASELINE_CORPUS_PATH='${CORPUS_DIR}'; cd '${REPOS_DIR}/${project}' && source .venv/bin/activate && memsearch --help && memsearch config set embedding.provider onnx && memsearch index '${CORPUS_DIR}' && python '${driver_path}'"; then
    if jq -e '.checks and .check_summary' "${result_path}" >/dev/null 2>&1; then
      jq '{check_summary, checks}' "${result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.memsearch_result/v1" and
      .summary.total == $query_count and
      .summary.fail == 0 and
      .check_summary.fail == 0 and
      .check_summary.incomplete == 0
    ' "${result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "pass" "retrieval_pass" "memsearch indexed the corpus and returned expected evidence and lifecycle checks passed" "${project}.log" "config; index; search"
    elif jq -e --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.memsearch_result/v1" and
      .summary.total == $query_count and
      .summary.fail == 0
    ' "${result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "fail" "retrieval_pass" "memsearch same-corpus retrieval passed, but one or more update/delete/recovery checks failed or were incomplete" "${project}.log" "config; index; search"
    else
      json_record "${project}" "${repo}" "${head}" "fail" "retrieval_wrong_result" "memsearch search ran but did not return expected evidence" "${project}.log" "config; index; search"
    fi
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "memsearch installed, but the current CLI retrieval command failed" "${project}.log" "memsearch --help; config; index; search"
}

project_mem0() {
  local project="mem0"
  local repo="https://github.com/mem0ai/mem0.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local result_path="${REPORT_DIR}/${project}-search.json"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-mem0.py"
  local home="${HOME_DIR}/${project}"
  local head
  mkdir -p "${home}"
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install/import" 420 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && python3 -m venv .venv && .venv/bin/pip install --upgrade pip && .venv/bin/pip install -e . fastembed ollama && .venv/bin/python - <<'PY'
from mem0 import Memory
print('mem0 Memory import ok:', Memory)
PY"; then
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "pip install or import failed" "${project}.log" "pip install -e . fastembed ollama; import Memory"
    return
  fi

  cat >"${driver_path}" <<'PY'
import gc
import json
import os
from pathlib import Path

os.environ.setdefault("MEM0_TELEMETRY", "false")

from mem0 import Memory

out_path = Path(os.environ["ELF_MEM0_RESULT_PATH"])
base = Path(os.environ["ELF_MEM0_HOME"])
corpus_path = Path(os.environ["ELF_BASELINE_CORPUS_PATH"])
queries_path = Path(os.environ["ELF_BASELINE_QUERIES_PATH"])
top_k = int(os.environ.get("ELF_BASELINE_TOP_K", "10"))

config = {
    "vector_store": {
        "provider": "qdrant",
        "config": {
            "collection_name": "elfbench",
            "path": str(base / "qdrant"),
            "embedding_model_dims": 384,
        },
    },
    "embedder": {
        "provider": "fastembed",
        "config": {
            "model": "BAAI/bge-small-en-v1.5",
            "embedding_dims": 384,
        },
    },
    "llm": {
        "provider": "ollama",
        "config": {
            "model": "llama3.1:8b",
            "ollama_base_url": "http://127.0.0.1:11434",
        },
    },
    "history_db_path": str(base / "history.db"),
    "version": "v1.1",
}

memory = Memory.from_config(config)

def plain_text(markdown: str) -> str:
    return " ".join(
        line.strip()
        for line in markdown.splitlines()
        if not line.lstrip().startswith("#")
    ).strip()


docs = [
    (plain_text(path.read_text()), path.name)
    for path in sorted(corpus_path.glob("*.md"))
]
queries = json.loads(queries_path.read_text())["queries"]

adds = []
memory_ids_by_source = {}
for text, source in docs:
    added = memory.add(
        text,
        user_id="elf-bench",
        metadata={"source": source},
        infer=False,
    )
    adds.append({"source": source, "result": added})
    results = added.get("results", []) if isinstance(added, dict) else []
    if results and isinstance(results[0], dict) and results[0].get("id"):
        memory_ids_by_source[source] = results[0]["id"]


def result_entries(search):
    return search.get("results", []) if isinstance(search, dict) else []


def search_memory(memory_instance, query_text):
    return memory_instance.search(
        query_text,
        filters={"user_id": "elf-bench"},
        top_k=top_k,
        threshold=0.0,
    )


def matches_expected(search, expected_doc, expected_terms):
    for entry in result_entries(search):
        entry_text = json.dumps(entry, default=str).lower()
        source = ((entry.get("metadata") or {}).get("source") or "")
        if source == expected_doc and all(
            term.lower() in entry_text for term in expected_terms
        ):
            return True
    return False


def query_result(query, search):
    return {
        "id": query["id"],
        "query": query["query"],
        "expected_doc": query["expected_doc"],
        "expected_terms": query["expected_terms"],
        "matched": matches_expected(
            search,
            query["expected_doc"],
            query["expected_terms"],
        ),
        "search": search,
    }


def make_check(name, status, reason, evidence):
    return {
        "name": name,
        "status": status,
        "reason": reason,
        "evidence": evidence,
    }


def summarize_checks(checks):
    return {
        "total": len(checks),
        "pass": sum(1 for check in checks if check["status"] == "pass"),
        "fail": sum(1 for check in checks if check["status"] == "fail"),
        "incomplete": sum(1 for check in checks if check["status"] == "incomplete"),
    }

query_results = []
for query in queries:
    query_results.append(query_result(query, search_memory(memory, query["query"])))

pass_count = sum(1 for result in query_results if result["matched"])
checks = [
    make_check(
        "same_corpus_retrieval",
        "pass" if pass_count == len(query_results) else "fail",
        "mem0 local FastEmbed/Qdrant search returned expected evidence for every query."
        if pass_count == len(query_results)
        else "mem0 local FastEmbed/Qdrant search missed one or more expected results.",
        {
            "total": len(query_results),
            "pass": pass_count,
            "fail": len(query_results) - pass_count,
        },
    )
]

auth_id = memory_ids_by_source.get("auth-memory.md")
if not auth_id:
    checks.append(
        make_check(
            "update_replaces_note_text",
            "incomplete",
            "The auth memory id was not returned by mem0 add(), so update could not be exercised.",
            {"source": "auth-memory.md"},
        )
    )
else:
    update_text = (
        "Rotated auth middleware validates JWT tokens with key id `kid-v4` "
        "under `RotatedJwtKeyPlan`. It still requires tenant scope "
        "`project_shared` for deployment operations after the emergency key rotation."
    )
    update_result = memory.update(
        auth_id,
        update_text,
        metadata={"source": "auth-memory.md", "lifecycle": "updated"},
    )
    update_search = search_memory(
        memory,
        "Which rotated JWT key id does the auth middleware require?",
    )
    update_matched = matches_expected(
        update_search,
        "auth-memory.md",
        ["kid-v4", "RotatedJwtKeyPlan"],
    )
    old_marker_absent = all(
        "kid-v3" not in json.dumps(entry, default=str).lower()
        for entry in result_entries(update_search)
        if entry.get("id") == auth_id
        or ((entry.get("metadata") or {}).get("source") == "auth-memory.md")
    )
    checks.append(
        make_check(
            "update_replaces_note_text",
            "pass" if update_matched and old_marker_absent else "fail",
            "mem0 update() returned the new marker and did not return the old marker for the updated memory."
            if update_matched and old_marker_absent
            else "mem0 update() did not cleanly replace the searchable auth memory text.",
            {
                "memory_id": auth_id,
                "update_result": update_result,
                "matched_new_marker": update_matched,
                "old_marker_absent": old_marker_absent,
                "search": update_search,
            },
        )
    )

delete_query = next(
    (
        query
        for query in queries
        if query["expected_doc"] in memory_ids_by_source
        and query["expected_doc"] not in {"auth-memory.md", "database-memory.md"}
    ),
    None,
)
if delete_query is None:
    checks.append(
        make_check(
            "delete_suppresses_retrieval",
            "incomplete",
            "No non-update, non-recovery memory id was available, so delete could not be exercised.",
            {"available_sources": sorted(memory_ids_by_source)},
        )
    )
else:
    delete_source = delete_query["expected_doc"]
    delete_id = memory_ids_by_source[delete_source]
    delete_result = memory.delete(delete_id)
    delete_search = search_memory(
        memory,
        delete_query["query"],
    )
    deleted_still_matched = matches_expected(
        delete_search,
        delete_source,
        delete_query["expected_terms"],
    )
    checks.append(
        make_check(
            "delete_suppresses_retrieval",
            "pass" if not deleted_still_matched else "fail",
            "mem0 delete() suppressed the deleted memory from subsequent search."
            if not deleted_still_matched
            else "mem0 delete() returned success but the deleted memory was still searchable.",
            {
                "memory_id": delete_id,
                "source": delete_source,
                "query": delete_query,
                "delete_result": delete_result,
                "deleted_still_matched": deleted_still_matched,
                "search": delete_search,
            },
        )
    )

del memory
gc.collect()
reopened_memory = Memory.from_config(config)
recovery_search = search_memory(
    reopened_memory,
    "The invoice list N+1 query was fixed by eager loading invoice lines through `InvoiceLineBatcher`. Do not reintroduce per-row SQL calls in invoice rendering.",
)
recovery_matched = matches_expected(
    recovery_search,
    "database-memory.md",
    ["InvoiceLineBatcher", "N+1"],
)
checks.append(
    make_check(
        "cold_start_recovery_search",
        "pass" if recovery_matched else "fail",
        "A newly constructed mem0 Memory over the same local Qdrant/history paths retrieved persisted evidence."
        if recovery_matched
        else "A newly constructed mem0 Memory over the same local Qdrant/history paths did not retrieve persisted evidence.",
        {
            "expected_doc": "database-memory.md",
            "matched": recovery_matched,
            "search": recovery_search,
        },
    )
)

check_summary = summarize_checks(checks)

out_path.write_text(
    json.dumps(
        {
            "schema": "elf.live_baseline.mem0_result/v1",
            "config": {
                "embedder": "fastembed:BAAI/bge-small-en-v1.5",
                "vector_store": "qdrant:path",
                "infer": False,
            },
            "corpus": {
                "document_count": len(docs),
                "query_count": len(queries),
            },
            "adds": adds,
            "summary": {
                "total": len(query_results),
                "pass": pass_count,
                "fail": len(query_results) - pass_count,
            },
            "check_summary": check_summary,
            "checks": checks,
            "queries": query_results,
        },
        indent=2,
        default=str,
    )
)
PY

  if run_cmd "${project}: local fastembed add/search" 900 "${log_path}" \
    "export HOME='${home}'; export ELF_MEM0_HOME='${home}'; export ELF_MEM0_RESULT_PATH='${result_path}'; export ELF_BASELINE_CORPUS_PATH='${CORPUS_DIR}'; export ELF_BASELINE_QUERIES_PATH='${REPORT_DIR}/queries.json'; export MEM0_TELEMETRY=false; cd '${REPOS_DIR}/${project}' && source .venv/bin/activate && python '${driver_path}'"; then
    if jq -e '.checks and .check_summary' "${result_path}" >/dev/null 2>&1; then
      jq '{check_summary, checks}' "${result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" --argjson document_count "${DOCUMENT_COUNT}" '
      .schema == "elf.live_baseline.mem0_result/v1" and
      .corpus.document_count == $document_count and
      .summary.total == $query_count and
      .summary.fail == 0 and
      .check_summary.fail == 0 and
      .check_summary.incomplete == 0
    ' "${result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "pass" "retrieval_pass" "mem0 infer=false local fastembed/Qdrant search found expected evidence and lifecycle checks passed" "${project}.log" "pip install -e . fastembed ollama; Memory.from_config; add/update/delete/search"
      return
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" --argjson document_count "${DOCUMENT_COUNT}" '
      .schema == "elf.live_baseline.mem0_result/v1" and
      .corpus.document_count == $document_count and
      .summary.total == $query_count and
      .summary.fail == 0
    ' "${result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "fail" "retrieval_pass" "mem0 same-corpus retrieval passed, but one or more update/delete/recovery checks failed or were incomplete" "${project}.log" "pip install -e . fastembed ollama; Memory.from_config; add/update/delete/search"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "fail" "retrieval_wrong_result" "mem0 local add/search ran but did not return expected evidence" "${project}.log" "pip install -e . fastembed ollama; Memory.from_config; add infer=false; search"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "mem0 installed and imported, but local fastembed/Qdrant add/search failed" "${project}.log" "pip install -e . fastembed ollama; Memory.from_config; add infer=false; search"
}

project_openviking() {
  local project="OpenViking"
  local repo="https://github.com/volcengine/OpenViking.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local home="${HOME_DIR}/${project}"
  local config_path="${REPORT_DIR}/${project}-ov.conf"
  local result_path="${REPORT_DIR}/${project}-search.json"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-openviking.py"
  local local_embed_failure_pattern="llama-cpp-python|target specific option mismatch|failed-wheel-build-for-install|Failed building wheel|Failed to build llama-cpp-python|No module named 'llama_cpp'|Local embedding is enabled but 'llama-cpp-python' is not installed"
  local head
  mkdir -p "${home}"
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install/help" 600 "${log_path}" \
    "export HOME='${home}'; cd '${REPOS_DIR}/${project}' && python3 -m venv .venv && .venv/bin/pip install --upgrade pip && .venv/bin/pip install maturin && .venv/bin/pip install -e . && (.venv/bin/openviking language en || .venv/bin/ov language en) && (.venv/bin/openviking --help || .venv/bin/ov --help)"; then
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "pip install or CLI help failed" "${project}.log" "pip install -e .; openviking/ov --help"
    return
  fi

  if rg -q "ERROR: Failed building editable|Failed to build openviking|error: failed-wheel-build-for-install|CMake Error" "${log_path}"; then
    json_record "${project}" "${repo}" "${head}" "fail" "partial_install" "OpenViking install/help returned success but the build log contains native build errors" "${project}.log" "pip install -e .; openviking/ov --help"
    return
  fi

  cat >"${config_path}" <<EOF
{
  "default_account": "elfbench",
  "default_user": "elfbench",
  "storage": {
    "workspace": "${home}/data",
    "skip_process_lock": true,
    "vectordb": {
      "backend": "local",
      "name": "elfbench_context",
      "dimension": 512
    }
  },
  "embedding": {
    "dense": {
      "provider": "local",
      "model": "bge-small-zh-v1.5-f16",
      "cache_dir": "${home}/models"
    },
    "text_source": "content_only",
    "max_concurrent": 2
  },
  "auto_generate_l0": false,
  "auto_generate_l1": false,
  "default_search_mode": "fast",
  "vlm": {},
  "query_planner": {},
  "rerank": {}
}
EOF

  cat >"${driver_path}" <<'PY'
import json
import os
from pathlib import Path

from openviking import OpenViking


def to_jsonable(value):
    if hasattr(value, "to_dict"):
        return value.to_dict()
    if hasattr(value, "model_dump"):
        return value.model_dump()
    if isinstance(value, list):
        return [to_jsonable(item) for item in value]
    if isinstance(value, dict):
        return {key: to_jsonable(item) for key, item in value.items()}
    return value


out_path = Path(os.environ["ELF_OPENVIKING_RESULT_PATH"])
data_path = os.environ["ELF_OPENVIKING_DATA_PATH"]
corpus_path = os.environ["ELF_OPENVIKING_CORPUS_PATH"]
queries_path = Path(os.environ["ELF_BASELINE_QUERIES_PATH"])
top_k = int(os.environ.get("ELF_BASELINE_TOP_K", "10"))


def result_matches(found, query):
    raw = json.dumps(to_jsonable(found), ensure_ascii=False, default=str).lower()
    return query["expected_doc"].lower() in raw and all(
        term.lower() in raw for term in query["expected_terms"]
    )


client = OpenViking(path=data_path)
client.initialize()
try:
    queries = json.loads(queries_path.read_text())["queries"]
    added = client.add_resource(
        corpus_path,
        to="viking://resources/elfbench",
        wait=True,
        timeout=240,
        build_index=True,
        summarize=False,
    )
    query_results = []
    for query in queries:
        found = client.find(
            query["query"],
            target_uri="viking://resources/elfbench",
            limit=top_k,
            score_threshold=0.0,
            level=[2],
        )
        query_results.append(
            {
                "id": query["id"],
                "query": query["query"],
                "expected_doc": query["expected_doc"],
                "expected_terms": query["expected_terms"],
                "matched": result_matches(found, query),
                "find": to_jsonable(found),
            }
        )
    pass_count = sum(1 for result in query_results if result["matched"])
    out_path.write_text(
        json.dumps(
            {
                "schema": "elf.live_baseline.openviking_result/v1",
                "config": {
                    "embedder": "local:bge-small-zh-v1.5-f16",
                    "vector_store": "local",
                    "mode": "OpenViking.add_resource/find",
                },
                "add": to_jsonable(added),
                "summary": {
                    "total": len(query_results),
                    "pass": pass_count,
                    "fail": len(query_results) - pass_count,
                },
                "queries": query_results,
            },
            ensure_ascii=False,
            indent=2,
            default=str,
        )
    )
finally:
    client.close()
PY

  if ! run_cmd "${project}: install local embedding extras" 900 "${log_path}" \
    "export HOME='${home}'; cd '${REPOS_DIR}/${project}' && .venv/bin/pip install -e '.[local-embed]'"; then
    if rg -q "${local_embed_failure_pattern}" "${log_path}"; then
      json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "OpenViking local-embed install failed in Docker while building llama-cpp-python for aarch64, so same-corpus local retrieval could not be run" "${project}.log" "pip install -e .; openviking/ov --help; pip install -e .[local-embed]"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "OpenViking local-embed install failed in Docker, so same-corpus local retrieval could not be run" "${project}.log" "pip install -e .; openviking/ov --help; pip install -e .[local-embed]"
    return
  fi

  if rg -q "${local_embed_failure_pattern}" "${log_path}"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "OpenViking local-embed install returned success but the log contains llama-cpp-python build/import failure, so same-corpus local retrieval could not be run" "${project}.log" "pip install -e .; openviking/ov --help; pip install -e .[local-embed]"
    return
  fi

  if run_cmd "${project}: local add/find" 900 "${log_path}" \
    "export HOME='${home}'; export OPENVIKING_CONFIG_FILE='${config_path}'; export ELF_OPENVIKING_DATA_PATH='${home}/data'; export ELF_OPENVIKING_CORPUS_PATH='${CORPUS_DIR}'; export ELF_OPENVIKING_RESULT_PATH='${result_path}'; export ELF_BASELINE_QUERIES_PATH='${REPORT_DIR}/queries.json'; cd '${REPOS_DIR}/${project}' && source .venv/bin/activate && python '${driver_path}'"; then
    if rg -q "${local_embed_failure_pattern}" "${log_path}"; then
      json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "OpenViking local add_resource/find hit llama-cpp-python build/import failure, so same-corpus local retrieval could not be run" "${project}.log" "pip install -e .[local-embed]; OpenViking.add_resource/find"
      return
    fi
    if [[ ! -s "${result_path}" ]] || ! jq -e . "${result_path}" >/dev/null 2>&1; then
      json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "OpenViking local add_resource/find returned success but did not write a valid result JSON" "${project}.log" "pip install -e .[local-embed]; OpenViking.add_resource/find"
      return
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.openviking_result/v1" and
      .summary.total == $query_count and
      .summary.fail == 0
    ' "${result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "pass" "retrieval_pass" "OpenViking local add_resource/find found expected evidence for every query" "${project}.log" "pip install -e .[local-embed]; OpenViking.add_resource/find"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "fail" "retrieval_wrong_result" "OpenViking local add_resource/find ran but did not return expected evidence" "${project}.log" "pip install -e .[local-embed]; OpenViking.add_resource/find"
    return
  fi

  if rg -q "${local_embed_failure_pattern}" "${log_path}"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "OpenViking local add_resource/find failed because llama-cpp-python was unavailable in Docker" "${project}.log" "pip install -e .[local-embed]; OpenViking.add_resource/find"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "OpenViking local-embed installed, but same-corpus add_resource/find failed in Docker" "${project}.log" "pip install -e .[local-embed]; OpenViking.add_resource/find"
}

project_claude_mem() {
  local project="claude-mem"
  local repo="https://github.com/thedotmack/claude-mem.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local result_path="${REPORT_DIR}/${project}-search.json"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-claude-mem.ts"
  local head
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install/build" 420 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && (npm ci || npm install --no-audit --no-fund) && npm run build --if-present"; then
    json_record "${project}" "${repo}" "${head}" "fail" "not_run" "npm install/build failed" "${project}.log" "npm install/build"
    return
  fi

  cat >"${driver_path}" <<'TS'
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Database } from "bun:sqlite";
import { MemoryItemsRepository } from "./src/storage/sqlite/memory-items.ts";
import { ProjectsRepository } from "./src/storage/sqlite/projects.ts";

const outPath = Bun.argv[2];
const corpusPath = Bun.argv[3];
const queriesPath = Bun.argv[4];
if (!outPath || !corpusPath || !queriesPath) {
  throw new Error("output path, corpus path, and query path are required");
}

type QueryCase = {
  id: string;
  query: string;
  expected_doc: string;
  expected_terms: string[];
};

function plainText(markdown: string): string {
  return markdown
    .split(/\r?\n/)
    .filter((line) => !line.trimStart().startsWith("#"))
    .join(" ")
    .replace(/\s+/g, " ")
    .trim();
}

function titleFrom(markdown: string, file: string): string {
  const heading = markdown
    .split(/\r?\n/)
    .find((line) => line.trimStart().startsWith("# "));
  return heading ? heading.replace(/^#\s+/, "").trim() : file;
}

function conceptsFor(file: string): string[] {
  return file
    .replace(/\.md$/i, "")
    .split(/[^A-Za-z0-9]+/)
    .map((part) => part.toLowerCase())
    .filter(Boolean);
}

function resultMatches(results: unknown[], query: QueryCase): boolean {
  return results.some((entry) => {
    const files = (entry as { filesRead?: string[] }).filesRead ?? [];
    const entryText = JSON.stringify(entry).toLowerCase();
    return (
      files.includes(query.expected_doc) &&
      query.expected_terms.every((term) =>
        entryText.includes(term.toLowerCase()),
      )
    );
  });
}

const db = new Database(":memory:");
db.run("PRAGMA foreign_keys = ON");

try {
  const projects = new ProjectsRepository(db);
  const memories = new MemoryItemsRepository(db);
  const project = projects.create({
    name: "elfbench",
    slug: "elfbench",
    rootPath: "/bench/corpus",
    metadata: { source: "elf-live-baseline" },
  });

  const docs = readdirSync(corpusPath)
    .filter((file) => file.endsWith(".md"))
    .sort()
    .map((file) => {
      const raw = readFileSync(join(corpusPath, file), "utf8");
      return {
        title: titleFrom(raw, file),
        text: plainText(raw),
        concepts: conceptsFor(file),
        file,
      };
    });
  const queries = JSON.parse(readFileSync(queriesPath, "utf8")).queries as QueryCase[];
  const topK = Number(process.env.ELF_BASELINE_TOP_K ?? "10");

  const created = docs.map((doc) =>
    memories.create({
      projectId: project.id,
      kind: "manual",
      type: "fact",
      title: doc.title,
      text: doc.text,
      narrative: doc.text,
      facts: [doc.text],
      concepts: doc.concepts,
      filesRead: [doc.file],
      metadata: { source: doc.file },
    }),
  );

  const queryResults = queries.map((query) => {
    const results = memories.search(project.id, query.query, topK);
    return {
      id: query.id,
      query: query.query,
      expected_doc: query.expected_doc,
      expected_terms: query.expected_terms,
      matched: resultMatches(results, query),
      results,
    };
  });
  const pass = queryResults.filter((result) => result.matched).length;

  writeFileSync(
    outPath,
    JSON.stringify(
      {
        schema: "elf.live_baseline.claude_mem_result/v1",
        corpus: {
          document_count: docs.length,
          query_count: queries.length,
        },
        created,
        summary: {
          total: queryResults.length,
          pass,
          fail: queryResults.length - pass,
        },
        queries: queryResults,
      },
      null,
      2,
    ),
  );
} finally {
  db.close();
}
TS

  if run_cmd "${project}: same-corpus sqlite search" 300 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && bun '${driver_path}' '${result_path}' '${CORPUS_DIR}' '${REPORT_DIR}/queries.json'"; then
    if jq -e --argjson query_count "${QUERY_COUNT}" --argjson document_count "${DOCUMENT_COUNT}" '
      .schema == "elf.live_baseline.claude_mem_result/v1" and
      .corpus.document_count == $document_count and
      .summary.total == $query_count and
      .summary.fail == 0
    ' "${result_path}" >/dev/null; then
      json_record "${project}" "${repo}" "${head}" "pass" "retrieval_pass" "claude-mem SQLite memory repository search found expected evidence for every query" "${project}.log" "npm install/build; MemoryItemsRepository.create/search"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "fail" "retrieval_wrong_result" "claude-mem same-corpus search ran but did not return expected evidence" "${project}.log" "npm install/build; MemoryItemsRepository.create/search"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "claude-mem built, but same-corpus SQLite search did not pass in Docker" "${project}.log" "npm install/build; MemoryItemsRepository.create/search"
}

run_project "ELF" project_elf
run_project "agentmemory" project_agentmemory
run_project "qmd" project_qmd
run_project "memsearch" project_memsearch
run_project "mem0" project_mem0
run_project "OpenViking" project_openviking
run_project "claude-mem" project_claude_mem
finish_report

jq . "${REPORT}"
echo "Live baseline report: ${REPORT}"

if [[ "${ELF_BASELINE_STRICT:-0}" == "1" ]]; then
  jq -e '.verdict == "pass"' "${REPORT}" >/dev/null
fi
