# Spec Index

Purpose: Provide the canonical entry point for repository specifications.

Audience: This documentation is written for LLM consumption and should remain explicit and unambiguous.

## Structure

- Store specs directly under `docs/spec/` (flat structure).
- Use descriptive file names with stable prefixes (`system_`, `t0_`, `t1_`, `trace_`, `search_`).
- Link new specs from `docs/index.md` or `docs/guide/index.md` when relevant.

## Authoring guidance (LLM-first)

- Use explicit nouns instead of pronouns whenever possible.
- Define acronyms and domain terms on first use.
- Prefer short sentences with one idea each.
- Include canonical field names, enums, units, and constraints.
- Provide small, concrete examples for non-obvious flows.
- Keep links stable and prefer absolute repo paths.
