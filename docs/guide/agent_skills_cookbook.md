# Agent Skills Cookbook (MCP-first)

Purpose: Provide reference agent-side workflows ("skills") for using ELF via MCP in a consistent, auditable, facts-first way.

Scope: This is a guide/playbook. It is non-normative and does not change the ELF system contract.

## 0) Contract: MCP vs Skills

### MCP tools (capability surface)

MCP tools are the model-controlled capability surface that forwards to ELF HTTP endpoints.
Treat every tool call as potentially attacker-influenced and rely on server-side enforcement.

MCP tools must:

- Be minimal and explicit (no hidden policy).
- Fail closed with stable error codes when a capability is disabled.
- Return structured, machine-readable outputs.

Hard guarantees that must be enforced by ELF (server-side), not by skills:

- English-only input boundary.
- Tenant/project/agent scoping and sharing grants.
- Size caps and quotas.
- Deterministic behavior where specified (e.g., `elf_notes_ingest` never calls an LLM).
- Auditability / provenance surfaces.

### Skills (workflow + policy)

Skills are agent-side workflows and policies that decide when and how to use tools.
Skills may call LLMs for summarization/planning, but they must be designed so that a tool cannot be misused if a skill is manipulated.

Skills should:

- Prefer facts-first memory (short notes) over storing raw long text in notes.
- Store long-form evidence in Doc Extension v1 and attach a pointer in the note `source_ref`.
- Hydrate evidence only when needed, using progressive disclosure (L0 -> L1 -> L2).
- Minimize writes, choose stable keys only when appropriate, and keep scopes explicit.

## 1) Tool glossary (MCP)

Memory (Core):

- `elf_notes_ingest` (deterministic; never calls an LLM)
- `elf_events_ingest` (LLM extraction; evidence-bound)
- `elf_search_quick_create` / `elf_search_planned_create`
- `elf_searches_get` / `elf_searches_timeline` / `elf_searches_notes`
- `elf_notes_list` / `elf_notes_get` / `elf_notes_patch` / `elf_notes_delete`
- `elf_notes_publish` / `elf_notes_unpublish`
- `elf_space_grants_list` / `elf_space_grant_upsert` / `elf_space_grant_revoke`

Docs (Extension v1):

- `elf_docs_put`
- `elf_docs_get`
- `elf_docs_search_l0` (discovery/backfill/debug; not a full search platform)
- `elf_docs_excerpts_get` (bounded evidence hydration)

Note: In the current MCP adapter, `read_profile` is configured on the MCP server and is not client-controlled for search/doc search tools.

## 2) Data contract: facts-first + evidence pointers

### Notes are facts-first

Notes should be compact English statements suitable for retrieval:

- One atomic fact per note where possible.
- Use stable `key` only for durable, updatable truths (preferences, constraints, decisions, profiles).
- Use unkeyed notes for one-off facts that should not overwrite.

### Evidence is hydrated via `source_ref`

When a note depends on long-form evidence, attach a versioned pointer in `source_ref`.

Recommended convention:

- `source_ref.schema = "source_ref/v1"`
- `source_ref.resolver = "elf_doc_ext/v1"`
- Include `doc_id` (required) and optional selector hints:
  - `chunk_id` (from `elf_docs_search_l0`), or
  - `quote` selector (exact + optional prefix/suffix), and optional `position` fallback.

Keep `source_ref` ASCII-safe and stable over time.

## 3) Workflow: doc_ingest (long evidence -> compact notes)

Goal: Persist a long evidence source in Doc Extension v1 and store compact facts in Core notes with a pointer back to the evidence.

Steps:

1. Canonicalize upstream inputs to English (ELF rejects non-English at the API boundary).
2. Store the long evidence with `elf_docs_put`.
3. Extract a small number of durable facts (agent-side) and write them via `elf_notes_ingest`.
4. Attach a `source_ref` pointer (doc_id + optional selector hints) to each note.
5. Pass `explain` in docs endpoints only when you need debug diagnostics.

Minimal example: `elf_docs_put`

