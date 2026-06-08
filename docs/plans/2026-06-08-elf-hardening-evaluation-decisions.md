# ELF Hardening Evaluation Decisions

**Date:** 2026-06-08

## Goal

Record the system evaluation decisions that drove the June 2026 ELF reliability
hardening work, so the rationale lives in the repository instead of only in chat
or tracker history.

## Context

The evaluation found several gaps that made local operation and API contract
review harder than necessary:

- Required runtime gates and service-backed checks needed to be restored and
  made easy to run.
- The MCP default ingestion profile update path needed an explicit PUT-backed
  contract.
- New operators needed a concrete getting-started path with local service setup.
- The HTTP API contract needed a generated, inspectable surface.
- Configuration should reject missing required fields instead of silently
  accepting ambiguous defaults.
- Local development needed a Docker Compose stack for the service dependencies.

## Selected Decisions

### 1) Restore gates, MCP default-set PUT, and getting-started docs

Decision: implement the gate restoration, MCP default-set PUT forwarding, and
operator getting-started documentation as one bounded reliability lane.

Tracking:

- Linear: [XY-789](https://linear.app/hack-ink/issue/XY-789/elf-hardening-14-restore-gates-mcp-default-set-put-and-getting-started)
- GitHub: [PR #109](https://github.com/hack-ink/ELF/pull/109)

Verification expectation:

- Service-backed integration coverage must be runnable through the repository
  checks.
- MCP default ingestion profile updates must use the API contract path rather
  than a parallel local-only behavior.
- Setup documentation must be enough for an operator to start the local system
  without relying on chat context.

### 2) Use utoipa and Scalar for the API contract surface

Decision: use `utoipa` for OpenAPI generation and Scalar for the browsable API
reference.

Tracking:

- Linear: [XY-790](https://linear.app/hack-ink/issue/XY-790/elf-hardening-24-add-utoipa-and-scalar-api-contract-surface)
- GitHub: [PR #111](https://github.com/hack-ink/ELF/pull/111)

Verification expectation:

- The generated OpenAPI document must cover the v2 HTTP routes needed by
  operators and tests.
- The Scalar UI must be served by the API app without requiring a separate docs
  process.
- Contract tests should assert the key route and schema names so the docs
  surface cannot drift silently.

### 3) Enforce stricter configuration field presence

Decision: make required configuration fields explicit and reject missing required
fields instead of accepting implicit defaults for operator-critical behavior.

Tracking:

- Linear: [XY-791](https://linear.app/hack-ink/issue/XY-791/elf-hardening-34-enforce-strict-config-field-presence)
- GitHub: [PR #110](https://github.com/hack-ink/ELF/pull/110)

Verification expectation:

- Config validation tests must cover required-field failures.
- Existing valid fixtures must keep passing after the stricter read path.
- Error messages should identify the missing field clearly enough for operator
  remediation.

### 4) Use Docker Compose for local service setup

Decision: use Docker Compose as the repo-owned local development stack for
Postgres, Qdrant, and the API/MCP-facing runtime dependencies.

Tracking:

- Linear: [XY-792](https://linear.app/hack-ink/issue/XY-792/elf-hardening-44-add-docker-compose-local-dev-stack)
- GitHub: [PR #112](https://github.com/hack-ink/ELF/pull/112)

Verification expectation:

- The compose stack must avoid colliding with unrelated local services.
- The documented environment should map directly to the repo-native checks and
  getting-started flow.
- Compose configuration should remain development-only and not introduce a new
  production deployment contract.

## Deferred / Non-goals

- Item 7 from the evaluation was explicitly ignored for this hardening pass.
- This plan does not introduce live provider calls, new hosted infrastructure,
  or a replacement runtime architecture.
- This plan does not make Docker Compose the production deployment surface.

## Delivery Order

The implementation order is:

1. Restore gates, MCP default-set PUT, and getting-started docs.
2. Add the utoipa + Scalar API contract surface.
3. Enforce stricter configuration field presence.
4. Add the Docker Compose local dev stack.
5. Land this decision record so future maintenance can trace the work back to
   the evaluated system gaps.

Each implementation lane should land only after repo-native verification passes,
with service-backed checks used where behavior depends on Postgres or Qdrant.
