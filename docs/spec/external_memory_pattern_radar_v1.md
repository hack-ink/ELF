# External Memory Pattern Radar v1

Purpose: Define the durable cursor, run, and issue-decision contract for ELF's external
memory pattern radar.
Status: normative
Read this when: You are changing the weekly radar runner, cursor file, summary output,
or follow-up issue creation boundary.
Not this document: The current project comparison, benchmark results, or step-by-step
operator runbook.
Defines: `elf.external_memory_pattern_radar_cursor/v1` and
`elf.external_memory_pattern_radar_run/v1`.

## Goal

The radar keeps ELF aware of fast-moving memory, RAG, graph-memory, and
agent-continuity systems without weakening ELF's evidence-linked source-of-truth model.

The radar is a decision-support workflow. It is not an adoption workflow.

## Artifacts

Canonical checked-in paths:

- Cursor: `docs/research/external_memory_pattern_radar/cursor.json`
- Latest prose summary: `docs/research/external_memory_pattern_radar/latest.md`

Temporary dry-run outputs may be written under `tmp/external-memory-pattern-radar/`.

## Cursor Schema

`cursor.json` must use:

```json
{
  "schema": "elf.external_memory_pattern_radar_cursor/v1",
  "cadence": "weekly",
  "generated_at": "RFC3339 timestamp",
  "source_docs": ["repo-relative path or URL"],
  "projects": [],
  "last_run": null
}
```

Each `projects[]` entry must contain:

| Field | Type | Requirement |
| --- | --- | --- |
| `id` | string | Stable snake-case or kebab-safe project id. |
| `name` | string | Human-readable project name. |
| `repo` | string | GitHub `owner/name`. |
| `homepage` | string | Primary upstream URL. |
| `watch_focus` | string array | ELF benchmark or product dimensions watched for this project. |
| `primary_references` | string array | Repo-relative docs or source URLs used as current ELF context. |
| `coverage_evidence` | evidence array | Existing ELF evidence for duplicate/coverage checks. |
| `last_seen` | object or null | Last observed GitHub metadata. |

`coverage_evidence[]` entries must contain `label`, `path`, and `summary`.

## Run Schema

`last_run` must use:

```json
{
  "schema": "elf.external_memory_pattern_radar_run/v1",
  "run_id": "string",
  "generated_at": "RFC3339 timestamp",
  "mode": "live|offline",
  "summary": {},
  "decisions": []
}
```

Every run must include one decision per project.

## Decision Contract

Every `decisions[]` entry must record:

| Field | Requirement |
| --- | --- |
| `project_id` | Must match a cursor project id. |
| `upstream_change` | What changed upstream, or why no upstream fetch/change occurred. |
| `reusable_pattern` | Candidate reusable pattern, or why no pattern is claimed. |
| `elf_verdict` | One of `covered`, `reject`, or `gap`. |
| `product_value` | Product value or explicit no-value statement. |
| `duplicate_coverage_evidence` | Existing ELF docs, issues, benchmark records, or code pointers. |
| `safety_boundary` | Boundary preventing unsafe adoption, overclaiming, or hidden runtime changes. |
| `issue_decision` | No-issue, defer, or create-issue decision with rationale. |
| `acceptance_evidence` | Evidence that the radar decision itself met this contract. |
| `source_links` | Upstream links used by the decision. |

Metadata-only upstream movement must not produce `elf_verdict = "gap"`. Metadata-only
movement may only produce `covered` or `reject`, because stars, push timestamps, and
release tags are review triggers rather than architecture evidence.

## Issue Creation Boundary

`issue_decision.action = "create_issue"` is valid only when all of the following are
present in the same decision record:

- `elf_verdict = "gap"`
- upstream source links
- repo evidence showing the ELF gap or missing coverage
- explicit non-goals
- validation criteria
- Linear duplicate-search evidence with `duplicate_search.queried = true`

If any item is missing, the decision must be `no_issue` or `defer`.

## Scheduled Workflow Boundary

GitHub Actions may refresh metadata and upload read-only artifacts. GitHub Actions must
not make AI source-review judgments, create Linear issues, or claim adoption value from
activity alone.

Codex or Decodex automation may promote a radar observation into a follow-up issue only
after source review and duplicate search satisfy this spec.
