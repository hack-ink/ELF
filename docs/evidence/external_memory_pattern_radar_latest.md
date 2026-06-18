---
type: Evidence
title: "External Memory Pattern Radar Summary"
description: "Preserve the latest weekly ELF external memory pattern radar outcome."
resource: docs/evidence/external_memory_pattern_radar_latest.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - external-memory-pattern-radar
  - evidence
source_refs: []
code_refs:
  - apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json
  - apps/elf-eval/src/bin/external_memory_pattern_radar.rs
related: []
drift_watch:
  - docs/evidence/external_memory_pattern_radar_latest.md
---
# External Memory Pattern Radar Summary

Goal: Preserve the latest weekly ELF external memory pattern radar outcome.
Read this when: Feeding the next full comparison report or deciding whether a watched upstream memory project created an ELF follow-up.
Inputs: `apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json`, GitHub repository metadata, checked-in ELF comparison evidence, and any Codex source-review notes.
Depends on: `docs/spec/external_memory_pattern_radar_v1.md` and `docs/runbook/external_memory_pattern_radar.md`.
Outputs: Latest no-issue, rejection, or issue-ready radar decisions.

- Run id: `external-memory-pattern-radar-2026-06-10`
- Generated at: `2026-06-10T08:32:00.790878Z`
- Mode: `live`
- Projects: `16`; covered: `16`; rejected: `0`; gaps: `0`; create_issue: `0`

## Decisions

| Project | Upstream change | ELF verdict | Issue decision | Acceptance evidence |
| --- | --- | --- | --- | --- |
| `agentmemory` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `mem0` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `qmd` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `claude-mem` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `openviking` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `graphiti` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `letta` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `lightrag` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `graphrag` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `ragflow` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `memsearch` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `langgraph` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `nanograph` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `llm-wiki` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `gbrain` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |
| `graphify` | No GitHub metadata delta was observed since the prior cursor. | `covered` | `no_issue` | No-issue decision recorded in the cursor.; Coverage evidence points at checked-in ELF research docs. |

## Safety Boundary

- The radar records upstream movement as a trigger for source review, not as proof of parity or a reason to adopt an external runtime.
- `create_issue` decisions are valid only when the cursor includes source links, repo evidence, non-goals, validation criteria, and Linear duplicate-search evidence.
- No-issue runs remain useful because each project records why ELF is already covered or why metadata-only movement was rejected.
