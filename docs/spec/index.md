# Spec Index

Purpose: Route agents to normative documents that define repository truth.
Status: normative
Read this when: You need to find the authoritative contract before changing code or data.
Not this document: Step-by-step execution guidance or saved planning artifacts.
Defines: Routing rules for normative documents under `docs/spec/`.

Question this index answers: "what must remain true?"

## Use this index when

- You need an invariant, contract, schema, enum, state model, interface, or required
  behavior.
- You are deciding whether code or data is correct.
- A runbook says "see the governing spec" and you need the authoritative source.

## Do not use this index when

- You need step-by-step instructions, maintenance actions, migrations, or incident
  response.
- You need a planning-tool artifact or a saved execution plan under `docs/reference/plans/`.
- You want rationale only, without an authoritative contract.

## What belongs in `docs/spec/`

- Contracts and invariants.
- Data shapes, canonical field names, enums, defaults, units, and limits.
- State transitions and protocol rules.
- Behavior that tests, code, or operators should treat as authoritative.

## Documents

- `agent_memory_quantitative_benchmark_v1.md`: Agent Memory Quantitative Benchmark v1.
- `agent_memory_knowledge_system_v1.md`: Agent Memory and Knowledge System v1.
- `external_memory_pattern_radar_v1.md`: External Memory Pattern Radar v1.
- `production_corpus_manifest_v1.md`: Production Corpus Manifest v1.
- `real_world_agent_memory_benchmark_v1.md`: Real-World Agent Memory Benchmark v1.
- `system_competitive_parity_gate_v1.md`: Competitive Parity Gate v1 Specification.
- `system_consolidation_proposals_v1.md`: Consolidation Proposals v1 Specification.
- `system_doc_chunking_profiles_v1.md`: System: `doc_chunking_profiles/v1` for `docs_put`.
- `system_doc_extension_v1_filters.md`: System: Document Extension v1 Filter and Payload Contract.
- `system_doc_extension_v1_trajectory.md`: System: Doc Extension v1 Retrieval Trajectory (`doc_retrieval_trajectory/v1`).
- `system_doc_source_ref_v1.md`: System: `doc_source_ref/v1` for `docs_put`.
- `system_elf_memory_service_v2.md`: ELF Memory Service v2.0 Specification.
- `system_graph_memory_postgres_v1.md`: Graph Memory Postgres v1.0 Specification.
- `system_knowledge_pages_v1.md`: Derived Knowledge Pages v1 Specification.
- `system_memory_summary_v1.md`: Reviewable Memory Summary v1 Specification.
- `system_provenance_mapping_v1.md`: System: Note Provenance Mapping (v1).
- `system_recall_debug_panel_v1.md`: Recall Debug Panel v1 Specification.
- `system_search_filter_expr_v1.md`: System: Search Filter Expression Contract v1.
- `system_source_ref_doc_pointer_v1.md`: System: `source_ref` Doc Pointer Resolver (v1).
- `system_version_registry.md`: System Version Registry.
- `system_work_journal_v1.md`: Work Journal v1 Specification.
## Spec document contract

Start each spec with a compact routing header:

- `Purpose`
- `Status: normative`
- `Read this when`
- `Not this document`
- `Defines`

Then keep the body explicit:

- Prefer concrete nouns over pronouns.
- Separate facts from rationale.
- Include canonical names exactly as code or data uses them.
- Include a small example when it removes ambiguity.
- Link to related runbooks instead of embedding procedures.

## Structure policy

- Prefer shallow paths while the spec set is small.
- Add subfolders only when they mirror stable system boundaries or materially reduce
  ambiguity.
- Do not require fixed filename prefixes up front.
- Choose names for topic clarity and retrieval quality, not visual uniformity.
- If a runbook depends on a spec, the runbook links back to the governing spec.