```json
{
  "scope": "project_shared",
  "title": "Decision record: search routing",
  "source_ref": {
    "schema": "doc_source_ref/v1",
    "doc_type": "knowledge",
    "ts": "2026-02-28T00:00:00Z"
  },
  "content": "Long-form English evidence text..."
}
```

Minimal example: `elf_notes_ingest` (facts-first notes with pointers)

```json
{
  "scope": "project_shared",
  "notes": [
    {
      "type": "decision",
      "key": "doc.v1.routing_scope",
      "text": "Doc Extension v1 supports only docs_search_l0 discovery; all evidence reading uses docs_excerpts_get.",
      "importance": 0.7,
      "confidence": 0.8,
      "ttl_days": null,
      "source_ref": {
        "schema": "source_ref/v1",
        "resolver": "elf_doc_ext/v1",
        "doc_id": "00000000-0000-0000-0000-000000000000"
      }
    }
  ]
}
```

Operational guidance:

- Prefer <= 3–7 notes per doc ingest unless you have a strong reason (avoid memory spam).
- If the fact is expected to evolve, provide a stable `key` so updates are possible.
- If the doc is sensitive, choose `agent_private` scope and only publish explicitly later.

## 4) Workflow: hydrate_context (note hit -> bounded excerpt)

Goal: Given a retrieved note, hydrate supporting evidence only when needed and only in bounded windows.

Recommended strategy:

1. Retrieve candidate notes via `elf_search_quick_create` (fast) or `elf_search_planned_create` (when you want `query_plan`).
2. If you need to cite/verify, resolve the note `source_ref`:
   - If it includes `doc_id` + `chunk_id` or selector hints: call `elf_docs_excerpts_get` directly.
     - Include `locator` fields from `source_ref` as available: `quote` and/or `position`.
   - Otherwise: call `elf_docs_search_l0` to find a relevant chunk_id, then hydrate using `elf_docs_excerpts_get`.
3. Use progressive disclosure:
   - Start with `level = "L1"` and upgrade to `L2` only when the first excerpt is insufficient.
   - Use `level = "L0"` for tight, cheapest verification checks (`~256` bytes).

Optional debug mode:

- Pass `explain: true` in `elf_docs_search_l0` or `elf_docs_excerpts_get` when you need to collect trace diagnostics.
- Keep an eye on `trace_id` and optional `trajectory` for observability.
- Use `locator` from excerpts to persist preferred selectors for reruns.

Minimal example: `elf_docs_search_l0` (discovery)

```json
{
  "query": "Why do we avoid a full doc search platform in v1?"
}
```

Minimal example: `elf_docs_excerpts_get` (hydration)

```json
{
  "doc_id": "00000000-0000-0000-0000-000000000000",
  "level": "L1",
  "chunk_id": "00000000-0000-0000-0000-000000000000"
}
```

Verification guidance:

- Prefer `verified=true` excerpts as evidence.
- Treat `verified=false` as best-effort context and avoid using it as hard proof without revalidation.

## 5) Workflow: memory_write_policy (when to write and how)

Goal: Keep writes minimal, consistent, and update-friendly.

### Choose `elf_notes_ingest` vs `elf_events_ingest`

- Use `elf_notes_ingest` when:
  - You already have a compact English fact to store.
  - You want deterministic behavior and strict control over stored text.
  - You are ingesting outputs of other tools (docs, logs) after agent-side normalization.

- Use `elf_events_ingest` when:
  - You want the server to run its LLM extractor to produce evidence-bound notes.
  - You have strong evidence text and can provide verifiable quotes.

### Keys

- Use a stable key for:
  - preferences: editor, language, workflow defaults
  - constraints: build rules, security rules, invariants
  - decisions: architectural choices, selected options, adopted conventions
  - profiles: stable descriptions of agents/projects

- Avoid keys for:
  - one-off facts that should not overwrite each other
  - uncertain observations

### Scope

- `agent_private`: private scratchpad and personal preferences.
- `project_shared`: shared team memory inside a project.
- `org_shared`: shared memory across projects inside a tenant (publish explicitly).

## 6) Workflow: share_workflow (publish + grants)

Goal: Make shared memory explicit and reversible.

