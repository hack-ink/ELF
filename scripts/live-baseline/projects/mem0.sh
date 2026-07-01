project_mem0() {
  local project="mem0"
  local repo="https://github.com/mem0ai/mem0.git"
  local log_path="${REPORT_DIR}/${project}.log"
  local result_path="${REPORT_DIR}/${project}-search.json"
  local openmemory_probe_path="${REPORT_DIR}/${project}-openmemory-ui-export.json"
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
      "surface": "bounded export-helper setup probe writes tmp/live-baseline/mem0-openmemory-ui-export.json; SDK get_all remains separate"
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


def history_entries(history):
    if isinstance(history, dict):
        for key in ("results", "history", "memories"):
            entries = history.get(key)
            if isinstance(entries, list):
                return entries
    if isinstance(history, list):
        return history
    return []


def history_has_event(history, expected_event):
    expected = expected_event.upper()
    return any(
        isinstance(entry, dict) and str(entry.get("event", "")).upper() == expected
        for entry in history_entries(history)
    )


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
    history_has_add_event = preference_history["available"] and history_has_event(
        preference_history["history"],
        "ADD",
    )
    history_has_update_event = preference_history["available"] and history_has_event(
        preference_history["history"],
        "UPDATE",
    )
    search_has_current = contains_terms(
        result_entries(preference_search),
        ["concise", "evidence-linked"],
    )
    search_omits_old = "verbose tutorial" not in json_lower(result_entries(preference_search))
    if not preference_history["available"]:
        preference_status = "blocked"
        preference_reason = "Memory.history could not be read for the updated preference memory."
    elif (
        history_has_old
        and history_has_current
        and history_has_add_event
        and history_has_update_event
        and search_has_current
        and search_omits_old
    ):
        preference_status = "pass"
        preference_reason = "mem0 history preserved ADD and UPDATE preference events while search returned only the current correction."
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
                "history_has_add_event": history_has_add_event,
                "history_has_update_event": history_has_update_event,
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
    delete_history_has_event = delete_history["available"] and history_has_event(
        delete_history["history"],
        "DELETE",
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
    probe_mem0_openmemory_ui_export "${REPOS_DIR}/${project}" "${result_path}" "${openmemory_probe_path}" "${log_path}"
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
      json_record "${project}" "${repo}" "${head}" "${typed_status}" "${retrieval_status}" "$(typed_status_reason "${project}" "${typed_status}")" "${project}.log" "pip install -e . fastembed ollama; Memory.from_config; add/update/delete/history/get_all/search; OpenMemory export probe"
      return
    fi
    json_record "${project}" "${repo}" "${head}" "incomplete" "invalid_json_result" "mem0 command completed, but did not produce a valid benchmark result" "${project}.log" "pip install -e . fastembed ollama; Memory.from_config; add infer=false; search"
    return
  fi

  json_record "${project}" "${repo}" "${head}" "incomplete" "retrieval_command_failed" "mem0 installed and imported, but local fastembed/Qdrant add/search failed" "${project}.log" "pip install -e . fastembed ollama; Memory.from_config; add infer=false; search"
}
