# Org-Shared (Tenant-Wide) Semantics Design
Date: 2026-02-22

## Summary
This design defines `org_shared` as **tenant-wide shared memory** (organization scope) rather than a project-scoped variant of `team_shared`/`project_shared`.

Because the current storage model and access controls are keyed on `(tenant_id, project_id, scope)`, this design implements tenant-wide `org_shared` by introducing an **org sentinel project** (`project_id="__org__"`) that holds all org-scoped notes and grants. Reads from any project are extended to include org-scoped notes in addition to the current project’s notes, while preserving explicit sharing via `memory_space_grants`.

Writes to `org_shared` (ingest, publish, and grant management that can affect tenant-wide visibility) are gated behind `SecurityAuthRole::{Admin,SuperAdmin}` when `security.auth_mode="static_keys"`.

## Goals
- Define `org_shared` semantics that are consistent across projects within a tenant.
- Preserve explicit sharing and auditability (no “implicit readability” without a grant).
- Avoid weakening isolation guarantees between projects.
- Minimize schema changes and blast radius by reusing existing tables and indexes.

## Non-Goals
- Making `X-ELF-Project-Id` optional across the public HTTP API.
- Introducing agentless tokens for normal endpoints.
- Adding a full organization membership registry.
- Implementing moderation workflows for promoting notes into `org_shared` (can be added later).

## Definitions
- **Tenant**: The top-level namespace keyed by `tenant_id`.
- **Project**: A sub-namespace keyed by `project_id` within a tenant.
- **Org sentinel project**: `project_id="__org__"`, reserved for tenant-wide (`org_shared`) storage.
- **team_shared**: Public API alias for internal `project_shared` (project-wide sharing).
- **org_shared**: Tenant-wide sharing, stored under the org sentinel project.

## Current Constraints (Why this change is needed)
- Public HTTP request context requires `tenant_id`, `project_id`, and `agent_id`.
- Storage tables and grant tables require `project_id NOT NULL`.
- Shared grants are currently loaded by `(tenant_id, project_id, grantee_agent_id)` and treat `org_shared` as project-scoped.

## Data Model
### Notes
- All notes continue to live in `memory_notes`.
- `org_shared` notes are stored with:
  - `tenant_id = <tenant>`
  - `project_id = "__org__"`
  - `scope = "org_shared"`

### Grants
- Grants continue to live in `memory_space_grants`.
- Grants for `org_shared` are stored with:
  - `tenant_id = <tenant>`
  - `project_id = "__org__"`
  - `scope = "org_shared"`
- `grantee_kind="project"` in the org sentinel project is defined as **tenant-wide read access** (all agents that can make requests within the tenant).

## API Semantics
### Reads (list/search/details)
When `org_shared` is included in the resolved allowed scopes for the request’s `read_profile`:
- Queries that currently filter by `(tenant_id, project_id)` are extended to include:
  - `(tenant_id, project_id = <request project>)` and
  - `(tenant_id, project_id = "__org__")` for `org_shared` only.

This yields a hierarchical view:
- `agent_private`: only the caller’s agent_id, project-scoped.
- `team_shared`/`project_shared`: project-scoped.
- `org_shared`: tenant-wide (org sentinel project).

### Writes
#### Ingest (add_note / add_event)
- If request scope is `org_shared`, the note is written to `project_id="__org__"` (not the caller’s project).
- If request scope is `project_shared` or `agent_private`, behavior is unchanged.

#### Publish / Unpublish
- Publishing a note to `org_shared` moves the note to `project_id="__org__"` and sets `scope="org_shared"`.
- Publishing to `team_shared`/`project_shared` remains project-scoped and creates a project-wide grant as today.

#### Grant management
- `org_shared` grant upsert/revoke/list operate on `project_id="__org__"` regardless of caller project.

## Authorization
### Static keys (`security.auth_mode="static_keys"`)
- `org_shared` **writes** require `SecurityAuthRole::{Admin,SuperAdmin}`:
  - ingest with `scope="org_shared"`
  - publish/unpublish to `space="org_shared"`
  - org_shared grant upsert/revoke
- `org_shared` reads are allowed for `User` tokens if the requested `read_profile` includes `org_shared` and an applicable grant exists (including org “project” grants).

### Auth mode off (`security.auth_mode="off"`)
- Treated as a trusted localhost mode; role gating is not enforceable without an auth key.
- The service should remain usable for local testing; operational deployments should use `static_keys`.

## Data Flow (Org Shared Read)
1. Resolve allowed scopes from `read_profile`.
2. Load shared read grants for the caller project.
3. If `org_shared` is allowed, also load shared read grants from the org sentinel project.
4. Execute list/search with a combined view of:
   - project-scoped notes
   - org-scoped notes (org sentinel project)
5. Apply `note_read_allowed` based on scope, status/ttl, and grants.

## Migration
- New semantics require moving existing project-scoped `org_shared` notes and grants into the org sentinel project.
- Provide a one-time SQL migration script and document operational steps:
  - Update `memory_notes.project_id` to `"__org__"` where `scope="org_shared"`.
  - Update `memory_space_grants.project_id` to `"__org__"` where `scope="org_shared"`.

## Testing
- Add acceptance tests that demonstrate cross-project visibility:
  - Create an `org_shared` note (admin write).
  - Verify an agent in a different project can retrieve it via list/search when `org_shared` is allowed.
  - Verify revocation removes visibility.
- Add negative tests:
  - `User` token cannot ingest/publish/grant for `org_shared` in `static_keys` mode.

## Risks and Mitigations
- **Accidental tenant-wide publication**: mitigated by Admin/SuperAdmin write gating.
- **Back-compat**: existing `org_shared` data needs migration; include explicit operator runbook and a rollback plan (restore prior project_id values from backups).
- **Confusion over “project” grantee_kind in org scope**: mitigate via explicit spec wording and tests.

## Open Questions
- Should `org_shared` reads require Admin role (stricter) or remain user-readable when granted? (Current design: user-readable when granted.)
- Should we add an explicit `grantee_kind="tenant"` in the future to avoid overloading `project`? (Deferred.)

