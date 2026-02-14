# Rust Development and Style Guide

These rules apply to Rust code and Rust development workflows in this repository.

## Scope

These rules apply to Rust crates, binaries, and tooling in this repository.
They do not apply to non-Rust projects.
All rules in this guide are mandatory.

## Tooling and Workflow

- The Rust toolchain is pinned. Do not modify `rust-toolchain.toml`, `.cargo/config.toml`, or `.rustfmt.toml`.
- Do not install, update, or override toolchains.
- Do not invoke system package managers.
- Use `cargo make` tasks when they are a good fit for formatting, linting, and testing.

## Runtime Safety

- Do not use `unwrap()` in non-test code.
- `expect()` requires a clear, user-actionable message.

## Time and TLS

- Use the `time` crate for all date and time types. Do not add `chrono`.
- Use rustls for TLS. Use native-tls only when rustls is not supported.

## Formatting and Layout

- `rustfmt` output is the final authority for formatting.
- Use tabs (`\t`) for indentation.
- Use a flat module structure. Do not create or keep `mod.rs`.
- If `mod.rs` exists, flatten it into `a.rs` and `a/xxx.rs` style files.

## Error Handling

- Use `color_eyre::eyre::Result` for fallible APIs. Do not introduce `anyhow`.
- Add context at crate or module boundaries and keep the original error as the source.
- Boundaries include public APIs, entrypoints, and module-level helpers that are consumed outside the module.
- Use `#[error(transparent)]` only for thin wrappers where this crate adds no context and the upstream message is already sufficient for developers.
- Use short, action-oriented error messages that include the source error.
- Use `ok_or_else` to convert `Option` to `Result` with context.

## Logging

- Use fully qualified tracing macros, such as `tracing::info!`.
- Do not import tracing macros.
- Always use structured fields for dynamic values such as identifiers, names, counts, and errors.
- Use short, action-oriented messages as complete sentences.

## Borrowing and Ownership

- Use borrowing with `&` over `.as_*()` conversions when both are applicable.
- Avoid `.clone()` unless it is required by ownership or lifetimes, or it clearly improves clarity.
- Use `into_iter()` when intentionally consuming collections.
- Do not use scope blocks solely to end a borrow.
- When an early release is required, use an explicit `drop`.
- When the value is a reference and you need to end a borrow without a drop warning, use `let _ = value;`.
