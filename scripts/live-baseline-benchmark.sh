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
BACKFILL_DOC_COUNT="${ELF_BASELINE_BACKFILL_DOCS:-2000}"
QUERY_TOP_K="${ELF_BASELINE_TOP_K:-10}"
CURRENT_PROJECT_STARTED_AT=""
PRODUCTION_SYNTHETIC_MANIFEST="${ROOT_DIR}/apps/elf-eval/fixtures/production_corpus/synthetic_coding_agent_manifest.json"
CORPUS_TRACK="generated_public"
CORPUS_PATH_DESCRIPTION="generated in Docker under /bench/corpus"
CORPUS_MANIFEST_ID=""

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
      echo "${project} same-corpus retrieval and every encoded behavior check passed"
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
  python3 - "${CORPUS_PROFILE}" "${SCALE_DOC_COUNT}" "${STRESS_DOC_COUNT}" "${BACKFILL_DOC_COUNT}" "${CORPUS_DIR}" "${REPORT_DIR}/queries.json" <<'PY'
import json
import sys
from pathlib import Path

profile, scale_doc_count_raw, stress_doc_count_raw, backfill_doc_count_raw, corpus_dir_raw, queries_path_raw = sys.argv[1:]
corpus_dir = Path(corpus_dir_raw)
queries_path = Path(queries_path_raw)
scale_doc_count = int(scale_doc_count_raw)
stress_doc_count = int(stress_doc_count_raw)
backfill_doc_count = int(backfill_doc_count_raw)

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
elif profile in {"backfill", "large"}:
    docs = list(anchors)
    target_count = max(backfill_doc_count, len(anchors))
else:
    raise SystemExit(f"unsupported ELF_BASELINE_PROFILE={profile!r}")

if profile in {"scale", "full", "stress", "backfill", "large"}:
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
    evidence_id = doc["name"].replace(".md", "")
    queries.append(
        {
            "id": f"q-{base_id}",
            "task": "same_corpus_retrieval",
            "query": doc["query"],
            "expected_doc": doc["name"],
            "expected_terms": doc["terms"],
            "expected_evidence_ids": [evidence_id],
            "allowed_alternate_evidence_ids": [],
        }
    )
    if profile in {"stress", "backfill", "large"}:
        queries.append(
            {
                "id": f"q-{base_id}-alt",
                "task": "same_corpus_retrieval",
                "query": doc["alternate_query"],
                "expected_doc": doc["name"],
                "expected_terms": doc["terms"],
                "expected_evidence_ids": [evidence_id],
                "allowed_alternate_evidence_ids": [],
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

prepare_production_corpus() {
  local manifest_path="${ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST:-}"
  local corpus_summary="${REPORT_DIR}/production-corpus-summary.json"

  case "${CORPUS_PROFILE}" in
    production-synthetic)
      manifest_path="${manifest_path:-${PRODUCTION_SYNTHETIC_MANIFEST}}"
      ;;
    production-private)
      if [[ -z "${manifest_path}" ]]; then
        echo "ELF_BASELINE_PROFILE=production-private requires ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST." >&2
        exit 1
      fi
      ;;
    *)
      echo "Unsupported production corpus profile: ${CORPUS_PROFILE}" >&2
      exit 1
      ;;
  esac

  if [[ ! -f "${manifest_path}" ]]; then
    echo "Missing production corpus manifest: ${manifest_path}" >&2
    exit 1
  fi

  python3 - "${CORPUS_PROFILE}" "${manifest_path}" "${CORPUS_DIR}" "${REPORT_DIR}/queries.json" "${corpus_summary}" <<'PY'
import json
import re
import sys
from collections import Counter
from pathlib import Path

profile, manifest_path_raw, corpus_dir_raw, queries_path_raw, summary_path_raw = sys.argv[1:]
manifest_path = Path(manifest_path_raw)
corpus_dir = Path(corpus_dir_raw)
queries_path = Path(queries_path_raw)
summary_path = Path(summary_path_raw)
corpus_track = "synthetic_production" if profile == "production-synthetic" else "private_production"
allowed_categories = {
    "issue",
    "pr",
    "worktree",
    "runbook",
    "decision",
    "blocker",
    "recovery_note",
}
allowed_tasks = {
    "resume_lane",
    "recover_exact_command",
    "explain_stale_blocker",
    "find_prior_decision",
    "compare_project_status",
    "detect_contradiction_update",
}
id_re = re.compile(r"[a-z0-9][a-z0-9_.-]{1,80}")


def fail(message):
    raise SystemExit(f"Invalid production corpus manifest: {message}")


def require_string(obj, field, context):
    value = obj.get(field)
    if not isinstance(value, str) or not value.strip():
        fail(f"{context}.{field} must be a non-empty string")
    return value.strip()


def require_string_list(obj, field, context):
    value = obj.get(field)
    if not isinstance(value, list) or not value:
        fail(f"{context}.{field} must be a non-empty string array")
    out = []
    for index, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            fail(f"{context}.{field}[{index}] must be a non-empty string")
        out.append(item.strip())
    return out


def load_text(item, context):
    has_text = isinstance(item.get("text"), str)
    has_path = isinstance(item.get("local_path"), str)
    if has_text == has_path:
        fail(f"{context} must set exactly one of text or local_path")
    if has_text:
        text = item["text"].strip()
    else:
        local_path = Path(item["local_path"])
        if not local_path.is_absolute():
            local_path = manifest_path.parent / local_path
        if not local_path.is_file():
            fail(f"{context}.local_path does not point to a readable file")
        text = local_path.read_text(encoding="utf-8").strip()
    if not text:
        fail(f"{context} text must not be empty")
    if "\x00" in text:
        fail(f"{context} text contains a NUL byte")
    return text


manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if manifest.get("schema") != "elf.production_corpus_manifest/v1":
    fail("schema must be elf.production_corpus_manifest/v1")

manifest_id = require_string(manifest, "manifest_id", "$")
if not id_re.fullmatch(manifest_id):
    fail("$.manifest_id must be lower-case ASCII and safe for reports")
evidence_items = manifest.get("evidence")
if not isinstance(evidence_items, list) or not evidence_items:
    fail("$.evidence must be a non-empty array")
query_items = manifest.get("queries")
if not isinstance(query_items, list) or not query_items:
    fail("$.queries must be a non-empty array")

for existing in corpus_dir.glob("*.md"):
    existing.unlink()

