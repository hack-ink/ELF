# AGENTS.md — Repository-Specific Rules for Automated Agents

These instructions define repository-specific execution rules and scope limits for this repository.

---

## 1. Execution Model

When a data debugging method is not specified, use `psql` with the `.env`-provided `PUBFI_DATABASE_URL` for the `pubfi_core` database.

## 1.1 Workspace Automation (cargo make)

- `Makefile.toml` is the source of truth for task names and behavior.
- Run `cargo make` from the repository root, and use it whenever an equivalent task exists.
- Run standalone commands only when `Makefile.toml` does not cover the capability or cannot produce the required effect for the current task.
- When task details are needed, inspect `Makefile.toml` directly or run `cargo make --list-all-steps`.

---

## 2. Language-Specific Rules Reference

Rust development rules live in `docs/guide/development/languages/rust.md`.
Python development rules live in `docs/guide/development/languages/python.md`.
