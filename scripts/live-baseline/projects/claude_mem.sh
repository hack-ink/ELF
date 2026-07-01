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
