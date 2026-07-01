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
