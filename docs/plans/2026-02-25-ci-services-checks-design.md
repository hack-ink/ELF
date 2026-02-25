# CI Service-Backed Checks Design

**Date:** 2026-02-25

## Goal

Make service-backed verification (Postgres + Qdrant) a first-class, always-on check for changes that can affect retrieval correctness, while keeping the heavier harness signals as nightly-only trend indicators.

## Context

Today the repository already runs:

- Fast checks on PR/push/merge queue: `.github/workflows/language.yml` and `.github/workflows/quality.yml`.
- Service-backed integration tests on a schedule: `.github/workflows/integration.yml` (daily).
- Service-backed harness scripts on a schedule: `.github/workflows/nightly-harness-signals.yml` (nightly).

Local developer guidance for service-backed testing lives in:

- `docs/guide/integration-testing.md`
- `docs/guide/testing.md`

## Requirements

- Do not rely on external providers or secrets for correctness checks.
- Run service-backed checks on both:
  - `pull_request` (fast feedback for contributors)
  - `merge_group` (merge queue parity)
- Avoid running on docs-only changes.
- Ensure we can run the full Rust test surface, including ignored tests that require services, without leaving coverage gaps.
- Keep the heavier harness scripts (trend/signal) separate from gating checks.

## Non-goals

- Do not build a full “retrieval quality platform” in CI.
- Do not add provider-backed LLM/embedding calls to required checks.
- Do not change ranking logic or memory semantics as part of this work.

## Design

### 1) Always-on integration tests with services

Update `.github/workflows/integration.yml` to run on PR and merge queue (in addition to schedule + manual).

In this workflow, run the full workspace test suite including ignored tests:

- `cargo nextest run --workspace --all-targets --all-features --run-ignored all`

Rationale:

- This makes “ignored tests” a convention for “requires services”, not “unexecuted”.
- It keeps the “no skipped tests” expectation enforceable in CI.

### 2) Always-on E2E harness (lightweight)

Add a new workflow to run the lightweight, deterministic E2E harness:

- `cargo make e2e` (which runs `scripts/context-misranking-harness.sh`)

Key properties:

- Uses local deterministic providers (`local-hash`, `local-token-overlap`).
- Uses Postgres + Qdrant services only.
- Produces clear pass/fail semantics and can upload logs on failure.

### 3) Keep “harness signals” nightly-only

Do not change `.github/workflows/nightly-harness-signals.yml` scope: it remains nightly + manual and continues to upload artifacts. This job can evolve independently without becoming a hard merge gate.

## Acceptance criteria

- `Integration Tests` runs on:
  - `pull_request`, `merge_group`, `schedule`, `workflow_dispatch`
- `Integration Tests` runs with `--run-ignored all` and succeeds on `main`.
- A new E2E workflow runs on:
  - `pull_request`, `merge_group`, `workflow_dispatch`
- E2E job starts Postgres + Qdrant via GitHub Actions services and successfully runs `cargo make e2e` without external secrets.
- Both workflows use `paths-ignore` for docs-only changes (`docs/**`, `**/*.md`, `.gitignore`).
- Local docs reflect the updated meaning of “E2E harness” vs “nightly harness signals”.

