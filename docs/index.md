# Documentation Index

Purpose: Route agents to the smallest correct document set for the current task.
Read this when: You are starting from repository docs and need to choose the right lane.
Not this document: Detailed subsystem contracts, step-by-step runbooks, research run state, or saved plan artifacts.
Routes to: `docs/policy.md`, `docs/spec/`, `docs/runbook/`, `docs/reference/`,
`docs/decisions/`, `docs/research/`, `docs/evidence/`, and `Makefile.toml`.

Audience: All documentation in this repository is written for AI agents and LLM workflows.
The split below is by question type, not by human-versus-agent audience.

## Read order

- Read `docs/policy.md` for document contracts and placement rules.
- Read `Makefile.toml` when the task depends on repo task names or execution entrypoints.
- Then choose one primary lane:
  - `docs/spec/index.md` when the question is "what must be true?"
  - `docs/runbook/index.md` when the question is "what should I do?"
- Use `docs/reference/` for current non-procedural orientation and retained
  historical plan artifacts.
- Use `docs/decisions/` for accepted rationale.
- Use `docs/research/` for active OKF research contracts. Machine-readable artifacts
  stay outside `docs/` and are cited only when an active owner still needs them.
- Use `docs/evidence/` for proof records, benchmark reports, external comparison
  evidence, drift audits, and promoted research evidence.

## Routing matrix

- Need contracts, invariants, schemas, enums, state machines, or required behavior ->
  `docs/spec/`
- Need the Agent Memory + Knowledge System product boundary, P0-P5 roadmap,
  Decodex phase gate, or competitor absorption rules ->
  `docs/spec/agent_memory_knowledge_system_v1.md`
- Need runbooks, migrations, validation steps, troubleshooting, or operational sequences ->
  `docs/runbook/`
- Need the single-user production backup, restore, and Qdrant rebuild path ->
  `docs/runbook/single_user_production.md`
- Need benchmark commands or interpretation steps -> `docs/runbook/benchmarking/`
- Need checked-in benchmark reports -> `docs/evidence/benchmarking/`
- Need external comparisons or architecture research inputs ->
  `docs/evidence/external_memory/`
- Need external-memory radar commands -> `docs/runbook/external_memory_pattern_radar.md`
- Need research provenance, evidence, trade-offs, or decision status ->
  `docs/research/`, `docs/decisions/`, and `docs/evidence/` depending on whether the
  point is latent, accepted, or audit evidence.
- Need repo task names or automation entrypoints -> `Makefile.toml`
- Need documentation placement or authoring rules -> `docs/policy.md`
- Need a retained planning-tool artifact or saved execution plan ->
  `docs/reference/plans/`

## Retrieval rules

- Optimize for agent routing and execution, not narrative flow.
- Keep one authoritative document per topic. Link instead of copying.
- Start each document with a short routing header that says what the document is for,
  when to read it, and what it does not cover.
- Keep links explicit and stable.
- Let structure emerge from real topics. Do not create empty folders, empty indexes, or
  naming schemes that are stricter than the current corpus needs.
