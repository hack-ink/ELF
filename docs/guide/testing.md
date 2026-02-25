# Test Names and Scope

Purpose: Provide consistent names for test categories and the commands that run them.

## Names

- `unit` — Tests inside `#[cfg(test)]` modules in `src/`. Run with `cargo make test`.
- `integration` — Rust integration tests under `tests/*.rs`. Run with `cargo make test`.
- `integration (ignored)` — Integration tests that require external services and are marked `#[ignore]`.
- `acceptance` — The integration suite in `packages/elf-service/tests/acceptance.rs` and `packages/elf-service/tests/acceptance/*.rs`. These are usually `#[ignore]` and require external services.
- `E2E harness` — Deterministic harness scripts for memory retrieval/ranking. Run locally with `cargo make e2e` and in CI via `.github/workflows/e2e.yml`.

Note: Some integration tests require external services such as Postgres or Qdrant and are marked `#[ignore]`. When requesting those, say "integration (ignored)" so the ignored set is included.

## Database names

- `elf_e2e` — Dedicated database for the E2E flow.
- `elf_test_*` — Ephemeral databases created by `elf_testkit::TestDatabase` for integration tests.

## Usage

When requesting tests, refer to the names above. Example: "Run unit and integration tests," "Run integration (ignored) tests," or "Run the E2E flow."
