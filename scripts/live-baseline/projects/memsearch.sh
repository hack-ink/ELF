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
