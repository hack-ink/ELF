# Project Cleanup Architecture Design

**Goal:** Restructure each app into a library-plus-binary layout, remove `#[path]` test imports, and make `cargo make lint` pass without suppressing lints.

**Scope (Option 2):**
- Apply the `lib + bin` layout to `elf-api`, `elf-mcp`, and `elf-worker`.
- Replace `#[path = "../src/..."]` test imports with library crate imports.
- Keep runtime behavior unchanged.
- Resolve clippy errors that remain after the refactor with minimal, local fixes.

**Out of Scope:**
- No API changes, no behavior changes, no renames or reorganizations beyond what is required for the refactor.
- No sweeping dependency upgrades or configuration redesigns.

**Architecture Summary:**
- Each app will expose its public surface in `src/lib.rs` and keep `src/main.rs` as a thin entrypoint.
- Tests will import the library crate directly, eliminating `#[path]` usage and clippy `dead_code` issues.
- Helper functions that exist only for testing will be placed in `#[cfg(test)]` modules in the library.
- Any remaining clippy errors will be fixed by small structural adjustments rather than `#[allow]` attributes.

**Testing and Verification:**
- Run `cargo make lint` to confirm workspace linting passes.
- Do not change test behavior; only update import paths and shared wiring required by the new layout.
