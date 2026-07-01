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
    "staged_retrieval_trajectory": {
      "status": "blocked",
      "surface": "no staged retrieval trajectory check is scored until same-corpus retrieval matches expected evidence ids"
    },
    "hierarchy_selection": {
      "status": "blocked",
      "surface": "no hierarchy selection check is scored until same-corpus retrieval matches expected evidence ids"
    },
    "recursive_context_expansion": {
      "status": "blocked",
      "surface": "no recursive/context expansion check is scored until same-corpus retrieval matches expected evidence ids"
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


def expected_evidence_ids(query):
    ids = query.get("expected_evidence_ids") or []
    if ids:
        return ids
    expected_doc = query["expected_doc"]
    return [expected_doc[:-3] if expected_doc.endswith(".md") else expected_doc]


def allowed_evidence_ids(query):
    return query.get("allowed_alternate_evidence_ids") or []


def result_raw(found):
    return json.dumps(to_jsonable(found), ensure_ascii=False, default=str).lower()


def visible_evidence_ids(found, query):
    raw = result_raw(found)
    candidate_ids = [*expected_evidence_ids(query), *allowed_evidence_ids(query)]
    visible = []
    for evidence_id in candidate_ids:
        lowered = evidence_id.lower()
        if lowered in raw or f"{lowered}.md" in raw:
            visible.append(evidence_id)
    return visible


def result_matches(found, query):
    raw = result_raw(found)
    expected_docs = [
        query["expected_doc"],
        *query.get("allowed_alternate_docs", []),
    ]
    has_doc = any(expected_doc.lower() in raw for expected_doc in expected_docs)
    has_terms = all(term.lower() in raw for term in query["expected_terms"])
    return has_doc and has_terms


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
        matched_evidence_ids = visible_evidence_ids(found, query)
        required_evidence_ids = expected_evidence_ids(query)
        query_results.append(
            {
                "id": query["id"],
                "query": query["query"],
                "expected_doc": query["expected_doc"],
                "expected_terms": query["expected_terms"],
                "expected_evidence_ids": required_evidence_ids,
                "allowed_alternate_evidence_ids": allowed_evidence_ids(query),
                "matched_evidence_ids": matched_evidence_ids,
                "missing_evidence_ids": [
                    evidence_id
                    for evidence_id in required_evidence_ids
                    if evidence_id not in matched_evidence_ids
                ],
                "matched": result_matches(found, query),
                "find": to_jsonable(found),
            }
        )
    pass_count = sum(1 for result in query_results if result["matched"])
    evidence_total = sum(len(result["expected_evidence_ids"]) for result in query_results)
    evidence_matched = sum(
        len(
            [
                evidence_id
                for evidence_id in result["matched_evidence_ids"]
                if evidence_id in result["expected_evidence_ids"]
            ]
        )
        for result in query_results
    )
    same_corpus_output_correct = (
        pass_count == len(query_results)
        and evidence_total > 0
        and evidence_matched == evidence_total
    )
    trajectory_gate_status = "not_encoded" if same_corpus_output_correct else "blocked"
    trajectory_gate_reason = (
        "OpenViking same-corpus retrieval matched expected evidence ids, but staged trajectory scoring is not encoded in this Docker adapter."
        if trajectory_gate_status == "not_encoded"
        else "OpenViking staged trajectory scoring is blocked until same-corpus retrieval matches expected evidence ids."
    )
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
            "name": "same_corpus_expected_evidence_ids_visible",
            "status": "pass"
            if all(result["expected_evidence_ids"] for result in query_results)
            else "incomplete",
            "reason": "OpenViking query results expose expected, matched, and missing evidence ids for every same-corpus query.",
            "evidence": {
                "total_queries": len(query_results),
                "queries_with_expected_evidence_ids": sum(
                    1 for result in query_results if result["expected_evidence_ids"]
                ),
                "expected_evidence_total": evidence_total,
                "expected_evidence_matched": evidence_matched,
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
        {
            "name": "staged_retrieval_trajectory",
            "status": trajectory_gate_status,
            "reason": trajectory_gate_reason,
            "evidence": {
                "blocked_by": "same_corpus_expected_evidence_miss"
                if trajectory_gate_status == "blocked"
                else None
            },
        },
        {
            "name": "hierarchy_selection",
            "status": trajectory_gate_status,
            "reason": trajectory_gate_reason.replace(
                "staged trajectory", "hierarchy selection"
            ),
            "evidence": {
                "blocked_by": "same_corpus_expected_evidence_miss"
                if trajectory_gate_status == "blocked"
                else None
            },
        },
        {
            "name": "recursive_context_expansion",
            "status": trajectory_gate_status,
            "reason": trajectory_gate_reason.replace(
                "staged trajectory", "recursive/context expansion"
            ),
            "evidence": {
                "blocked_by": "same_corpus_expected_evidence_miss"
                if trajectory_gate_status == "blocked"
                else None
            },
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
