# CLI Alignment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Share CLI style and version formatting across all app binaries, align build metadata injection, and remove the placeholder root CLI.

**Architecture:** Create a small shared crate `crates/elf-cli` that exposes `styles()` and a `VERSION` constant. Each app’s CLI uses these shared items via `#[command(...)]`. Reuse the workspace root `build.rs` for each app to inject `VERGEN_*` environment variables. Remove the root `src/` placeholder once apps are aligned.

**Tech Stack:** Rust 2024, `clap`, `vergen-gitcl`, `color-eyre`.

---

### Task 1: Add shared CLI crate

**Files:**
- Create: `crates/elf-cli/Cargo.toml`
- Create: `crates/elf-cli/src/lib.rs`
- Modify: `Cargo.toml`

**Step 1: Write minimal crate manifest**
Create `crates/elf-cli/Cargo.toml` with a library target, edition 2024, and `clap` dependency from workspace. Add `build = "../../build.rs"` and `[build-dependencies] vergen-gitcl = { workspace = true }` so `VERSION` can use `VERGEN_*` at compile time.

**Step 2: Implement shared API**
Create `crates/elf-cli/src/lib.rs` exporting:
- `pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_SHA"), "-", env!("VERGEN_CARGO_TARGET_TRIPLE"));`
- `pub fn styles() -> clap::builder::Styles` using the same styling as the previous root CLI.

**Step 3: Add crate to workspace**
Add `crates/elf-cli` to `[workspace].members` in `Cargo.toml`.

---

### Task 2: Align app CLIs to shared styles/version

**Files:**
- Modify: `apps/elf-api/src/main.rs`
- Modify: `apps/elf-worker/src/main.rs`
- Modify: `apps/elf-mcp/src/main.rs`
- Modify: `apps/elf-api/Cargo.toml`
- Modify: `apps/elf-worker/Cargo.toml`
- Modify: `apps/elf-mcp/Cargo.toml`

**Step 1: Add shared dependency**
Add `elf-cli = { path = "../../crates/elf-cli" }` to each app’s dependencies.

**Step 2: Apply clap attributes**
Update each `Args` struct to include:
```
#[command(
    version = elf_cli::VERSION,
    rename_all = "kebab",
    styles = elf_cli::styles(),
)]
```
Keep existing arguments unchanged.

---

### Task 3: Align build metadata injection

**Files:**
- Modify: `Cargo.toml`
- Modify: `apps/elf-api/Cargo.toml`
- Modify: `apps/elf-worker/Cargo.toml`
- Modify: `apps/elf-mcp/Cargo.toml`

**Step 1: Add workspace build dependency**
Add `vergen-gitcl = "10.0.0-beta.5"` to `[workspace.dependencies]`.

**Step 2: Reuse root build script in each app**
Add `build = "../../build.rs"` under `[package]` for each app.

**Step 3: Add build-dependencies**
Add `[build-dependencies] vergen-gitcl = { workspace = true }` to each app.

---

### Task 4: Remove placeholder root CLI

**Files:**
- Delete: `src/main.rs`
- Delete: `src/cli.rs`

**Step 1: Delete files**
Remove the root `src/` placeholder CLI since all apps have their own CLIs.

---

### Task 5: Minimal verification

**Commands:**
- `cargo run -p elf-api -- --help`
- `cargo run -p elf-worker -- --help`
- `cargo run -p elf-mcp -- --help`

**Expected:**
- CLI help renders with shared styling.
- `--version` is present and uses `CARGO_PKG_VERSION-VERGEN_GIT_SHA-VERGEN_CARGO_TARGET_TRIPLE` format.
- No file logging is introduced; console output remains default.