evidence_by_id = {}
category_counts = Counter()
for index, item in enumerate(evidence_items):
    context = f"$.evidence[{index}]"
    if not isinstance(item, dict):
        fail(f"{context} must be an object")
    evidence_id = require_string(item, "evidence_id", context)
    if not id_re.fullmatch(evidence_id):
        fail(f"{context}.evidence_id must be lower-case ASCII and safe for filenames")
    if evidence_id in evidence_by_id:
        fail(f"{context}.evidence_id duplicates an earlier item")
    category = require_string(item, "category", context)
    if category not in allowed_categories:
        fail(f"{context}.category must be one of {sorted(allowed_categories)}")
    title = require_string(item, "title", context)
    text = load_text(item, context)
    evidence_by_id[evidence_id] = {
        "category": category,
        "title": title,
        "text": text,
    }
    category_counts[category] += 1
    (corpus_dir / f"{evidence_id}.md").write_text(
        "\n".join(
            [
                f"# {title}",
                "",
                text,
                "",
            ]
        ),
        encoding="utf-8",
    )

queries = []
query_ids = set()
task_counts = Counter()
for index, item in enumerate(query_items):
    context = f"$.queries[{index}]"
    if not isinstance(item, dict):
        fail(f"{context} must be an object")
    query_id = require_string(item, "query_id", context)
    if not id_re.fullmatch(query_id):
        fail(f"{context}.query_id must be lower-case ASCII and safe for reports")
    if query_id in query_ids:
        fail(f"{context}.query_id duplicates an earlier item")
    query_ids.add(query_id)
    task = require_string(item, "task", context)
    if task not in allowed_tasks:
        fail(f"{context}.task must be one of {sorted(allowed_tasks)}")
    query = require_string(item, "query", context)
    expected_ids = require_string_list(item, "expected_evidence_ids", context)
    allowed_alternate_ids = item.get("allowed_alternate_evidence_ids", [])
    if allowed_alternate_ids is None:
        allowed_alternate_ids = []
    if not isinstance(allowed_alternate_ids, list):
        fail(f"{context}.allowed_alternate_evidence_ids must be an array")
    allowed_alternate_ids = [
        evidence_id.strip()
        for evidence_id in allowed_alternate_ids
        if isinstance(evidence_id, str) and evidence_id.strip()
    ]
    expected_terms = require_string_list(item, "expected_terms", context)
    for evidence_id in [*expected_ids, *allowed_alternate_ids]:
        if evidence_id not in evidence_by_id:
            fail(f"{context} references unknown evidence_id {evidence_id!r}")
    queries.append(
        {
            "id": query_id,
            "task": task,
            "query": query,
            "expected_doc": f"{expected_ids[0]}.md",
            "allowed_alternate_docs": [
                f"{evidence_id}.md" for evidence_id in [*expected_ids[1:], *allowed_alternate_ids]
            ],
            "expected_terms": expected_terms,
            "expected_evidence_ids": expected_ids,
            "allowed_alternate_evidence_ids": allowed_alternate_ids,
        }
    )
    task_counts[task] += 1

queries_path.write_text(
    json.dumps(
        {
            "schema": "elf.live_baseline.queries/v1",
            "profile": profile,
            "corpus_track": corpus_track,
            "manifest_schema": manifest["schema"],
            "manifest_id": manifest_id,
            "document_count": len(evidence_by_id),
            "queries": queries,
        },
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)

summary_path.write_text(
    json.dumps(
        {
            "schema": "elf.production_corpus_summary/v1",
            "corpus_track": corpus_track,
            "manifest_schema": manifest["schema"],
            "manifest_id": manifest_id,
            "document_count": len(evidence_by_id),
            "query_count": len(queries),
            "category_counts": dict(sorted(category_counts.items())),
            "task_counts": dict(sorted(task_counts.items())),
            "evidence_ids": sorted(evidence_by_id),
            "query_evidence": [
                {
                    "query_id": query["id"],
                    "task": query["task"],
                    "expected_evidence_ids": query["expected_evidence_ids"],
                    "allowed_alternate_evidence_ids": query["allowed_alternate_evidence_ids"],
                }
                for query in queries
            ],
        },
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY

  CORPUS_TRACK="$(jq -r '.corpus_track' "${corpus_summary}")"
  CORPUS_MANIFEST_ID="$(jq -r '.manifest_id' "${corpus_summary}")"
  CORPUS_PATH_DESCRIPTION="production corpus materialized in Docker under /bench/corpus"
}

rm -rf "${WORK_DIR}"
mkdir -p "${REPORT_DIR}"
find "${REPORT_DIR}" -maxdepth 1 -type f -delete
mkdir -p "${REPOS_DIR}" "${CORPUS_DIR}" "${HOME_DIR}"
: >"${RECORDS}"

case "${CORPUS_PROFILE}" in
  production-synthetic | production-private)
    prepare_production_corpus
    ;;
  *)
    generate_corpus
    ;;
esac
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

project_agentmemory() {
  local project="agentmemory"
  local repo="https://github.com/rohitg00/agentmemory.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local result_path="${REPORT_DIR}/${project}-search.json"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-agentmemory.ts"
  local head
  cat >"${REPORT_DIR}/${project}-adapter.json" <<'JSON'
{
  "schema": "elf.live_baseline.adapter_metadata/v1",
  "project": "agentmemory",
  "storage": {
    "status": "mocked",
    "detail": "The harness registers agentmemory functions against in-memory SDK and KV mocks; it does not prove package durability."
  },
  "behaviors": {
    "same_corpus_retrieval": {
      "status": "mocked",
      "surface": "mem::remember and mem::search through an in-memory SDK/KV mock"
    },
    "update": {
      "status": "mocked",
      "surface": "superseding mem::remember through the in-memory mock"
    },
    "delete_or_expire": {
      "status": "mocked",
      "surface": "mem::forget through the in-memory mock; expiry is unsupported by this adapter"
    },
    "expire": {
      "status": "unsupported",
      "surface": "no TTL/expiry behavior is exposed by the encoded local adapter"
    },
    "cold_start_reload": {
      "status": "blocked",
      "surface": "no durable KV/index path is available in the Docker harness",
      "evidence": "The adapter state is a process-local Map and search index.",
      "retry": "Wire a persistent agentmemory KV/index path or hosted runtime, then restart a fresh process over that store."
    },
    "scale_stress_profile": {
      "status": "incomplete",
      "surface": "smoke adapter only until durable package behavior is available"
    }
  }
}
JSON
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "clone failed" "${project}.log" "git clone"
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
  status:
    | "pass"
    | "wrong_result"
    | "lifecycle_fail"
    | "incomplete"
    | "blocked"
    | "not_encoded",
  reason: string,
  evidence: unknown,
) {
  return { name, status, reason, evidence };
}