Pattern:

1. Keep drafts `agent_private`.
2. When stable, publish to `project_shared` or `org_shared` using `elf_notes_publish`.
3. Grant explicit read access to other agents using `elf_space_grant_upsert`.
4. Revoke or unpublish when needed.

Reminder: sharing is enforced by scopes + grants. Treat this as part of the memory contract, not an optional convention.

Note: Sharing tools operate on `space` values `team_shared` and `org_shared` (where `team_shared` corresponds to project-level sharing).

Minimal examples:

Publish a note to team-shared space:

```json
{
  "note_id": "00000000-0000-0000-0000-000000000000",
  "space": "team_shared"
}
```

Grant access to a specific agent:

```json
{
  "space": "team_shared",
  "grantee_kind": "agent",
  "grantee_agent_id": "agent_abc123"
}
```

Revoke that grant:

```json
{
  "space": "team_shared",
  "grantee_kind": "agent",
  "grantee_agent_id": "agent_abc123"
}
```

## 7) Workflow: reflect_consolidate (episodic -> stable facts)

Goal: Periodically reduce memory noise and keep stable truths current.

Simple loop (agent-side):

1. Pull recent/high-hit notes (`elf_notes_list` with filters) and recent decisions (stable key prefixes).
2. Identify duplicates, conflicts, and near-expiry items.
3. Produce a small set of updates:
   - update stable-key notes when the truth changed
   - deprecate or delete notes that are no longer valid
4. Optionally attach a doc pointer explaining why the consolidation happened.

Non-goal: This loop must not be required for ELF correctness. It is an optimization for better context usage.

Minimal example: `elf_notes_list` (pull candidates)

```json
{
  "scope": "project_shared",
  "status": "active",
  "type": "decision"
}
```

## 8) Failure modes and safety checklist

- Prompt injection: assume an attacker can influence skill reasoning. Tool-side authz and input gates must still protect you.
- Over-writing: do not introduce stable keys unless you are willing to overwrite.
- Excessive writes: cap how many notes you ingest per session/doc.
- Hydration blowups: start at L1; upgrade to L2 only on demand.
- Drift: keep workflows centralized and versioned. When tool contracts change, update the cookbook first.

## 9) Prompt templates (agent-side)

These templates are optional. They are provided to reduce drift across agents.
Do not treat them as server contracts.

### Template: extract facts from a doc into `elf_notes_ingest` JSON

System:

You are a memory normalization engine for a facts-first agent memory system.
Output must be valid JSON only.
Output must match the schema described below exactly.
All text must be English only.
Each note text must be a single compact sentence.
Prefer stable keys only for durable truths (preferences, constraints, decisions, profiles).

User:

Return JSON matching this schema:
{
  "scope": "agent_private|project_shared|org_shared",
  "notes": [
    {
      "type": "preference|constraint|decision|profile|fact|plan",
      "key": "string|null",
      "text": "string",
      "importance": 0.0,
      "confidence": 0.0,
      "ttl_days": "integer|null",
      "source_ref": {
        "schema": "source_ref/v1",
        "resolver": "elf_doc_ext/v1",
        "doc_id": "uuid"
      }
    }
  ]
}

Constraints:
- MAX_NOTES = 7
- Every note must include a `source_ref` pointer to doc_id = <DOC_ID>.

Doc title: <TITLE>
Doc content:
<CONTENT>

### Template: consolidation pass (suggest patches or deletes)

System:

You are a memory consolidation engine.
Decide a minimal set of safe changes to reduce duplicates and keep stable keys accurate.
All output must be English only.

User:

Given these notes (JSON), produce a plan (English bullets) that includes:
- Which notes to delete (note_id)
- Which notes to patch (note_id + new text)
- Which new stable-key notes to add (notes_ingest JSON)

Notes:
<NOTES_JSON>

## 10) Pinned references (internal)

- Core contract: `docs/spec/system_elf_memory_service_v2.md`
- Doc Extension v1 design: `docs/plans/2026-02-24-doc-ext-v1-design.md`
- Doc pointer resolver: `docs/spec/system_source_ref_doc_pointer_v1.md`
