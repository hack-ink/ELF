# Documentation Governance

Purpose: Define how documentation is organized, updated, and kept consistent across this
repository.

## Principles

- Write documentation that is clear, concise, retrieval-friendly, and LLM-first.
- Keep contracts and invariants in `docs/spec/`; keep runbooks and how-to guidance in
  `docs/guide/`.
- Avoid duplicating authoritative content. Link to the source of truth instead.

## Document classes and ownership

| Class | Location | Source of truth for | Update trigger |
| --- | --- | --- | --- |
| Spec | `docs/spec/` | Contracts, schemas, pipeline behavior, invariants | Any behavior or schema change |
| Operational docs | `docs/guide/` | Runbooks, pipeline walkthroughs, maintenance | When operating procedures change |
| Plans | `docs/plans/` | Draft plans and design notes (non-normative) | As-needed, may drift |

## Placement rules

- If it defines a contract, it belongs in `docs/spec/`.
- If it explains how to run or maintain a system, it belongs in `docs/guide/`.
- If it is temporary or exploratory, it belongs in `docs/plans/`.
- Module documentation must live under `docs/guide/` and be linked from `docs/guide/index.md`.
  Do not add module-level README files.
- Do not duplicate the same content in both spec and guide files. Spec defines what must be true;
  guide explains how to operate or implement it. When in doubt, link to the source of truth.

## Canonical entry points

- Repository overview: `README.md` (the only README in the repository).
- Specs: `docs/spec/index.md`.
- Operational docs: `docs/guide/index.md`.
- Unified documentation index: `docs/index.md`.

## Compatibility note

Legacy paths are no longer maintained. Use `docs/` paths for all references.

## LLM reading guidance

When answering questions about system behavior:

1. Read `AGENTS.md` for tool and scope rules.
2. Use `docs/spec/index.md` for contracts and invariants.
3. Use `docs/guide/index.md` for runbooks and operational workflows.

## Update workflow

- Behavior or schema change: update the relevant `docs/spec/` doc.
- Procedure change: update the relevant `docs/guide/` guide.
- Avoid copying long sections between documents. Link instead.

## Naming conventions

- Spec files use descriptive `snake_case` names with stable prefixes (`system_`, `t0_`, `t1_`,
  `trace_`, `search_`).
- Guide files use descriptive `snake_case` names within their category folders
  (`development/`, `operations/`, `pipelines/`, `testing/`).
- Plan files use `YYYY-MM-DD_<topic>_<type>.md` with `snake_case` topics (for example,
  `2026-01-01_cryptopotato_crawler_plan.md`).
