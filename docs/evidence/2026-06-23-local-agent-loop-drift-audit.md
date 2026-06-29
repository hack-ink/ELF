---
type: Drift Audit
title: "Local Agent Loop Drift Audit"
description: "Drift audit for the one-command local setup and agent integration recipes."
resource: docs/evidence/2026-06-23-local-agent-loop-drift-audit.md
status: active
authority: current_state
owner: docs
last_verified: 2026-06-23
tags:
  - docs
  - drift-audit
  - agent-setup
source_refs: []
code_refs:
  - Makefile.toml
  - scripts/local-agent-loop.sh
  - config/local/elf.docker.toml
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/app/server.rs
  - apps/elf-mcp/src/app/server/tools/docs.rs
related:
  - docs/runbook/agent-setup.md
  - docs/runbook/agent_skills_cookbook.md
  - docs/runbook/getting_started.md
drift_watch:
  - Makefile.toml
  - scripts/local-agent-loop.sh
  - config/local/elf.docker.toml
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/app/server.rs
  - apps/elf-mcp/src/app/server/tools/docs.rs
---
# Local Agent Loop Drift Audit

Purpose: Anchor the one-command local setup and agent integration recipe to the
current task, config, HTTP, and MCP surfaces.
Read this when: You need evidence behind `cargo make local-agent-loop`, the local
agent memory+knowledge demo, or the Codex/Claude/Cursor/MCP/CLI recipes.
Not this document: Production deployment, benchmark interpretation, or hosted
provider quality evidence.

## Watched Claims

- `cargo make local-agent-loop` is a repository-native entrypoint for the local
  agent memory+knowledge loop.
- The deterministic local recipe uses `config/local/elf.docker.toml`, local Docker
  Postgres/Qdrant, and local deterministic embedding/rerank providers.
- The local recipe imports source evidence, writes a source-linked memory candidate,
  creates and applies a reviewable proposal, recalls approved memory, inspects recall
  debug, and exercises correction/rollback.
- The deterministic path does not call `elf_events_ingest`, LLM query expansion, or a
  hosted extractor.
- MCP client recipes use the current `elf-mcp` tool surface for agent-facing docs,
  note ingest, search, and recall-debug calls.

## Evidence Anchors

- `Makefile.toml` defines `local-agent-loop`.
- `scripts/local-agent-loop.sh` owns the runnable local loop and writes artifacts
  under `tmp/local-agent-loop/`.
- `config/local/elf.docker.toml` binds local HTTP/admin/MCP services to loopback and
  configures local deterministic embedding/rerank providers with query expansion off.
- `apps/elf-api/src/routes.rs` exposes the demo HTTP routes:
  - `POST /v2/docs`
  - `POST /v2/notes/ingest`
  - `POST /v2/searches`
  - `POST /v2/recall-debug/panel`
  - `POST /v2/admin/consolidation/runs`
  - `POST /v2/admin/consolidation/proposals/{proposal_id}/review`
  - `POST /v2/admin/notes/{note_id}/corrections`
- `apps/elf-mcp/src/app/server.rs` and
  `apps/elf-mcp/src/app/server/tools/docs.rs` expose the agent-facing MCP tools:
  - `elf_docs_put`
  - `elf_notes_ingest`
  - `elf_searches_create`
  - `elf_recall_debug_panel`

## Reverse Checks

- Run `bash -n scripts/local-agent-loop.sh` after script changes.
- Run `cargo make check-docs` after docs or task-name changes.
- Run the registered repository gate before handoff.

## Verdict

pass

## Required Updates

- Update `docs/runbook/agent-setup.md`, this drift audit, and
  `docs/runbook/agent_skills_cookbook.md` if the local config, MCP tool names, or
  demo route sequence changes.
- Do not convert the deterministic local recipe into a provider-backed quality claim
  unless provider credentials, corpus ownership, and scored evidence are supplied.

## Citations

- `Makefile.toml`
- `scripts/local-agent-loop.sh`
- `config/local/elf.docker.toml`
- `apps/elf-api/src/routes.rs`
- `apps/elf-mcp/src/app/server.rs`
- `apps/elf-mcp/src/app/server/tools/docs.rs`
