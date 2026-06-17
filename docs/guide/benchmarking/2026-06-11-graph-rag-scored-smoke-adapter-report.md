# Graph/RAG Scored Smoke Adapter Report - June 11, 2026

Goal: Record the XY-900 promotion of graph/RAG Docker smokes and the XY-929
representative fixture slice into scored or typed `real_world_job` adapter evidence
without upgrading smoke or typed non-pass evidence into broad quality claims.
Read this when: You need to decide whether ELF currently wins, ties, loses, or remains
untested against RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, and graphify graph/RAG
strengths.
Inputs: `memory_projects_manifest.json`, the graph/RAG smoke and representative
fixture commands in `Makefile.toml`, and the generated report contracts.
Outputs: Scored-smoke status, representative typed non-pass status, claim boundary,
blocker taxonomy, and next measurement gate for each in-scope project.

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

XY-929 adds a representative external-adapter fixture slice for graph/RAG navigation,
citations, graph summaries, temporal validity, graph reports, stale-source lint, and
unsupported-claim handling. The slice intentionally remains typed non-pass: 5 jobs,
0 pass, 3 blocked, 1 incomplete, and 1 wrong_result. It strengthens the reporting
contract, not the quality claim.

## Scored Smoke Status

| Project | Scored scenario | Command | Current scored status | Claim boundary |
| --- | --- | --- | --- | --- |
| RAGFlow | `retrieval`: reference chunks mapped to generated evidence ids | `cargo make smoke-ragflow-docker` | `blocked` or `incomplete` by execution boundary | Smoke-only. No RAGFlow quality claim until returned reference chunks map to `ragflow-smoke-anchor`. |
| LightRAG | `retrieval`: context/source export mapped to fixture evidence ids | `cargo make smoke-lightrag-docker-context` | `incomplete` when the API service is not started | Smoke-only. No graph-RAG quality claim until context or references map to generated evidence ids. |
| GraphRAG | `knowledge_compilation`: output tables mapped to generated evidence ids | `cargo make smoke-graphrag-docker` | `blocked` | Smoke-only. No graph-navigation or synthesis claim until output tables map to generated evidence ids. |
| Graphiti/Zep | `memory_evolution`: current and historical validity facts | `cargo make smoke-graphiti-zep-docker-temporal` | `blocked` before live opt-in; `provider_api_key_missing` when live path is enabled without explicit credentials | Provider-bound. No ELF-over-Graphiti/Zep claim until temporal output maps to scored evidence ids. |
| graphify | `knowledge_compilation`: `graph.json`, `GRAPH_REPORT.md`, and query output mapping | `cargo make smoke-graphify-docker-graph-report` | `wrong_result` after setup/run pass | Scored tiny smoke. The graph/report output maps to evidence ids, but the job remains non-pass; no broad graph-navigation quality claim follows. |

## Artifact Contract

Each promoted smoke now writes a generated fixture and scored report:

| Project | Generated report |
| --- | --- |
| RAGFlow | `tmp/real-world-memory/ragflow-smoke/ragflow-report.json` and `.md` |
| LightRAG | `tmp/real-world-memory/lightrag-context/lightrag-report.json` and `.md` |
| GraphRAG | `tmp/real-world-memory/graphrag-smoke/graphrag-report.json` and `.md` |
| Graphiti/Zep | `tmp/real-world-memory/graphiti-zep-smoke/graphiti-zep-report.json` and `.md` |
| graphify | `tmp/real-world-memory/graphify-smoke/graphify-report.json` and `.md` |

## Representative Fixture Slice

Run the representative graph/RAG slice separately from the heavyweight live adapter
sweep:

```sh
cargo make real-world-memory-graph-rag
```

Artifacts:

```text
tmp/real-world-memory/graph-rag/report.json
tmp/real-world-memory/graph-rag/report.md
```

Current focused report summary:

| Metric | Value |
| --- | --- |
| Jobs | 5 |
| Pass | 0 |
| Blocked | 3 |
| Incomplete | 1 |
| Wrong result | 1 |
| Temporal validity not encoded | 1 |

Representative job outcomes:

| Project | Representative contract | Job status | ELF outcome | Boundary |
| --- | --- | --- | --- | --- |
| RAGFlow | Reference chunks must map generated document ids, chunk ids, content, and document metadata to benchmark evidence ids. | `blocked` | `blocked` | Resource/API setup and returned reference chunks are still missing. |
| LightRAG | Context/source export must expose generated file paths, snippets, or reference content mapped to evidence ids. | `incomplete` | `blocked` | The opt-in Docker API export is not available by default, so comparison remains blocked. |
| GraphRAG | Output tables must map documents, text units, communities, reports, entities, and relationships to generated evidence ids. | `blocked` | `blocked` | Provider-backed Docker output tables are required before citation or synthesis scoring can pass. |
| Graphiti/Zep | Current and historical graph facts must carry validity windows and evidence ids. | `blocked` | `blocked` | Temporal validity is not encoded without provider-backed current/historical output. |
| graphify | `graph.json`, source-location report sections, unsupported-claim lint, and stale-source lint are scored. | `wrong_result` | `not_tested` | The representative job reaches scoring but misses stale-source/answer requirements; no ELF victory or graphify quality conclusion follows. |
| llm-wiki | Citation-bearing wiki/page generation with stale-source and unsupported-claim lint. | `not_encoded` | `not_tested` | No contained output contract exists yet. |
| gbrain | Compiled-truth or timeline export with evidence-linked page sections. | `blocked` | `blocked` | Docker-local setup and export readback remain missing. |
| Private, hosted, or large-corpus graph/RAG profiles | Provider, private data, or hosted service behavior. | `not_encoded` | `non_goal` | These profiles are outside the generated public representative lane unless explicitly authorized. |

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
- Say the XY-929 representative slice produces typed non-pass reports for RAGFlow,
  LightRAG, GraphRAG, Graphiti/Zep, graphify, llm-wiki, and gbrain claim boundaries.
- Say graph/RAG quality remains untested where live output has not mapped to generated
  evidence ids or where scored output remains typed non-pass.
- Say graphify reached a tiny Docker graph/report smoke and currently scores
  `wrong_result`.
- Say Graphiti/Zep remains blocked by default live-run opt-in, and provider-blocked
  when that live path is explicitly enabled without credentials; it remains the
  temporal-validity reference.

Not allowed:

- Do not call a smoke pass a broad RAG, graph, temporal, or production-quality pass.
- Do not call a representative blocked, incomplete, wrong_result, or not_encoded job a
  broad RAG, graph, temporal, or production-quality result.
- Do not claim ELF beats Graphiti/Zep, RAGFlow, LightRAG, GraphRAG, or graphify on
  their graph/RAG strengths from these smoke or representative non-pass reports.
- Do not use hosted/cloud-only results, host-global installs, private corpora, or
  unrecorded credentials as evidence for this lane.
