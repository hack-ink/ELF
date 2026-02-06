<div align="center">

# ELF

Evidence-linked fact memory for agents.

[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Language Checks](https://github.com/hack-ink/ELF/actions/workflows/language.yml/badge.svg?branch=main)](https://github.com/hack-ink/ELF/actions/workflows/language.yml)
[![Release](https://github.com/hack-ink/ELF/actions/workflows/release.yml/badge.svg)](https://github.com/hack-ink/ELF/actions/workflows/release.yml)
[![GitHub tag (latest by date)](https://img.shields.io/github/v/tag/hack-ink/ELF)](https://github.com/hack-ink/ELF/tags)
[![GitHub last commit](https://img.shields.io/github/last-commit/hack-ink/ELF?color=red&style=plastic)](https://github.com/hack-ink/ELF)
[![GitHub code lines](https://tokei.rs/b1/github/hack-ink/ELF)](https://github.com/hack-ink/ELF)

</div>

## What Is ELF?

ELF is a memory service that stores short, evidence-linked facts for agents. It separates deterministic writes from LLM extraction, enforces evidence binding, and provides chunk-first hybrid retrieval with configurable quality and cost controls. Postgres with pgvector is the source of truth for notes and chunk embeddings; Qdrant is a derived, rebuildable chunk index for fast candidate retrieval. ELF exposes HTTP and MCP interfaces for agent integrations, including a progressive search workflow (index view first, details on demand).

## Why ELF

- Evidence-linked memory. Every extracted note includes verbatim evidence quotes.
- Deterministic ingestion. `add_note` never calls an LLM; `add_event` always does.
- Source-of-truth storage. Postgres is authoritative; Qdrant can be rebuilt at any time.
- Chunk-first hybrid retrieval. Dense + BM25 candidate retrieval over token-aware chunks with optional reranking.
- Query expansion modes. `off`, `always`, or `dynamic` to balance recall and latency.
- Progressive disclosure search. `/search` returns a compact index; `/search/details` fetches full notes and can record hits.
- Cost and debugging controls. Expansion and rerank caching plus search traces and explain endpoints.
- Multi-tenant scoping. Tenant, project, agent, and scope boundaries are enforced.
- MCP integration. A dedicated `elf-mcp` server for Claude and other MCP clients.
- Evaluation-ready. `elf-eval` lets you measure retrieval quality quickly.

## Architecture

```mermaid
flowchart TB
  subgraph Clients
    Agent[Agent / App]
    MCPClient[MCP Client]
    Eval[elf-eval]
  end

  subgraph Services
    API[elf-api]
    MCP[elf-mcp]
    Worker[elf-worker]
  end

  subgraph Storage
    PG[(Postgres + pgvector\nsource of truth)]
    Qdrant[(Qdrant\nrebuildable index)]
  end

  subgraph Providers
    Embed[Embedding Provider]
    Rerank[Reranker]
    Extractor[LLM Extractor]
  end

  Agent -->|HTTP| API
  MCPClient -->|MCP| MCP
  MCP -->|HTTP| API
  Eval -->|HTTP| API

  API -->|add_note| PG
  API -->|add_event| Extractor
  Extractor -->|evidence-bound notes| API
  API -->|persist| PG
  PG -->|outbox| Worker
  Worker -->|index chunks (dense + BM25)| Qdrant

  API -->|search| Expand{Expand?\noff/always/dynamic}
  Expand -->|original| Embed
  Expand -->|LLM variants| Extractor
  Extractor -->|expanded queries| Embed
  Embed -->|dense vectors| Qdrant
  API -->|BM25 query| Qdrant
  Qdrant -->|RRF fusion candidates| API
  API -->|scope/TTL filter| PG
  PG -->|notes| API
  API -->|rerank + recency| Rerank
  Rerank -->|scores| API
  API -->|top-k| Agent
```

## Comparison (qmd, claude-mem, mem0)

Comparison focuses on shared capabilities plus ELF strengths. These projects solve adjacent problems, but their primary units of storage and default workflows differ.

Note: In this section, mem0 refers to the Mem0 ecosystem, including OpenMemory (an MCP memory server with a built-in UI).

### Scope And Intended Use

| Aspect            | ELF                             | [qmd](https://github.com/tobi/qmd) | [claude-mem](https://github.com/thedotmack/claude-mem) | [mem0](https://github.com/mem0ai/mem0) |
| ----------------- | ------------------------------- | --------------------------------- | ------------------------------------------------------ | -------------------------------------- |
| Primary artifact  | Evidence-bound notes            | Local Markdown index (chunks)     | Session observations and summaries                      | User, session, and agent memories      |
| Default write path | HTTP `add_note` / `add_event`  | CLI index + search                | Auto-capture via Claude Code plugin hooks              | SDK/API (LLM-assisted)                 |
| Default deployment | API + worker + MCP server      | Local CLI + MCP server            | Local plugin + worker + UI + MCP tools                 | SDK + hosted option; OpenMemory MCP server + UI |

### Interfaces And Integration

| Capability                      | ELF | qmd | claude-mem | mem0 |
| ------------------------------- | --- | --- | ---------- | ---- |
| Local-first, self-hosted memory | ✅  | ✅  | ✅         | ✅ (OpenMemory) |
| MCP integration                 | ✅  | ✅  | ✅         | ✅ (OpenMemory) |
| HTTP API service                | ✅  | —   | ✅         | ✅ (SDK/API) |
| CLI-first workflow              | —   | ✅  | —          | —    |
| Web UI viewer                   | —   | —   | ✅         | ✅ (OpenMemory) |
| Hosted option                   | —   | —   | —          | ✅    |

### Retrieval Pipeline

| Capability                      | ELF | qmd | claude-mem | mem0 |
| ------------------------------- | --- | --- | ---------- | ---- |
| Full-text search (BM25 or FTS)  | ✅  | ✅  | ✅         | —    |
| Vector semantic search          | ✅  | ✅  | ✅         | ✅    |
| Hybrid dense + sparse fusion    | ✅  | ✅  | ✅         | —    |
| LLM reranking stage             | ✅  | ✅  | —          | —    |
| Query expansion                 | ✅  | ✅  | —          | —    |
| Progressive disclosure workflow | ✅  | —   | ✅         | —    |

### Quality, Safety, And Memory Semantics

| Capability                                | ELF | qmd | claude-mem | mem0 |
| ----------------------------------------- | --- | --- | ---------- | ---- |
| Evidence-bound notes (verbatim quotes)    | ✅  | —   | —          | —    |
| Deterministic vs LLM ingestion separation | ✅  | —   | —          | —    |
| Source-of-truth DB with rebuildable index | ✅  | —   | —          | —    |
| Multi-tenant scoping                      | ✅  | —   | —          | ✅ (user_id) |
| TTL and lifecycle policies                | ✅  | —   | —          | —    |
| English-only boundary enforcement         | ✅  | —   | —          | —    |
| Redaction on write                        | ✅  | —   | —          | —    |

### Operations And Evaluation

| Capability               | ELF | qmd | claude-mem | mem0 |
| ------------------------ | --- | --- | ---------- | ---- |
| Retrieval evaluation CLI | ✅  | —   | —          | —    |
| Structured JSON outputs  | ✅  | ✅  | ✅         | ✅    |

### ELF-Only Advantages

- Evidence binding with verbatim quote checks.
- Postgres is the source of truth; vector index is fully rebuildable.
- Deterministic `add_note` and LLM-only `add_event` semantics.
- Query expansion modes (`off`, `always`, `dynamic`) for cost/latency control.
- Dedicated evaluation CLI to measure retrieval quality.

### Learnings Integrated

- Hybrid retrieval + rerank as a first-class pipeline, inspired by qmd's local hybrid stack.
- Progressive cost control for retrieval, informed by claude-mem's progressive disclosure approach.

## Quickstart

### Requirements

- Postgres with pgvector
- Qdrant
- Provider endpoints for embeddings, rerank, and extraction

### Run

Copy `elf.example.toml` to `elf.toml`, then fill in provider and storage values. Initialize the Postgres schema and Qdrant collection once before starting the services. Start each service in a separate terminal.

```sh
cp elf.example.toml elf.toml
psql "<dsn from elf.toml>" -f sql/init.sql

export ELF_QDRANT_HTTP_URL="http://127.0.0.1:6334"
export ELF_QDRANT_COLLECTION="mem_notes_v1"
export ELF_QDRANT_VECTOR_DIM="4096"
./qdrant/init.sh

cargo run -p elf-worker -- -c elf.toml
cargo run -p elf-api -- -c elf.toml
cargo run -p elf-mcp -- -c elf.toml
```

### Evaluate

See `docs/guide/evaluation.md` for the dataset format and usage notes.

```sh
cargo run -p elf-eval -- -c elf.toml -i path/to/eval.json
```

## Configuration

See `elf.example.toml` and `docs/spec/system_elf_memory_service_v1.md` for the full contract. All config is explicit and required; no environment defaults are allowed. Embedding dimensions must match the Qdrant vector dimension. Search caching and explain trace retention are configured under `search.cache` and `search.explain`.

## Development

```sh
cargo make fmt
cargo make lint
cargo make test
```

## Support

If you find this project helpful and want to support its development:

- Ko-fi: https://ko-fi.com/hack_ink
- Afdian: https://afdian.com/a/hack_ink

- Bitcoin: `bc1pedlrf67ss52md29qqkzr2avma6ghyrt4jx9ecp9457qsl75x247sqcp43c`
- Ethereum: `0x3e25247CfF03F99a7D83b28F207112234feE73a6`
- Polkadot: `156HGo9setPcU2qhFMVWLkcmtCEGySLwNqa3DaEiYSWtte4Y`

## Appreciation

- The Rust community for their continuous support and development of the Rust ecosystem.

<div align="right">

### License

<sup>Licensed under [GPL-3.0](LICENSE).</sup>

</div>
