# Documentation Governance

Purpose: Define how agent-facing documentation is organized, updated, and kept consistent
across this repository.
Status: normative
Read this when: You are creating, moving, splitting, or revising repository documentation.
Not this document: System behavior contracts or operational runbooks for one subsystem.
Defines: Document classes, placement rules, routing headers, and docs update workflow.

Audience: All documentation under `docs/` is written for AI agents and LLM workflows.
The split between `spec` and `guide` is by task shape, not by reader type.

## Principles

- Optimize for retrieval, routing, and execution.
- Keep one authoritative document per topic.
- Separate normative truth from procedural steps.
- Prefer explicit section labels and stable links over prose-heavy narrative.
- Let structure emerge from real topics. Avoid premature folder taxonomies.

## Document classes

| Class | Location | Answers | Source of truth for | Update trigger |
| --- | --- | --- | --- | --- |
| Spec | `docs/spec/` | What must be true? | Contracts, schemas, invariants, required behavior | Any behavior or schema change |
| Guide | `docs/guide/` | What should I do? | Runbooks, migrations, validation, troubleshooting | Any procedure or operational change |
| Plan artifacts | `docs/plans/` | Which saved plan artifact should a planning tool or execution workflow use? | Tool-managed planning outputs | As emitted or updated by the relevant tool |

## Placement rules

- If a document defines correctness, it belongs in `docs/spec/`.
- If a document defines actions, it belongs in `docs/guide/`.
- If a document is non-normative decision support, comparison, or research input, treat it
  as guide-class material and store it under `docs/guide/`.
- Do not treat `docs/plans/` as a general-purpose docs bucket.
- Use `docs/plans/` only for artifacts produced or consumed by planning tools or
  workflows that explicitly depend on saved plan files.
- Do not duplicate the same authoritative content across documents. Link to the source
  of truth instead.
- A guide may summarize why a step exists, but normative statements still live in the
  governing spec.

## Document contracts

Every document should start with a short routing header.

Spec header:

- `Purpose`
- `Status: normative`
- `Read this when`
- `Not this document`
- `Defines`

Guide header:

- `Goal`
- `Read this when`
- `Inputs` or `Preconditions`
- `Depends on`
- `Outputs` or `Verification`

## Structure rules

- Prefer shallow paths by default.
- Add subfolders only when they mirror stable system boundaries or improve retrieval.
- Use descriptive `snake_case` file names.
- Do not require fixed filename prefixes unless a real ambiguity appears.
- Do not create empty folders, empty indexes, or placeholder documents to satisfy a
  taxonomy.

## Canonical entry points

- Unified documentation router: `docs/index.md`
- Normative router: `docs/spec/index.md`
- Procedural router: `docs/guide/index.md`
- Repo task and automation entrypoints: `Makefile.toml`

## LLM reading guidance

When answering a repository question:

1. Read `docs/index.md` for routing.
2. Route by question type:
   - "What must be true?" -> `docs/spec/index.md`
   - "What should I do?" -> `docs/guide/index.md`
3. Read `Makefile.toml` when the task depends on repository automation or named tasks.
4. Use `docs/plans/` only when the task explicitly concerns a saved plan artifact used by
   a planning tool or execution workflow.

## Update workflow

- Behavior or schema change: update the relevant spec.
- Procedure change: update the relevant guide.
- If a change touches both truth and procedure, update both documents and keep their
  boundary explicit.
- When a guide starts carrying normative content, move that content into spec and link
  to it.
- Do not impose local document-header requirements on files under `docs/plans/`; those
  files are owned by the planning tool or workflow that created them.
