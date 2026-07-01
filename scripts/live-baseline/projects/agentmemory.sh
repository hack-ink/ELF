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
