# Issue Labeling

This guide standardizes how GitHub issues are labeled in this repository.

## Goals

- Make issues easy to route to the right owner (system area).
- Make the intent of an issue explicit (feature, bug, architecture, spec, research, performance, chore).
- Support cross-cutting workflows by tagging evaluation, reliability, provenance, cost, and governance themes.

## Label description style

Label descriptions must be short, clear sentences and must end with terminal punctuation (usually a period).

## Label taxonomy

### `kind:*` (required, exactly one)

Every issue must have exactly one `kind:*` label.

- `kind:epic`: Umbrella issue that tracks multiple deliverables.
- `kind:feat`: New capability or product behavior that is not primarily a refactor or cleanup.
- `kind:arch`: Architecture and design changes that affect system shape, boundaries, or major flows.
- `kind:spec`: Specification or contract definition (APIs, schemas, invariants, query semantics).
- `kind:research`: Investigation, evaluation, or spike that produces a decision memo or research artifact.
- `kind:perf`: Performance and efficiency improvements (latency, throughput, storage, cost).
- `kind:bug`: Something is not working as intended.
- `kind:chore`: Maintenance work that does not fit other kinds.

### `area:*` (required, one or more)

Every issue must have at least one `area:*` label.

Use `area:*` for ownership and routing. Prefer one primary area and add additional areas only when the change clearly spans multiple subsystems.

Current areas:

- `area:api`: HTTP API service and request/response contracts.
- `area:service`: Retrieval logic, ranking, and request orchestration.
- `area:storage`: Postgres schema, SQL queries, and storage correctness.
- `area:providers`: Embedding, rerank, and extractor provider integrations.
- `area:worker`: Background workers, outbox processing, and indexing pipelines.
- `area:mcp`: MCP server and tool routing.
- `area:ui`: Viewer and developer-facing UI work.
- `area:docs`: Documentation and developer experience docs.
- `area:ops`: Local dev, scripts, and operational runbooks.
- `area:security`: Authentication, secrets, and security hygiene.
- `area:observability`: Logging, tracing, and diagnostics.

### `status:*` (optional, at most one)

Use `status:*` when an issue is intentionally not progressing.

- `status:deferred`: Not planned for the near term.
- `status:blocked`: Cannot proceed until dependencies are resolved. The issue body should include a short "Blocked by" section.

### `theme:*` (optional, any number)

Use `theme:*` to tag cross-cutting concerns that benefit from consistent closed-loop workflows.

- `theme:governance`: Approval workflows, review queues, policy, and auditability.
- `theme:evaluation`: Quality measurement, gold sets, regressions, and metrics.
- `theme:provenance`: Evidence, citations, lineage, and explainability.
- `theme:reliability`: Correctness, consistency, failure handling, and operational robustness.
- `theme:cost`: Latency, compute, storage, and cost controls.

### Reserved labels

These labels exist for automation and should not be repurposed.

- `dependencies`: Dependency updates (Dependabot and tooling).
- `bot`: Automated issue or pull request created by a bot.

## Labeling rules

1. Add exactly one `kind:*` label.
2. Add at least one `area:*` label.
3. Add `status:*` only when it materially affects planning (deferred or blocked).
4. Add `theme:*` labels when the work is explicitly about governance, evaluation, provenance, reliability, or cost.

## Examples

- Postgres schema correctness bug:
  - `kind:bug`, `area:storage`, `theme:reliability`.
- Add an optional auth mechanism:
  - `kind:feat`, `area:api`, `area:security`, `theme:governance`.
- Retrieval ranking experiment:
  - `kind:research`, `area:service`, `theme:evaluation`.
- Performance work postponed:
  - `kind:perf`, `area:service`, `status:deferred`, `theme:cost`.

## Query patterns

- All epics: `label:kind:epic`.
- Open feature work: `label:kind:feat is:open`.
- Reliability issues in storage: `label:area:storage label:theme:reliability`.