function summarizeChecks(checks: Array<{ status: string }>) {
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
    pass === queryResults.length ? "pass" : "wrong_result",
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
      updateMatched && oldMarkerAbsent ? "pass" : "lifecycle_fail",
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
      deletedStillMatched ? "lifecycle_fail" : "pass",
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
    "blocked",
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
        .summary.total == $query_count
      ' "${result_path}" >/dev/null; then
        local typed_status
        local retrieval_status
        typed_status="$(typed_status_from_result "${result_path}")"
        if jq -e '.summary.fail == 0' "${result_path}" >/dev/null; then
          retrieval_status="retrieval_pass"
        else
          retrieval_status="retrieval_wrong_result"
        fi
        json_record "${project}" "${repo}" "${head}" "${typed_status}" "${retrieval_status}" "$(typed_status_reason "${project}" "${typed_status}")" "${project}.log" "npm install/build; mem::remember/mem::forget/mem::search"
        return
      fi
      json_record "${project}" "${repo}" "${head}" "incomplete" "invalid_json_result" "agentmemory command completed, but did not produce a valid benchmark result" "${project}.log" "npm install/build; mem::remember; mem::search"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "agentmemory install/build passed but same-corpus remember/search failed" "${project}.log" "npm install/build; mem::remember; mem::search"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "install/build failed" "${project}.log" "npm install/build"
}

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

