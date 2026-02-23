# Org-Shared (Tenant-Wide) Semantics Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement tenant-wide `org_shared` semantics using an org sentinel project (`project_id="__org__"`), including cross-project reads and Admin/SuperAdmin-gated writes in `static_keys` mode.

**Architecture:** Treat `tenant_id` as the org boundary. Store all `org_shared` notes and grants under a reserved `project_id="__org__"`. Read paths union org + project scopes; write paths route org_shared to the org sentinel and enforce role-based gating at the HTTP layer.

**Tech Stack:** Rust (axum, sqlx), Postgres, existing ELF config/auth (`SecurityAuthRole`).

---

### Task 1: Introduce the org sentinel constant and reservation rules

**Files:**
- Modify: `packages/elf-service/src/access.rs`
- Modify: `packages/elf-service/src/sharing.rs`
- (Optional) Modify: `docs/spec/system_elf_memory_service_v2.md`

**Step 1: Add a single source of truth constant**
- Add `const ORG_PROJECT_ID: &str = "__org__";` in a shared module used by access + sharing (pick the lowest-impact existing module; avoid creating new crates).

**Step 2: Document reservation**
- Add a short note in the spec that `__org__` is reserved and not a user project id.

**Step 3: Verify**
- Run: `cargo make test-rust`
- Expected: PASS

**Step 4: Commit (optional)**
```bash
git add packages/elf-service/src/access.rs packages/elf-service/src/sharing.rs docs/spec/system_elf_memory_service_v2.md
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"sharing","summary":"Define org sentinel project id","intent":"Add a reserved project id for org_shared storage","impact":"Centralizes __org__ constant for later org_shared semantics","breaking":false,"risk":"low","refs":["gh:hack-ink/ELF#72"]}'
```

### Task 2: Propagate auth role to request handling (static_keys mode)

**Files:**
- Modify: `apps/elf-api/src/routes.rs`
- Add tests: `apps/elf-api/tests/http.rs`

**Step 1: Add role propagation mechanism**
- In `api_auth_middleware`, after resolving `key`, attach `key.role` to the request for downstream handlers.
  - Preferred: `req.extensions_mut().insert(key.role);`
  - Avoid: new public headers (keep role server-side).

**Step 2: Add helper to require Admin for org_shared writes**
- Implement `fn require_admin_for_org_shared(role: Option<&SecurityAuthRole>, ...) -> Result<(), ApiError>`
- Call it from endpoints that can write org_shared:
  - notes ingest (`/v2/notes/ingest`) when `scope == "org_shared"`
  - events ingest (`/v2/events/ingest`) when `scope == Some("org_shared")`
  - publish/unpublish when `space == "org_shared"`
  - grant upsert/revoke for `space == "org_shared"`

**Step 3: Tests**
- Add tests that a `User` key cannot org_shared ingest/publish/grant.
- Add tests that an `Admin` key can org_shared ingest/publish/grant.

**Step 4: Verify**
- Run: `cargo make test-rust`
- Expected: PASS

### Task 3: Route org_shared writes to the org sentinel project

**Files:**
- Modify: `apps/elf-api/src/routes.rs`
- Modify: `packages/elf-service/src/add_note.rs`
- Modify: `packages/elf-service/src/add_event.rs`
- Modify: `packages/elf-service/src/sharing.rs`
- Add tests: `packages/elf-service/tests/acceptance/suite.rs` (or a new acceptance test module under `packages/elf-service/tests/acceptance/`)

**Step 1: Ingest routing**
- When request scope is `org_shared`, replace `project_id` passed to the service with `ORG_PROJECT_ID`.

**Step 2: Publish routing**
- When publishing/unpublishing `space == "org_shared"`, operate against the org sentinel project id for:
  - the note lookup
  - the scope update
  - the grant creation

**Step 3: Add a tenant-wide project grant on org publish**
- Ensure that publishing to org_shared creates a grant row with:
  - `tenant_id=<tenant>`
  - `project_id="__org__"`
  - `scope="org_shared"`
  - `grantee_kind="project"`
  - `space_owner_agent_id=<note owner>`

**Step 4: Cross-project acceptance test**
- Setup:
  - tenant: `t`
  - project A: `a`
  - project B: `b`
  - admin agent: `admin1`
  - user agent in B: `user2`
- Flow:
  1) Ingest a private note as `admin1` in project A.
  2) Publish it to `org_shared` (admin role).
  3) Search/list from project B as `user2` with `read_profile` that includes `org_shared`.
  4) Assert the note is visible.

**Step 5: Verify**
- Run: `cargo make test-rust`
- Expected: PASS

### Task 4: Extend read paths to include org_shared across projects

**Files:**
- Modify: `packages/elf-service/src/access.rs`
- Modify: `packages/elf-service/src/list.rs`
- Modify: `packages/elf-service/src/search.rs`
- Modify: `packages/elf-service/src/progressive_search.rs`

**Step 1: Load org grants in addition to project grants**
- If allowed scopes include `org_shared`, call `load_shared_read_grants(..., project_id="__org__", ...)` and union with project grants.

**Step 2: Extend note queries**
- For list/search queries that currently filter by `project_id = $project`, extend to:
  - include org notes (`project_id="__org__"`) for `scope="org_shared"` only
  - avoid accidentally including `agent_private` from org sentinel (should not exist).

**Step 3: Verify**
- Run: `cargo make test-rust`
- Expected: PASS

### Task 5: Operational migration script + runbook

**Files:**
- Add: `sql/migrate_org_shared_to_org_project.sql`
- Modify: `docs/spec/system_elf_memory_service_v2.md`

**Step 1: Add a migration SQL script**
- Write a safe, explicit script that moves existing `org_shared` rows into the org sentinel project:
  - `UPDATE memory_notes SET project_id='__org__' WHERE scope='org_shared' AND project_id <> '__org__';`
  - `UPDATE memory_space_grants SET project_id='__org__' WHERE scope='org_shared' AND project_id <> '__org__';`
- Include a `BEGIN; ... COMMIT;` wrapper and a `SELECT count(*)` before/after.

**Step 2: Document runbook + rollback**
- Document:
  - pre-checks (backups, counts)
  - how to run the script
  - rollback expectations (restore from backup; optional reverse-update if previous mapping recorded)

**Step 3: Verify**
- Run: `cargo make test-rust`
- Expected: PASS

---

Plan complete and saved to `docs/plans/2026-02-22-org-shared-implementation-plan.md`.

Two execution options:
1) **Subagent-Driven (this session)** — execute tasks one-by-one with review checkpoints
2) **Parallel Session (separate)** — open a new session and execute with `executing-plans` checkpoints

Which approach do you want?

