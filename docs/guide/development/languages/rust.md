# Rust Development and LLM-Friendly Style Guide

This guide defines the Rust rules for this repository. It is optimized for LLM readability, deterministic diffs, and safe execution. All comments and messages must also follow the Global Language Rules in `AGENTS.md`.

## Scope

These rules apply to Rust crates, binaries, and tooling in this repository. They do not apply to non-Rust projects.

All rules in this guide are mandatory. There is no distinction between required and preferred rules.

## Decision Priorities

Use this priority order when trade-offs appear:

1. Correctness and safety.
2. Deterministic behavior and reproducibility.
3. LLM readability and auditability.
4. Simplicity of implementation.
5. Performance.

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
- Prefer rustls for TLS. Only use native-tls when rustls is not supported.

## Formatting and Layout

- `rustfmt` output is the final authority for formatting.
- Use tabs (`\t`) for indentation.

### Module Item Order

At module scope, order items as follows:

```
mod
use
macro_rules!
type
const
static
trait
enum
struct
impl
fn
```

Additional rules:

- Within each group, place `pub` items before non-`pub` items.
- Within the `fn` group at the same visibility, place non-`async` functions before `async` functions.
- Tests must be declared last, after all other items.
- Inside `#[cfg(test)] mod tests`, you must use `use super::*;`.

### File Structure

- Use a flat module structure. Do not create or keep `mod.rs`. If `mod.rs` exists, flatten it into `a.rs` and `a/xxx.rs` style files.

## Imports and Paths

Do not add import header comments.
Group imports by origin in this order: standard library, third-party crates, self or workspace crates.
Separate groups with a single blank line.

Rules:

- Do not import functions directly. Import the module or type and call `module::function(...)`.
- Calls to functions or macros must use a single module qualifier, such as `parent::function(...)` or `parent::macro!(...)`, unless the function or macro is defined in the same file.
- Standard library macros must be used without a `std::` qualifier, such as `vec!`, `format!`, or `println!`.
- If `crate::prelude::*` is imported, do not add redundant imports.
- In tests, prefer `use super::*;` for ergonomic access to the module under test.

## Types and `impl` Blocks

- Use `Self` instead of the concrete type name in `impl` method signatures.
- Keep `impl` blocks for a type contiguous in the `impl` section.
- Order `impl` blocks as: inherent, standard library traits, third-party traits, project traits.

## Generics and Trait Bounds

- All trait bounds must be in a `where` clause.
- Inline trait bounds are not allowed.
- You may use `impl Trait` in parameters or return positions.

## Error Handling

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

## Numeric Literals

- Separate numeric literal suffixes with a single underscore, for example `10_f32`.
- Insert underscores every three digits for integers with more than three digits, for example `1_000_000`.

## Readability Rules

- Keep one logical operation per line.
- Prefer functions at or under 100 lines. Extract helpers when a function exceeds 120 lines or the happy path is no longer obvious.
- Limit nesting depth to two levels. Extract helpers if deeper nesting appears.
- Prefer guard clauses and early returns to keep the happy path linear.
- Avoid complex `if let` or `match` guards. Extract a named boolean when logic grows.
- Use descriptive names and avoid single-letter locals except for trivial indices like `i`.
- Prefer explicit type annotations when inference spans multiple steps or reduces clarity.
- Prefer struct literals with named fields over `Default::default()` when fields matter.
- Avoid struct update syntax (`..`) unless the remaining fields are truly irrelevant.
- Keep boolean expressions short; extract them into named variables when they grow.
- Prefer type annotations on `let` bindings or function signatures. Use turbofish only when those locations cannot express the type.

## Functional Style

Functional style is allowed when it stays simple and readable.

- Limit iterator chains to at most three method calls after the base expression.
- Closures must be single-expression and side-effect free.
- If a closure needs `if`, `match`, or multiple statements, extract a named function.
- Avoid chaining `flat_map`, `filter_map`, `zip`, and `fold` in a single pipeline.
- Use `for` loops when you need multiple mutable state variables, `break`, or `continue`.

Example (use):

```rust
let filtered: Vec<_> = items.iter().filter(|item| item.is_valid()).collect();
let mapped: Vec<_> = filtered.into_iter().map(build_item).collect();
```

Example (avoid):

```rust
let result: Vec<_> = items
	.iter()
	.filter(|item| item.is_valid())
	.map(|item| build_item(item))
	.filter(|item| item.score > threshold)
	.collect();
```

## Borrowing and Ownership

- Prefer borrowing with `&` over `.as_*()` conversions when both are applicable.
- Avoid `.clone()` unless it is required by ownership or lifetimes, or it clearly improves clarity.
- Use `into_iter()` when intentionally consuming collections.
- Do not use scope blocks solely to end a borrow.
- When an early release is required, use an explicit `drop`.
- When the value is a reference and you need to end a borrow without a drop warning, use `let _ = value;`.

## Vertical Spacing

Inside Rust functions:

- Do not insert blank lines within the same statement type.
- Insert one blank line between different statement types.
- Insert exactly one blank line before the final return or tail expression, unless the body is a single expression.

Treat statements as the same type when they share the same syntactic form or call target, such as:

- Multiple `let` statements.
- Multiple `let mut` statements.
- Multiple `if` statements.
- Multiple `if let` statements.
- Multiple `match` statements.
- Multiple `for` loops.
- Multiple `while` loops.
- Multiple `loop` loops.
- Multiple calls to the same macro name (for example, `println!` with `println!`, or `tracing::...` with `tracing::...`).
- Multiple `Type::function(...)` calls.
- Multiple `self.method(...)` calls.
- Multiple assignment statements like `a = b`.

Additional rules.

- Treat `let` and `let mut` as different statement types.
- Different macro names are different statement types.
- When both appear together, place `let` statements before `let mut` statements.

## Comments and Documentation

- Comments must be full sentences with proper punctuation.
- Use comments only when intent is not clear from names and types.
- Public items should have doc comments when the intent is not obvious.

## Tests

- Use descriptive test names in `snake_case` that encode the behavior and expected outcome.
- Tests must be deterministic to keep LLM reasoning and CI outcomes stable.
- Integration tests that require external services must be marked `#[ignore]` with a clear message about required dependencies.

## LLM Readability Checklist

Before finalizing a Rust change, ensure the following:

- Functions are short, flat, and linear.
- Iterator chains are short and clear.
- Error boundaries are explicit.
- Logging uses structured fields.
- Names convey intent without relying on comments.
