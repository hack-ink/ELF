# Spec Index

Purpose: Provide the canonical entry point for repository specifications.

Audience: This documentation is written for LLM consumption and should remain explicit and unambiguous.

## Structure

- Store specs directly under `docs/spec/` (flat structure).
- Use descriptive file names with stable prefixes (`system_`, `t0_`, `t1_`, `trace_`, `search_`).
- Link new specs from `docs/index.md` or `docs/guide/index.md` when relevant.

## Specs

- `docs/spec/system_elf_memory_service_v2.md` - ELF Memory Service v2.0 specification.
- `docs/spec/system_source_ref_doc_pointer_v1.md` - `source_ref` doc pointer resolver for Doc Extension v1.
- `docs/spec/system_doc_source_ref_v1.md` - `doc_source_ref/v1` schema for docs ingestion provenance.
- `docs/spec/system_graph_memory_postgres_v1.md` - Graph memory schema and invariants for Postgres.
- `docs/spec/system_version_registry.md` - Registry of versioned identifiers and schema versions.
- `docs/spec/system_doc_extension_v1_filters.md` - Doc Extension v1 filter contracts and Qdrant requirements for `docs_search_l0`.
- `docs/spec/system_search_filter_expr_v1.md` - Search structured filter expression contract (`search_filter_expr/v1`) and service-side filter-impact diagnostics.

## Rollout

- `docs_search_filters/v1`:
  - `docs/spec/system_doc_extension_v1_filters.md`
  - Status: active
- `doc_source_ref/v1`:
  - `docs/spec/system_doc_source_ref_v1.md`
  - Status: active
- `search_filter_expr/v1`:
  - `docs/spec/system_search_filter_expr_v1.md`
  - Status: active

## Authoring guidance (LLM-first)

- Use explicit nouns instead of pronouns whenever possible.
- Define acronyms and domain terms on first use.
- Prefer short sentences with one idea each.
- Include canonical field names, enums, units, and constraints.
- Provide small, concrete examples for non-obvious flows.
- Keep links stable and prefer absolute repo paths.
