# Graph/RAG Scored Smoke Adapter Report - June 11, 2026

Goal: Record the XY-900 promotion of graph/RAG Docker smokes into scored
`real_world_job` adapter evidence without upgrading smoke evidence into broad quality
claims.
Read this when: You need to decide whether ELF currently wins, ties, loses, or remains
untested against RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, and graphify graph/RAG
strengths.
Inputs: `memory_projects_manifest.json`, the graph/RAG smoke commands in
`Makefile.toml`, and the generated smoke report contracts.
Outputs: Scored-smoke status, claim boundary, blocker taxonomy, and next measurement
gate for each in-scope project.

## Verdict

XY-900 promotes the in-scope Docker smokes into scored adapter evidence where the smoke
already has enough generated evidence ids to evaluate a bounded job. This is still
smoke-only evidence.

Current graph/RAG quality comparison remains mostly untested. ELF cannot claim a win,
tie, or loss against the in-scope graph/RAG strengths from smoke evidence alone.
`graphify` is the current exception only in the narrow sense that its Docker smoke
reaches graph/report output and scores one tiny `knowledge_compilation` job as
`wrong_result`; that is a bounded graphify non-pass, not an ELF victory claim.

Graphiti/Zep remains the temporal-validity reference. The default checked-in smoke is
typed `blocked` before live execution because `ELF_GRAPHITI_ZEP_SMOKE_START=1` and
`ELF_GRAPHITI_ZEP_SMOKE_RUN=1` are not set. When that live path is explicitly enabled
without provider credentials, the blocker remains `provider_api_key_missing`; no
hosted Zep service or unrecorded provider credentials are used or implied.

## Scored Smoke Status

| Project | Scored scenario | Command | Current scored status | Claim boundary |
| --- | --- | --- | --- | --- |
| RAGFlow | `retrieval`: reference chunks mapped to generated evidence ids | `cargo make ragflow-docker-smoke` | `blocked` or `incomplete` by execution boundary | Smoke-only. No RAGFlow quality claim until returned reference chunks map to `ragflow-smoke-anchor`. |
| LightRAG | `retrieval`: context/source export mapped to fixture evidence ids | `cargo make lightrag-docker-context-smoke` | `incomplete` when the API service is not started | Smoke-only. No graph-RAG quality claim until context or references map to generated evidence ids. |
| GraphRAG | `knowledge_compilation`: output tables mapped to generated evidence ids | `cargo make graphrag-docker-smoke` | `blocked` | Smoke-only. No graph-navigation or synthesis claim until output tables map to generated evidence ids. |
| Graphiti/Zep | `memory_evolution`: current and historical validity facts | `cargo make graphiti-zep-docker-temporal-smoke` | `blocked` before live opt-in; `provider_api_key_missing` when live path is enabled without explicit credentials | Provider-bound. No ELF-over-Graphiti/Zep claim until temporal output maps to scored evidence ids. |
| graphify | `knowledge_compilation`: `graph.json`, `GRAPH_REPORT.md`, and query output mapping | `cargo make graphify-docker-graph-report-smoke` | `wrong_result` after setup/run pass | Scored tiny smoke. The graph/report output maps to evidence ids, but the job remains non-pass; no broad graph-navigation quality claim follows. |

## Artifact Contract

Each promoted smoke now writes a generated fixture and scored report:

| Project | Generated report |
| --- | --- |
| RAGFlow | `tmp/real-world-memory/ragflow-smoke/ragflow-report.json` and `.md` |
| LightRAG | `tmp/real-world-memory/lightrag-context/lightrag-report.json` and `.md` |
| GraphRAG | `tmp/real-world-memory/graphrag-smoke/graphrag-report.json` and `.md` |
| Graphiti/Zep | `tmp/real-world-memory/graphiti-zep-smoke/graphiti-zep-report.json` and `.md` |
| graphify | `tmp/real-world-memory/graphify-smoke/graphify-report.json` and `.md` |

The aggregate live-adapter sweep can include these reports through explicit opt-in
flags. These flags include an adapter in the aggregate report; provider-backed,
service-started, or resource-heavy live attempts still require the adapter-specific
controls listed by each smoke task:

- `ELF_REAL_WORLD_LIVE_ENABLE_RAGFLOW=1`
- `ELF_REAL_WORLD_LIVE_ENABLE_LIGHTRAG=1`
- `ELF_REAL_WORLD_LIVE_ENABLE_GRAPHRAG=1`
- `ELF_REAL_WORLD_LIVE_ENABLE_GRAPHITI_ZEP=1`
- `ELF_REAL_WORLD_LIVE_ENABLE_GRAPHIFY=1`

Default `cargo make real-world-memory-live-adapters` still runs ELF and qmd only. That
keeps heavyweight services, provider-backed runs, and graph/report installs out of the
default sweep unless explicitly requested.

## Typed Limits

Resource, runtime, provider, and setup limits remain first-class report states:

- `blocked`: live execution requires explicit resource opt-in, provider credentials,
  a Docker service profile, or a generated output that is not yet available.
- `incomplete`: setup or service reachability failed before the behavioral check.
- `wrong_result`: the smoke reached scoring but failed required answer or rubric
  signals, including unmapped evidence where applicable.
- `pass`: the smoke reached output and all required generated evidence ids mapped.
- `not_encoded`: broad quality, scale, private corpus, hosted-service behavior, and
  non-smoke suites remain outside the current adapter.

## Claim Rules

Allowed:

- Say the in-scope graph/RAG smokes now produce scored `real_world_job` adapter reports
  or typed non-pass reports.
- Say graph/RAG quality remains untested where live output has not mapped to generated
  evidence ids or where scored output remains typed non-pass.
- Say graphify reached a tiny Docker graph/report smoke and currently scores
  `wrong_result`.
- Say Graphiti/Zep remains blocked by default live-run opt-in, and provider-blocked
  when that live path is explicitly enabled without credentials; it remains the
  temporal-validity reference.

Not allowed:

- Do not call a smoke pass a broad RAG, graph, temporal, or production-quality pass.
- Do not claim ELF beats Graphiti/Zep, RAGFlow, LightRAG, GraphRAG, or graphify on
  their graph/RAG strengths from these smoke reports.
- Do not use hosted/cloud-only results, host-global installs, private corpora, or
  unrecorded credentials as evidence for this lane.