project_memsearch() {
  local project="memsearch"
  local repo="https://github.com/zilliztech/memsearch.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local home="${HOME_DIR}/${project}"
  local result_path="${REPORT_DIR}/${project}-search.json"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-memsearch.py"
  local corpus_path
  local head
  mkdir -p "${home}"
  cat >"${REPORT_DIR}/${project}-adapter.json" <<'JSON'
{
  "schema": "elf.live_baseline.adapter_metadata/v1",
  "project": "memsearch",
  "storage": {
    "status": "real",
    "detail": "The adapter uses memsearch CLI indexing and search with the local ONNX embedder inside Docker."
  },
  "behaviors": {
    "same_corpus_retrieval": {
      "status": "real",
      "surface": "memsearch index and memsearch search"
    },
    "update": {
      "status": "real",
      "surface": "rewrite corpus file, rerun memsearch index, and query for the replacement marker"
    },
    "delete_or_expire": {
      "status": "real",
      "surface": "delete corpus file, rerun memsearch index, and verify deleted evidence is not returned"
    },
    "expire": {
      "status": "unsupported",
      "surface": "the encoded CLI path supports reindex/delete but no TTL/expiry behavior"
    },
    "cold_start_reload": {
      "status": "real",
      "surface": "fresh memsearch CLI search process over the local index"
    },
    "scale_stress_profile": {
      "status": "incomplete",
      "surface": "smoke lifecycle path is encoded; scale/stress timing and resource thresholds are not yet calibrated"
    }
  }
}
JSON
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install" 420 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && python3 -m venv .venv && .venv/bin/pip install --upgrade pip && .venv/bin/pip install -e '.[local,onnx]'"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "pip install failed" "${project}.log" "pip install -e .[local,onnx]"
    return
  fi
  corpus_path="$(prepare_project_corpus "${project}")"

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
    wrong_result = sum(1 for check in checks if check["status"] == "wrong_result")
    lifecycle_fail = sum(1 for check in checks if check["status"] == "lifecycle_fail")
    return {
        "total": len(checks),
        "pass": sum(1 for check in checks if check["status"] == "pass"),
        "fail": wrong_result + lifecycle_fail,
        "wrong_result": wrong_result,
        "lifecycle_fail": lifecycle_fail,
        "incomplete": sum(1 for check in checks if check["status"] == "incomplete"),
        "blocked": sum(1 for check in checks if check["status"] == "blocked"),
        "not_encoded": sum(1 for check in checks if check["status"] == "not_encoded"),
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
        "pass" if pass_count == len(query_results) else "wrong_result",
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
            "not_encoded",
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
            "pass" if update_matched and old_marker_absent else "lifecycle_fail",
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
            "not_encoded",
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
            "lifecycle_fail" if deleted_still_matched else "pass",
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
        "pass" if recovery_matched else "lifecycle_fail",
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
    "export HOME='${home}'; export ELF_MEMSEARCH_RESULT_PATH='${result_path}'; export ELF_BASELINE_QUERIES_PATH='${REPORT_DIR}/queries.json'; export ELF_BASELINE_CORPUS_PATH='${corpus_path}'; cd '${REPOS_DIR}/${project}' && source .venv/bin/activate && memsearch --help && memsearch config set embedding.provider onnx && memsearch index '${corpus_path}' && python '${driver_path}'"; then
    if jq -e '.checks and .check_summary' "${result_path}" >/dev/null 2>&1; then
      jq '{check_summary, checks}' "${result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.memsearch_result/v1" and
      .summary.total == $query_count
    ' "${result_path}" >/dev/null; then
      local typed_status
      local retrieval_status
      typed_status="$(typed_status_from_result "${result_path}")"
      if jq -e '.summary.fail == 0' "${result_path}" >/dev/null; then
        retrieval_status="retrieval_pass"
      else
        retrieval_status="retrieval_wrong_result"
      fi
      json_record "${project}" "${repo}" "${head}" "${typed_status}" "${retrieval_status}" "$(typed_status_reason "${project}" "${typed_status}")" "${project}.log" "config; index; search"
    else
      json_record "${project}" "${repo}" "${head}" "incomplete" "invalid_json_result" "memsearch command completed, but did not produce a valid benchmark result" "${project}.log" "config; index; search"
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
  local corpus_path
  local head
  mkdir -p "${home}"
  cat >"${REPORT_DIR}/${project}-adapter.json" <<'JSON'
{
  "schema": "elf.live_baseline.adapter_metadata/v1",
  "project": "mem0",
  "storage": {
    "status": "real",
    "detail": "The adapter uses Memory.from_config with local FastEmbed, Qdrant path storage, and history DB paths inside Docker."
  },
  "behaviors": {
    "same_corpus_retrieval": {
      "status": "real",
      "surface": "Memory.add(infer=false) and Memory.search"
    },
    "update": {
      "status": "real",
      "surface": "Memory.update against the stored memory id"
    },
    "delete_or_expire": {
      "status": "real",
      "surface": "Memory.delete against the stored memory id"
    },
    "expire": {
      "status": "unsupported",
      "surface": "the encoded local Memory path does not expose TTL/expiry behavior"
    },
    "cold_start_reload": {
      "status": "real",
      "surface": "new Memory.from_config over the same local Qdrant/history paths"
    },
    "preference_history": {
      "status": "real",
      "surface": "Memory.history after a local preference correction update"
    },
    "entity_scope_personalization": {
      "status": "real",
      "surface": "Memory.add/search with user_id, agent_id, and run_id filters"
    },
    "deletion_audit": {
      "status": "real",
      "surface": "Memory.history after Memory.delete"
    },
    "local_export_readback": {
      "status": "real",
      "surface": "Memory.get_all over local OSS storage for inspection/export-style readback"
    },
    "openmemory_ui_export": {
      "status": "blocked",
      "surface": "the Docker live-baseline runner does not launch the OpenMemory web UI or hosted Platform export flow"
    },
    "scale_stress_profile": {
      "status": "incomplete",
      "surface": "smoke lifecycle path is encoded; scale/stress timing and resource thresholds are not yet calibrated"
    }
  }
}
JSON
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install/import" 420 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && python3 -m venv .venv && .venv/bin/pip install --upgrade pip && .venv/bin/pip install -e . fastembed ollama && .venv/bin/python - <<'PY'
from mem0 import Memory
print('mem0 Memory import ok:', Memory)
PY"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "pip install or import failed" "${project}.log" "pip install -e . fastembed ollama; import Memory"
    return
  fi
  corpus_path="$(prepare_project_corpus "${project}")"

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
    if isinstance(search, dict):
        for key in ("results", "memories"):
            entries = search.get(key)
            if isinstance(entries, list):
                return entries
    if isinstance(search, list):
        return search
    return []


def search_memory(memory_instance, query_text, filters=None):
    return memory_instance.search(
        query_text,
        filters=filters or {"user_id": "elf-bench"},
        top_k=top_k,
        threshold=0.0,
    )


def json_lower(value):
    return json.dumps(value, default=str).lower()


def contains_terms(value, terms):
    text = json_lower(value)
    return all(term.lower() in text for term in terms)


def first_memory_id(add_result):
    results = add_result.get("results", []) if isinstance(add_result, dict) else []
    if results and isinstance(results[0], dict):
        return results[0].get("id")
    return None


def memory_history(memory_instance, memory_id):
    if not hasattr(memory_instance, "history"):
        return {
            "available": False,
            "history": None,
            "error": "Memory.history is unavailable",
        }
    try:
        return {
            "available": True,
            "history": memory_instance.history(memory_id),
            "error": None,
        }
    except Exception as exc:
        return {
            "available": False,
            "history": None,
            "error": repr(exc),
        }


def get_all_memories(memory_instance, filters):
    if not hasattr(memory_instance, "get_all"):
        return {
            "available": False,
            "memories": None,
            "error": "Memory.get_all is unavailable",
        }
    try:
        return {
            "available": True,
            "memories": memory_instance.get_all(filters=filters),
            "error": None,
        }
    except TypeError:
        try:
            return {
                "available": True,
                "memories": memory_instance.get_all(
                    user_id=filters.get("user_id"),
                    agent_id=filters.get("agent_id"),
                    run_id=filters.get("run_id"),
                ),
                "error": None,
            }
        except Exception as exc:
            return {
                "available": False,
                "memories": None,
                "error": repr(exc),
            }
    except Exception as exc:
        return {
            "available": False,
            "memories": None,
            "error": repr(exc),
        }


def matches_expected(search, expected_doc, expected_terms):
    for entry in result_entries(search):
        entry_text = json_lower(entry)
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
    wrong_result = sum(1 for check in checks if check["status"] == "wrong_result")
    lifecycle_fail = sum(1 for check in checks if check["status"] == "lifecycle_fail")
    return {
        "total": len(checks),
        "pass": sum(1 for check in checks if check["status"] == "pass"),
        "fail": wrong_result + lifecycle_fail,
        "wrong_result": wrong_result,
        "lifecycle_fail": lifecycle_fail,
        "incomplete": sum(1 for check in checks if check["status"] == "incomplete"),
        "blocked": sum(1 for check in checks if check["status"] == "blocked"),
        "not_encoded": sum(1 for check in checks if check["status"] == "not_encoded"),
    }

query_results = []
for query in queries:
    query_results.append(query_result(query, search_memory(memory, query["query"])))

pass_count = sum(1 for result in query_results if result["matched"])
checks = [
    make_check(
        "same_corpus_retrieval",
        "pass" if pass_count == len(query_results) else "wrong_result",
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
            "not_encoded",
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
            "pass" if update_matched and old_marker_absent else "lifecycle_fail",
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

history_filters = {
    "user_id": "elf-history-user",
    "agent_id": "elf-history-agent",
    "run_id": "elf-project",
}
old_preference = (
    "Preference v1 for ELF: provide verbose tutorial explanations for every answer."
)
current_preference = (
    "Preference v2 for ELF: answer concisely with evidence-linked bullets."
)
preference_add = memory.add(
    old_preference,
    user_id=history_filters["user_id"],
    agent_id=history_filters["agent_id"],
    run_id=history_filters["run_id"],
    metadata={"source": "preference-history", "kind": "preference"},
    infer=False,
)
preference_id = first_memory_id(preference_add)
if not preference_id:
    checks.append(
        make_check(
            "preference_correction_history",
            "incomplete",
            "The preference memory id was not returned, so correction history could not be inspected.",
            {"add_result": preference_add},
        )
    )
else:
    preference_update = memory.update(
        preference_id,
        current_preference,
        metadata={"source": "preference-history", "kind": "preference"},
    )
    preference_history = memory_history(memory, preference_id)
    preference_search = search_memory(
        memory,
        "How should answers be written for the ELF project?",
        history_filters,
    )
    history_has_old = contains_terms(preference_history["history"], ["verbose tutorial"])
    history_has_current = contains_terms(
        preference_history["history"],
        ["concise", "evidence-linked"],
    )
    search_has_current = contains_terms(
        result_entries(preference_search),
        ["concise", "evidence-linked"],
    )
    search_omits_old = "verbose tutorial" not in json_lower(result_entries(preference_search))
    if not preference_history["available"]:
        preference_status = "blocked"
        preference_reason = "Memory.history could not be read for the updated preference memory."
    elif history_has_old and history_has_current and search_has_current and search_omits_old:
        preference_status = "pass"
        preference_reason = "mem0 history preserved the old and current preference while search returned only the current correction."
    else:
        preference_status = "lifecycle_fail"
        preference_reason = "mem0 did not expose a clean preference correction chain with current-only search readback."
    checks.append(
        make_check(
            "preference_correction_history",
            preference_status,
            preference_reason,
            {
                "memory_id": preference_id,
                "add_result": preference_add,
                "update_result": preference_update,
                "history_available": preference_history["available"],
                "history_error": preference_history["error"],
                "history_has_old": history_has_old,
                "history_has_current": history_has_current,
                "search_has_current": search_has_current,
                "search_omits_old": search_omits_old,
                "history": preference_history["history"],
                "search": preference_search,
            },
        )
    )

other_scope_add = memory.add(
    "Preference for PubFi: answer in long-form Chinese prose with no bullets.",
    user_id=history_filters["user_id"],
    agent_id=history_filters["agent_id"],
    run_id="pubfi-project",
    metadata={"source": "pubfi-preference", "kind": "preference"},
    infer=False,
)
entity_search = search_memory(
    memory,
    "What answer style preference applies here?",
    history_filters,
)
entity_search_text = json_lower(result_entries(entity_search))
entity_has_current = "evidence-linked bullets" in entity_search_text
entity_omits_other = "long-form chinese" not in entity_search_text
checks.append(
    make_check(
        "entity_scoped_personalization",
        "pass" if entity_has_current and entity_omits_other else "lifecycle_fail",
        "mem0 search respected user_id, agent_id, and run_id filters for the current preference scope."
        if entity_has_current and entity_omits_other
        else "mem0 entity-scoped search did not isolate the current preference from another run/project scope.",
        {
            "current_memory_id": preference_id,
            "other_scope_add": other_scope_add,
            "filters": history_filters,
            "has_current": entity_has_current,
            "omits_other_scope": entity_omits_other,
            "search": entity_search,
        },
    )
)

export_readback = get_all_memories(memory, history_filters)
export_has_current = contains_terms(
    export_readback["memories"],
    ["concise", "evidence-linked"],
)
export_omits_other = "long-form chinese" not in json_lower(export_readback["memories"])
if not export_readback["available"]:
    export_status = "blocked"
    export_reason = "Memory.get_all could not be read for local OSS inspection/export-style evidence."
elif export_has_current and export_omits_other:
    export_status = "pass"
    export_reason = "mem0 get_all returned local export-style readback for the current scoped preference without the other scope."
else:
    export_status = "lifecycle_fail"
    export_reason = "mem0 get_all did not return the current scoped preference cleanly for local export-style readback."
checks.append(
    make_check(
        "local_get_all_export_readback",
        export_status,
        export_reason,
        {
            "available": export_readback["available"],
            "error": export_readback["error"],
            "filters": history_filters,
            "has_current": export_has_current,
            "omits_other_scope": export_omits_other,
            "memories": export_readback["memories"],
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
            "not_encoded",
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
            "pass" if not deleted_still_matched else "lifecycle_fail",
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
    delete_history = memory_history(memory, delete_id)
    delete_history_has_event = delete_history["available"] and contains_terms(
        delete_history["history"],
        ["delete"],
    )
    if not delete_history["available"]:
        delete_audit_status = "blocked"
        delete_audit_reason = "Memory.history could not be read after delete, so deletion audit readback is blocked."
    elif delete_history_has_event and not deleted_still_matched:
        delete_audit_status = "pass"
        delete_audit_reason = "mem0 history exposed a delete event and search suppressed the deleted memory."
    else:
        delete_audit_status = "lifecycle_fail"
        delete_audit_reason = "mem0 did not expose a delete audit event while suppressing the deleted memory."
    checks.append(
        make_check(
            "delete_history_audit_readback",
            delete_audit_status,
            delete_audit_reason,
            {
                "memory_id": delete_id,
                "source": delete_source,
                "history_available": delete_history["available"],
                "history_error": delete_history["error"],
                "history_has_delete_event": delete_history_has_event,
                "deleted_still_matched": deleted_still_matched,
                "history": delete_history["history"],
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
        "pass" if recovery_matched else "lifecycle_fail",
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
    "export HOME='${home}'; export ELF_MEM0_HOME='${home}'; export ELF_MEM0_RESULT_PATH='${result_path}'; export ELF_BASELINE_CORPUS_PATH='${corpus_path}'; export ELF_BASELINE_QUERIES_PATH='${REPORT_DIR}/queries.json'; export MEM0_TELEMETRY=false; cd '${REPOS_DIR}/${project}' && source .venv/bin/activate && python '${driver_path}'"; then
    if jq -e '.checks and .check_summary' "${result_path}" >/dev/null 2>&1; then
      jq '{check_summary, checks}' "${result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" --argjson document_count "${DOCUMENT_COUNT}" '
      .schema == "elf.live_baseline.mem0_result/v1" and
      .corpus.document_count == $document_count and
      .summary.total == $query_count
    ' "${result_path}" >/dev/null; then
      local typed_status
      local retrieval_status
      typed_status="$(typed_status_from_result "${result_path}")"
      if jq -e '.summary.fail == 0' "${result_path}" >/dev/null; then
        retrieval_status="retrieval_pass"
      else
        retrieval_status="retrieval_wrong_result"
      fi
      json_record "${project}" "${repo}" "${head}" "${typed_status}" "${retrieval_status}" "$(typed_status_reason "${project}" "${typed_status}")" "${project}.log" "pip install -e . fastembed ollama; Memory.from_config; add/update/delete/history/get_all/search"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "incomplete" "invalid_json_result" "mem0 command completed, but did not produce a valid benchmark result" "${project}.log" "pip install -e . fastembed ollama; Memory.from_config; add infer=false; search"
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
  local constraints_path="${REPORT_DIR}/${project}-constraints.txt"
  local llama_cpp_python_version="${ELF_BASELINE_OPENVIKING_LLAMA_CPP_PYTHON_VERSION:-0.3.28}"
  local llama_cpp_python_index="${ELF_BASELINE_OPENVIKING_LLAMA_CPP_PYTHON_INDEX:-https://abetlen.github.io/llama-cpp-python/whl/cpu}"
  local local_embed_failure_pattern="target specific option mismatch|failed-wheel-build-for-install|Failed building wheel for llama-cpp-python|Failed to build llama-cpp-python|Could not build wheels for llama-cpp-python|No module named 'llama_cpp'|Local embedding is enabled but 'llama-cpp-python' is not installed|No matching distribution found|Could not find a version that satisfies|not a supported wheel"
  local local_embed_install_reason="OpenViking local-embed install failed in Docker for pinned llama-cpp-python==${llama_cpp_python_version} from the CPU wheel index, so same-corpus local retrieval could not be run"
  local local_embed_command_summary="pip install -e .; openviking/ov --help; pip install llama-cpp-python==${llama_cpp_python_version} --extra-index-url ${llama_cpp_python_index} --only-binary llama-cpp-python; pip install -e .[local-embed]; OpenViking.add_resource/find"
  local head
  mkdir -p "${home}"
  cat >"${REPORT_DIR}/${project}-adapter.json" <<JSON
{
  "schema": "elf.live_baseline.adapter_metadata/v1",
  "project": "OpenViking",
  "storage": {
    "status": "real",
    "detail": "The adapter uses OpenViking local storage after pinning the Docker local embedding dependency path."
  },
  "behaviors": {
    "same_corpus_retrieval": {
      "status": "real",
      "surface": "OpenViking.add_resource and OpenViking.find after installing .[local-embed] with llama-cpp-python==${llama_cpp_python_version} from the CPU wheel index",
      "evidence": "The Docker dependency boundary is the local llama-cpp-python wheel/import path, not provider-backed ELF embeddings. Once setup reaches add_resource/find, misses are reported as wrong_result.",
      "retry": "Retry with ELF_BASELINE_PROJECTS=OpenViking cargo make baseline-live-docker; override ELF_BASELINE_OPENVIKING_LLAMA_CPP_PYTHON_VERSION or ELF_BASELINE_OPENVIKING_LLAMA_CPP_PYTHON_INDEX only when the pinned CPU wheel is unavailable for the Docker platform. Treat wheel install/import failures as incomplete, not wrong_result."
    },
    "update": {
      "status": "not_encoded",
      "surface": "no update replacement check is encoded for OpenViking"
    },
    "delete_or_expire": {
      "status": "not_encoded",
      "surface": "no delete or expiry check is encoded for OpenViking"
    },
    "expire": {
      "status": "unsupported",
      "surface": "no TTL/expiry behavior is encoded in the local adapter"
    },
    "cold_start_reload": {
      "status": "not_encoded",
      "surface": "no restart/reopen check is encoded until local same-corpus retrieval completes"
    },
    "scale_stress_profile": {
      "status": "blocked",
      "surface": "scale/stress is blocked until smoke same-corpus retrieval returns evidence-bearing results"
    }
  }
}
JSON
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install/help" 600 "${log_path}" \
    "export HOME='${home}'; cd '${REPOS_DIR}/${project}' && python3 -m venv .venv && .venv/bin/pip install --upgrade pip && .venv/bin/pip install maturin && .venv/bin/pip install -e . && (.venv/bin/openviking language en || .venv/bin/ov language en) && (.venv/bin/openviking --help || .venv/bin/ov --help)"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "pip install or CLI help failed" "${project}.log" "pip install -e .; openviking/ov --help"
    return
  fi

  if rg -q "ERROR: Failed building editable|Failed to build openviking|error: failed-wheel-build-for-install|CMake Error" "${log_path}"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "partial_install" "OpenViking install/help returned success but the build log contains native build errors" "${project}.log" "pip install -e .; openviking/ov --help"
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
    checks = [
        {
            "name": "same_corpus_retrieval",
            "status": "pass" if pass_count == len(query_results) else "wrong_result",
            "reason": "OpenViking find returned expected evidence for every query."
            if pass_count == len(query_results)
            else "OpenViking find missed one or more expected results.",
            "evidence": {
                "total": len(query_results),
                "pass": pass_count,
                "fail": len(query_results) - pass_count,
            },
        },
        {
            "name": "update_replaces_note_text",
            "status": "not_encoded",
            "reason": "OpenViking update replacement is not encoded in this Docker adapter.",
            "evidence": {},
        },
        {
            "name": "delete_suppresses_retrieval",
            "status": "not_encoded",
            "reason": "OpenViking delete or expiry behavior is not encoded in this Docker adapter.",
            "evidence": {},
        },
        {
            "name": "cold_start_recovery_search",
            "status": "not_encoded",
            "reason": "OpenViking cold-start reload is not encoded until the local retrieval path is stable in Docker.",
            "evidence": {},
        },
    ]
    wrong_result_count = sum(
        1 for check in checks if check["status"] == "wrong_result"
    )
    lifecycle_fail_count = sum(
        1 for check in checks if check["status"] == "lifecycle_fail"
    )
    check_summary = {
        "total": len(checks),
        "pass": sum(1 for check in checks if check["status"] == "pass"),
        "fail": wrong_result_count + lifecycle_fail_count,
        "wrong_result": wrong_result_count,
        "lifecycle_fail": lifecycle_fail_count,
        "incomplete": sum(1 for check in checks if check["status"] == "incomplete"),
        "blocked": sum(1 for check in checks if check["status"] == "blocked"),
        "not_encoded": sum(1 for check in checks if check["status"] == "not_encoded"),
    }
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
                "check_summary": check_summary,
                "checks": checks,
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

  if ! run_cmd "${project}: install pinned local embedding extras" 900 "${log_path}" \
    "export HOME='${home}'; cd '${REPOS_DIR}/${project}' && printf 'llama-cpp-python==${llama_cpp_python_version}\n' > '${constraints_path}' && .venv/bin/pip install --extra-index-url '${llama_cpp_python_index}' --only-binary llama-cpp-python -c '${constraints_path}' 'llama-cpp-python==${llama_cpp_python_version}' && .venv/bin/pip install --extra-index-url '${llama_cpp_python_index}' --only-binary llama-cpp-python -c '${constraints_path}' -e '.[local-embed]' && .venv/bin/python - <<'PY'
import llama_cpp

print('llama_cpp_import_ok', getattr(llama_cpp, '__version__', 'unknown'))
PY"; then
    if rg -q "${local_embed_failure_pattern}" "${log_path}"; then
      json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "${local_embed_install_reason}" "${project}.log" "${local_embed_command_summary}"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "${local_embed_install_reason}" "${project}.log" "${local_embed_command_summary}"
    return
  fi

  if rg -q "${local_embed_failure_pattern}" "${log_path}"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "OpenViking pinned local-embed install returned success but the log contains llama-cpp-python wheel/import failure, so same-corpus local retrieval could not be run" "${project}.log" "${local_embed_command_summary}"
    return
  fi

  if run_cmd "${project}: local add/find" 900 "${log_path}" \
    "export HOME='${home}'; export OPENVIKING_CONFIG_FILE='${config_path}'; export ELF_OPENVIKING_DATA_PATH='${home}/data'; export ELF_OPENVIKING_CORPUS_PATH='${CORPUS_DIR}'; export ELF_OPENVIKING_RESULT_PATH='${result_path}'; export ELF_BASELINE_QUERIES_PATH='${REPORT_DIR}/queries.json'; cd '${REPOS_DIR}/${project}' && source .venv/bin/activate && python '${driver_path}'"; then
    if jq -e '.checks and .check_summary' "${result_path}" >/dev/null 2>&1; then
      jq '{check_summary, checks}' "${result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if rg -q "${local_embed_failure_pattern}" "${log_path}"; then
      json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "OpenViking local add_resource/find hit pinned llama-cpp-python wheel/import failure, so same-corpus local retrieval could not be run" "${project}.log" "${local_embed_command_summary}"
      return
    fi
    if [[ ! -s "${result_path}" ]] || ! jq -e . "${result_path}" >/dev/null 2>&1; then
      json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "OpenViking local add_resource/find returned success but did not write a valid result JSON" "${project}.log" "${local_embed_command_summary}"
      return
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" '
      .schema == "elf.live_baseline.openviking_result/v1" and
      .summary.total == $query_count
    ' "${result_path}" >/dev/null; then
      local typed_status
      local retrieval_status
      typed_status="$(typed_status_from_result "${result_path}")"
      if jq -e '.summary.fail == 0' "${result_path}" >/dev/null; then
        retrieval_status="retrieval_pass"
      else
        retrieval_status="retrieval_wrong_result"
      fi
      json_record "${project}" "${repo}" "${head}" "${typed_status}" "${retrieval_status}" "$(typed_status_reason "${project}" "${typed_status}")" "${project}.log" "${local_embed_command_summary}"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "incomplete" "invalid_json_result" "OpenViking local add_resource/find did not produce a valid benchmark result" "${project}.log" "${local_embed_command_summary}"
    return
  fi

  if rg -q "${local_embed_failure_pattern}" "${log_path}"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "local_embed_install_failed" "OpenViking local add_resource/find failed because pinned llama-cpp-python was unavailable in Docker" "${project}.log" "${local_embed_command_summary}"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "OpenViking pinned local-embed installed, but same-corpus add_resource/find failed in Docker" "${project}.log" "${local_embed_command_summary}"
}

project_claude_mem() {
  local project="claude-mem"
  local repo="https://github.com/thedotmack/claude-mem.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local result_path="${REPORT_DIR}/${project}-search.json"
  local driver_path="${REPOS_DIR}/${project}/elf-live-baseline-claude-mem.ts"
  local home="${HOME_DIR}/${project}"
  local corpus_path
  local db_path="${HOME_DIR}/${project}/claude-mem.sqlite"
  local head
  mkdir -p "${home}"
  cat >"${REPORT_DIR}/${project}-adapter.json" <<'JSON'
{
  "schema": "elf.live_baseline.adapter_metadata/v1",
  "project": "claude-mem",
  "storage": {
    "status": "real",
    "detail": "The adapter uses claude-mem repository classes with a durable SQLite file inside Docker for same-corpus and lifecycle checks."
  },
  "behaviors": {
    "same_corpus_retrieval": {
      "status": "real",
      "surface": "MemoryItemsRepository.create/search over a Docker-local SQLite database"
    },
    "update": {
      "status": "real",
      "surface": "MemoryItemsRepository.update against the stored memory item id"
    },
    "delete_or_expire": {
      "status": "real",
      "surface": "delete from the repository-owned SQLite memory_items table and verify FTS suppression"
    },
    "expire": {
      "status": "unsupported",
      "surface": "no TTL/expiry behavior is encoded in the local adapter"
    },
    "cold_start_reload": {
      "status": "real",
      "surface": "new Database and repository instances over the same Docker-local SQLite file"
    },
    "progressive_disclosure": {
      "status": "real",
      "surface": "search returns bounded memory items and detail/source hydration uses getById plus listSources"
    },
    "scale_stress_profile": {
      "status": "incomplete",
      "surface": "durable smoke lifecycle path is encoded; scale/stress timing and resource thresholds are not yet calibrated"
    }
  }
}
JSON
  head="$(clone_project "${project}" "${repo}" "${log_path}")" || {
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "clone failed" "${project}.log" "git clone"
    return
  }

  if ! run_cmd "${project}: install/build" 420 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && (npm ci || npm install --no-audit --no-fund) && npm run build --if-present"; then
    json_record "${project}" "${repo}" "${head}" "incomplete" "not_run" "npm install/build failed" "${project}.log" "npm install/build"
    return
  fi
  corpus_path="$(prepare_project_corpus "${project}")"

  cat >"${driver_path}" <<'TS'
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Database } from "bun:sqlite";
import { MemoryItemsRepository } from "./src/storage/sqlite/memory-items.ts";
import { ProjectsRepository } from "./src/storage/sqlite/projects.ts";

const outPath = Bun.argv[2];
const corpusPath = Bun.argv[3];
const queriesPath = Bun.argv[4];
const dbPath = Bun.argv[5];
if (!outPath || !corpusPath || !queriesPath || !dbPath) {
  throw new Error("output path, corpus path, query path, and database path are required");
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

function resultEntriesForSource(results: unknown[], source: string): unknown[] {
  return results.filter((entry) => {
    const files = (entry as { filesRead?: string[] }).filesRead ?? [];
    return files.includes(source);
  });
}

function makeCheck(
  name: string,
  status:
    | "pass"
    | "wrong_result"
    | "lifecycle_fail"
    | "incomplete"
    | "blocked"
    | "not_encoded",
  reason: string,
  evidence: unknown,
) {
  return { name, status, reason, evidence };
}

function summarizeChecks(checks: Array<{ status: string }>) {
  const wrongResult = checks.filter((check) => check.status === "wrong_result")
    .length;
  const lifecycleFail = checks.filter(
    (check) => check.status === "lifecycle_fail",
  ).length;
  return {
    total: checks.length,
    pass: checks.filter((check) => check.status === "pass").length,
    fail: wrongResult + lifecycleFail,
    wrong_result: wrongResult,
    lifecycle_fail: lifecycleFail,
    incomplete: checks.filter((check) => check.status === "incomplete").length,
    blocked: checks.filter((check) => check.status === "blocked").length,
    not_encoded: checks.filter((check) => check.status === "not_encoded")
      .length,
  };
}

function markerQuery(query: QueryCase): string {
  return query.expected_terms.join(" ");
}

const db = new Database(dbPath);
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

  const created = [];
  const createdBySource = new Map<string, ReturnType<MemoryItemsRepository["create"]>>();
  for (const doc of docs) {
    const item = memories.create({
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
    });
    const source = memories.addSource({
      memoryItemId: item.id,
      sourceType: "import",
      sourceUri: `file://${doc.file}`,
      metadata: { source: doc.file },
    });
    created.push({ item, source });
    createdBySource.set(doc.file, item);
  }

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
  const checks = [
    makeCheck(
      "same_corpus_retrieval",
      pass === queryResults.length ? "pass" : "wrong_result",
      pass === queryResults.length
        ? "claude-mem repository search returned expected evidence for every query."
        : "claude-mem repository search missed one or more expected results.",
      {
        total: queryResults.length,
        pass,
        fail: queryResults.length - pass,
      },
    ),
  ];

  const auth = createdBySource.get("auth-memory.md");
  if (!auth) {
    checks.push(
      makeCheck(
        "update_replaces_note_text",
        "incomplete",
        "The auth memory item was not created, so update replacement could not be exercised.",
        { source: "auth-memory.md" },
      ),
    );
  } else {
    const updateText =
      "Rotated auth middleware validates JWT tokens with key id `kid-v4` under `RotatedJwtKeyPlan`. It still requires tenant scope `project_shared` for deployment operations after the emergency key rotation.";
    const update = memories.update(auth.id, {
      title: "Auth Memory Updated",
      text: updateText,
      narrative: updateText,
      facts: [updateText],
      concepts: conceptsFor("auth-memory.md"),
      filesRead: ["auth-memory.md"],
      metadata: { source: "auth-memory.md", lifecycle: "updated" },
    });
    const updateQuery: QueryCase = {
      id: "lifecycle-update-new-marker",
      query: "Which rotated JWT key id does the auth middleware require?",
      expected_doc: "auth-memory.md",
      expected_terms: ["kid-v4", "RotatedJwtKeyPlan"],
    };
    const updateResults = memories.search(project.id, markerQuery(updateQuery), topK);
    const updateMatched = resultMatches(updateResults, updateQuery);
    const oldMarkerAbsent = resultEntriesForSource(updateResults, "auth-memory.md")
      .every((entry) => !JSON.stringify(entry).toLowerCase().includes("kid-v3"));
    checks.push(
      makeCheck(
        "update_replaces_note_text",
        updateMatched && oldMarkerAbsent ? "pass" : "lifecycle_fail",
        updateMatched && oldMarkerAbsent
          ? "claude-mem update returned the new marker and did not return the old marker for the updated memory item."
          : "claude-mem update did not cleanly replace the searchable auth memory item text.",
        {
          memory_item_id: auth.id,
          update,
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
      createdBySource.has(query.expected_doc),
  );
  if (!deleteQuery) {
    checks.push(
      makeCheck(
        "delete_suppresses_retrieval",
        "incomplete",
        "No non-update, non-recovery memory item was available, so delete suppression could not be exercised.",
        { available_sources: Array.from(createdBySource.keys()).sort() },
      ),
    );
  } else {
    const deleteId = createdBySource.get(deleteQuery.expected_doc)!.id;
    const deleteResult = db.prepare("DELETE FROM memory_items WHERE id = ?").run(deleteId);
    const deleteResults = memories.search(project.id, markerQuery(deleteQuery), topK);
    const deletedStillMatched = resultMatches(deleteResults, deleteQuery);
    checks.push(
      makeCheck(
        "delete_suppresses_retrieval",
        deletedStillMatched ? "lifecycle_fail" : "pass",
        deletedStillMatched
          ? "claude-mem SQLite delete returned success but the deleted memory item was still searchable."
          : "claude-mem SQLite delete suppressed the deleted memory item from subsequent FTS search.",
        {
          memory_item_id: deleteId,
          source: deleteQuery.expected_doc,
          query: deleteQuery,
          changes: deleteResult.changes,
          deleted_still_matched: deletedStillMatched,
          results: deleteResults,
        },
      ),
    );
  }

  const progressQuery =
    queries.find(
      (query) =>
        query.expected_doc === "database-memory.md" ||
        (query.expected_doc !== "auth-memory.md" &&
          query.expected_doc !== deleteQuery?.expected_doc),
    ) ?? queries[0];
  const progressResults = memories.search(project.id, markerQuery(progressQuery), topK);
  const progressItem = progressResults.find((entry) =>
    ((entry as { filesRead?: string[] }).filesRead ?? []).includes(
      progressQuery.expected_doc,
    ),
  );
  const detail = progressItem ? memories.getById(progressItem.id) : null;
  const sources = detail ? memories.listSources(detail.id) : [];
  const detailHasEvidence =
    !!detail &&
    !!detail.text &&
    detail.facts.length > 0 &&
    detail.concepts.length > 0 &&
    detail.filesRead.includes(progressQuery.expected_doc);
  const sourceHydrated = sources.some((source) =>
    source.sourceUri?.includes(progressQuery.expected_doc),
  );
  checks.push(
    makeCheck(
      "progressive_disclosure_detail_hydration",
      progressResults.length > 0 && detailHasEvidence && sourceHydrated
        ? "pass"
        : "lifecycle_fail",
      progressResults.length > 0 && detailHasEvidence && sourceHydrated
        ? "claude-mem search returned a bounded item that could be hydrated into detail and source evidence."
        : "claude-mem search/detail/source hydration did not expose the expected progressive-disclosure evidence.",
      {
        query: progressQuery,
        search_result_count: progressResults.length,
        detail_has_evidence: detailHasEvidence,
        source_hydrated: sourceHydrated,
        detail,
        sources,
      },
    ),
  );

  db.close();

  const reopenedDb = new Database(dbPath);
  reopenedDb.run("PRAGMA foreign_keys = ON");
  const reopenedProjects = new ProjectsRepository(reopenedDb);
  const reopenedMemories = new MemoryItemsRepository(reopenedDb);
  const reopenedProject =
    reopenedProjects.getByRootPath("/bench/corpus") ?? reopenedProjects.getById(project.id);
  const recoveryQuery: QueryCase = {
    id: "lifecycle-cold-start-recovery",
    query:
      "The invoice list N+1 query was fixed by eager loading invoice lines through `InvoiceLineBatcher`. Do not reintroduce per-row SQL calls in invoice rendering.",
    expected_doc: "database-memory.md",
    expected_terms: ["InvoiceLineBatcher", "N+1"],
  };
  const recoveryResults = reopenedProject
    ? reopenedMemories.search(reopenedProject.id, markerQuery(recoveryQuery), topK)
    : [];
  const recoveryMatched = resultMatches(recoveryResults, recoveryQuery);
  checks.push(
    makeCheck(
      "cold_start_recovery_search",
      recoveryMatched ? "pass" : "lifecycle_fail",
      recoveryMatched
        ? "A new claude-mem repository instance reopened the durable SQLite file and retrieved persisted evidence."
        : "A new claude-mem repository instance did not retrieve expected persisted evidence from the durable SQLite file.",
      {
        db_path: dbPath,
        expected_doc: recoveryQuery.expected_doc,
        matched: recoveryMatched,
        results: recoveryResults,
      },
    ),
  );
  reopenedDb.close();

  const checkSummary = summarizeChecks(checks);

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
        check_summary: checkSummary,
        checks,
        queries: queryResults,
      },
      null,
      2,
    ),
  );
} catch (err) {
  try {
    db.close();
  } catch {
    // Ignore close errors while surfacing the original benchmark failure.
  }
  throw err;
}
TS

  if run_cmd "${project}: same-corpus durable sqlite search" 300 "${log_path}" \
    "cd '${REPOS_DIR}/${project}' && bun '${driver_path}' '${result_path}' '${corpus_path}' '${REPORT_DIR}/queries.json' '${db_path}'"; then
    if jq -e '.checks and .check_summary' "${result_path}" >/dev/null 2>&1; then
      jq '{check_summary, checks}' "${result_path}" >"${REPORT_DIR}/${project}-checks.json"
    fi
    if jq -e --argjson query_count "${QUERY_COUNT}" --argjson document_count "${DOCUMENT_COUNT}" '
      .schema == "elf.live_baseline.claude_mem_result/v1" and
      .corpus.document_count == $document_count and
      .summary.total == $query_count
    ' "${result_path}" >/dev/null; then
      local typed_status
      local retrieval_status
      typed_status="$(typed_status_from_result "${result_path}")"
      if jq -e '.summary.fail == 0' "${result_path}" >/dev/null; then
        retrieval_status="retrieval_pass"
      else
        retrieval_status="retrieval_wrong_result"
      fi
      json_record "${project}" "${repo}" "${head}" "${typed_status}" "${retrieval_status}" "$(typed_status_reason "${project}" "${typed_status}")" "${project}.log" "npm install/build; MemoryItemsRepository.create/update/search; durable SQLite reopen"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "incomplete" "invalid_json_result" "claude-mem same-corpus search did not produce a valid benchmark result" "${project}.log" "npm install/build; MemoryItemsRepository.create/update/search; durable SQLite reopen"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "claude-mem built, but same-corpus SQLite search did not pass in Docker" "${project}.log" "npm install/build; MemoryItemsRepository.create/update/search; durable SQLite reopen"
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
