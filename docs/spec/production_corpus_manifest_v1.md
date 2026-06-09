# Production Corpus Manifest v1

Purpose: Define the sanitized/private coding-agent production corpus manifest used by
ELF adoption benchmarks.
Status: normative
Read this when: You are creating, validating, or running a production-style personal
agent memory benchmark corpus.
Not this document: Docker benchmark run commands, report publication steps, or private
fixture storage procedures.
Defines: `elf.production_corpus_manifest/v1` fields, required evidence categories,
query tasks, evidence expectations, and private-content safety rules.

## Contract

A production corpus manifest is a JSON object with:

- `schema`: exactly `elf.production_corpus_manifest/v1`.
- `manifest_id`: stable lower-risk identifier for the corpus snapshot. Allowed
  shape: `[a-z0-9][a-z0-9_.-]{1,80}`.
- `description`: optional English summary.
- `evidence`: non-empty array of production-style memory evidence items.
- `queries`: non-empty array of task-oriented retrieval checks.

The checked-in benchmark fixture must be synthetic and sanitized. Real private
production content must not be committed.

## Evidence Items

Each `evidence[]` item must include:

- `evidence_id`: lower-case ASCII identifier safe for filenames. Allowed shape:
  `[a-z0-9][a-z0-9_.-]{1,80}`.
- `category`: one of `issue`, `pr`, `worktree`, `runbook`, `decision`, `blocker`,
  or `recovery_note`.
- `title`: short English title.
- Exactly one of:
  - `text`: sanitized inline English evidence text.
  - `local_path`: path to a local sanitized text/Markdown file, resolved relative to
    the manifest when not absolute.

Evidence text must not contain secrets, tokens, private keys, personal credentials, or
unsanitized private conversation content.

## Query Cases

Each `queries[]` item must include:

- `query_id`: stable query identifier. Allowed shape:
  `[a-z0-9][a-z0-9_.-]{1,80}`.
- `task`: one of `resume_lane`, `recover_exact_command`, `explain_stale_blocker`,
  `find_prior_decision`, `compare_project_status`, or
  `detect_contradiction_update`.
- `query`: English task-oriented search query.
- `expected_evidence_ids`: non-empty array of evidence IDs that satisfy the query.
- `allowed_alternate_evidence_ids`: array of acceptable alternate evidence IDs. Use
  an empty array when no alternate is allowed.
- `expected_terms`: non-empty array of terms that should appear in the matched
  evidence snippet when the expected note key is not the top result.

Every query must record both expected evidence IDs and allowed alternates, even when
the allowed alternate list is empty.

## Benchmark Mapping

The Docker benchmark materializes each evidence item as a temporary Markdown document
inside the benchmark work directory. The source document filename is
`<evidence_id>.md`. Reports must expose evidence IDs and allowed alternates, not local
private file paths.

For `production-private` runs, the runner must fail closed when the manifest is absent,
the manifest references a missing `local_path`, or any query references an unknown
evidence ID. It must not silently fall back to the checked-in synthetic corpus.

## Minimal Example

```json
{
  "schema": "elf.production_corpus_manifest/v1",
  "manifest_id": "local-private-prod-corpus-2026-06-09",
  "evidence": [
    {
      "evidence_id": "issue-xy123-resume",
      "category": "issue",
      "title": "XY-123 Resume State",
      "text": "XY-123 resumes on branch y/example with command `cargo make checks`."
    }
  ],
  "queries": [
    {
      "query_id": "q-resume-xy123",
      "task": "resume_lane",
      "query": "How do I resume XY-123?",
      "expected_evidence_ids": ["issue-xy123-resume"],
      "allowed_alternate_evidence_ids": [],
      "expected_terms": ["XY-123", "cargo make checks"]
    }
  ]
}
```

## Related Guides

- `docs/guide/benchmarking/live_baseline_benchmark.md`: run commands, private fixture
  placement, and report publication.
