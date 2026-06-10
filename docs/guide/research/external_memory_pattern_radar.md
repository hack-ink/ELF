# External Memory Pattern Radar

Goal: Run ELF's weekly external memory pattern radar and preserve no-issue, rejection,
or issue-ready outcomes for future comparison reports.
Read this when: You are refreshing upstream memory/RAG/agent-continuity watch state or
deciding whether a watched upstream pattern deserves an ELF follow-up issue.
Inputs: `docs/research/external_memory_pattern_radar/cursor.json`, GitHub repository
metadata, current ELF research docs, and Linear duplicate-search readback when creating
issues.
Depends on: `docs/spec/external_memory_pattern_radar_v1.md`,
`docs/guide/research/comparison_external_projects.md`, and
`docs/guide/research/research_projects_inventory.md`.
Outputs: Updated cursor JSON plus `docs/research/external_memory_pattern_radar/latest.md`.

## Scope

The radar watches agentmemory, mem0, qmd, claude-mem, OpenViking, Graphiti, Letta,
LightRAG, GraphRAG, RAGFlow, and adjacent projects already represented in ELF's
external comparison research.

The radar does not adopt external runtimes by default and does not create follow-up
issues from stars, activity, release tags, or push timestamps alone.

## Commands

Run a live cursor refresh:

```sh
cargo make external-memory-radar
```

Run the deterministic no-network dry run used by local PR checks and fallback
verification:

```sh
cargo make external-memory-radar-dry-run
```

Run a live read-only artifact refresh under `tmp/` without changing checked-in files:

```sh
cargo make external-memory-radar-artifact
```

Validate the checked-in cursor:

```sh
cargo make external-memory-radar-validate
```

## Issue Decision Rules

For every candidate pattern, the cursor decision must record:

- upstream change
- reusable pattern
- ELF verdict: `covered`, `reject`, or `gap`
- product value
- duplicate/coverage evidence
- safety boundary
- issue decision
- acceptance evidence

`create_issue` is allowed only when the decision also records upstream source links,
repo evidence, non-goals, validation criteria, and Linear duplicate-search evidence.
When the run is no-issue, the cursor still records why the pattern is already covered
or why the observed change is rejected.

## Weekly Schedule

`.github/workflows/external-memory-pattern-radar.yml` runs weekly and on manual
dispatch. The scheduled workflow refreshes live GitHub metadata and writes artifacts under
`tmp/external-memory-pattern-radar/` and uploads them for review.

The workflow is intentionally read-only with respect to Linear and repository contents.
Codex or Decodex automation may consume the artifact, perform source review, search
Linear, and then submit a small PR that updates the cursor and prose summary.

## Next Comparison Report Input

The next full comparison report should consume:

- changed project metadata from `projects[].last_seen`
- no-issue and rejection rationales from `last_run.decisions[]`
- issue-ready `gap` records only when `issue_decision.action = "create_issue"`
- source links, repo evidence, non-goals, and validation criteria from proposed issues

Do not quote a watched project as an ELF gap or parity win unless the cursor decision
contains source-backed evidence under the radar spec.
