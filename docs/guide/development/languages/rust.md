# Rust Development and LLM-Friendly Style Guide

This guide defines the Rust rules for this repository. It is optimized for LLM readability, deterministic diffs, and safe execution. All comments and messages must also follow the Global Language Rules in `AGENTS.md`.

## Scope

These rules apply to Rust crates, binaries, and tooling in this repository. They do not apply to non-Rust projects.

All rules in this guide are mandatory.

## Agent Checklist

Before you start a Rust change:

- Identify which sections apply (Imports and Paths, Error Handling, Logging, Functional Style, Vertical Spacing).
- Ensure your change can follow the Completion Checklist tasks.

Before you claim a Rust change is complete:

- Follow the Completion Checklist section.
- Ensure errors use `color_eyre::eyre::Result` and add boundary context with `WrapErr`.
- Ensure logs use `tracing::...!` with structured fields.
- Ensure function bodies follow the Vertical Spacing phases and declaration ordering rules.

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
- Use rustls for TLS. Use native-tls only when rustls is not supported.

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
- For extension traits (for example, traits named `FooExt`), place the trait definition immediately followed by its `impl` blocks.
- Keep `impl` blocks adjacent to their type definitions. See Types and `impl` Blocks.
- Tests must be declared last, after all other items.
- Inside `#[cfg(test)] mod tests`, use `use super::*;` unless the module exists only to mark dev-dependencies as used (for example, `#[cfg(test)] mod _test` with `use some_crate as _;`).

Editing checklist:

1. Ensure the top-level groups match the required order (mod, use, macro_rules!, type, const, static, trait, enum, struct, impl, fn).
2. Keep a type definition immediately followed by its `impl` blocks.
3. Keep `#[cfg(test)] mod tests` as the last item in the module.

### File Structure

- Use a flat module structure. Do not create or keep `mod.rs`. If `mod.rs` exists, flatten it into `a.rs` and `a/xxx.rs` style files.

## Imports and Paths

Group imports by origin in this order: standard library, third-party crates, self or workspace crates.
Treat workspace member crates as part of the self/workspace group, alongside `crate::` and `super::` paths.
Separate groups with a blank line and do not add header comments for import groups.

Editing checklist:

1. Group imports by origin (standard library, third-party crates, self or workspace crates).
2. Do not alias imports (except `use some_crate as _;` in `#[cfg(test)] mod _test`).
3. Import modules and types, not free functions or macros. For non-local calls, use qualified paths like `module::function(...)` and `module::macro!(...)`.
4. In `error.rs`, do not add `use` imports and use fully qualified paths.

Rules:

- Do not alias imports with `use ... as ...`. The only exception is `use some_crate as _;` inside `#[cfg(test)] mod _test` to mark dev-dependencies as used for `unused_crate_dependencies` and similar lints.
- When name conflicts exist, use a more qualified path at the usage site instead of aliasing.
- Do not import free functions or macros into scope with `use`.
- Calls to free functions and macros defined outside the current module must use a path qualifier, such as `parent::function(...)`, `Type::function(...)`, or `parent::macro!(...)`.
- Method calls like `value.method(...)` are allowed.
- You may re-export functions with `pub use` when you need them in a crate's public API, for example `pub use crate::module::function;`.
- You may use `use super::*;` only when the parent module is intentionally designed as a module prelude.
- In files named `error.rs`, do not add `use` imports. Use fully qualified paths at call and type sites.
- Standard library macros must be used without a `std::` qualifier, such as `vec!`, `format!`, or `println!`.
- If `crate::prelude::*` is imported, do not add redundant imports.
- Do not rely on `crate::prelude::*` to bring free functions or macros into scope. Use qualified paths for those call sites.

Example (use):

```rust
use crate::worker;

pub fn run_worker() {
	let _ = worker::run();
}
```

Example (avoid):

```rust
use crate::worker::run;

pub fn run_worker() {
	let _ = run();
}
```

## Types and `impl` Blocks

- Use `Self` instead of the concrete type name in `impl` method signatures.
- `impl` blocks for a type must be placed immediately after the type definition with no blank line between them.
- Keep all `impl` blocks for a type contiguous and grouped immediately after the type definition.
- Order `impl` blocks as: inherent, standard library traits, third-party traits, project traits.

## Generics and Trait Bounds

- All trait bounds must be in a `where` clause.
- Inline trait bounds are not allowed.
- You may use `impl Trait` in parameters or return positions.

## Error Handling

- Use `color_eyre::eyre::Result` for fallible APIs. Do not introduce `anyhow`.
- Add context at crate or module boundaries and keep the original error as the source.
- Boundaries include public APIs, entrypoints, and module-level helpers that are consumed outside the module.
- Use `#[error(transparent)]` only for thin wrappers where this crate adds no context and the upstream message is already sufficient for developers.
- Use short, action-oriented error messages that include the source error.
- Use `ok_or_else` to convert `Option` to `Result` with context.

Example (use):

```rust
use color_eyre::eyre::WrapErr;

fn load_config(path: &std::path::Path) -> color_eyre::eyre::Result<Config> {
	let bytes = std::fs::read(path)
		.wrap_err_with(|| format!("Failed to read config file at {path:?}."))?;

	parse_config(&bytes).wrap_err("Failed to parse config file.")
}
```

Example (avoid):

```rust
fn load_config(path: &std::path::Path) -> color_eyre::eyre::Result<Config> {
	let bytes = std::fs::read(path)?;

	parse_config(&bytes)
}
```

## Logging

- Use fully qualified tracing macros, such as `tracing::info!`.
- Do not import tracing macros.
- Always use structured fields for dynamic values such as identifiers, names, counts, and errors.
- Use short, action-oriented messages as complete sentences.

Example (use):

```rust
tracing::info!(user_id = %user_id, "Created session.");
```

Example (avoid):

```rust
tracing::info!("Created session for user {user_id}.");
```

## Numeric Literals

- Separate numeric literal suffixes with a single underscore, for example `10_f32`.
- Insert underscores every three digits for integers with more than three digits, for example `1_000_000`.

## Readability Rules

In this section, the happy path is the main success flow and excludes error-handling branches.

- Keep one logical operation per line.
- Keep functions at or under 120 lines. Extract helpers when a function exceeds 120 lines or the happy path is no longer obvious.
- Do not introduce a new helper function when the code is a single expression and the helper is used only once. Inline it at the call site unless the helper name encodes a meaningful domain concept or isolates non-trivial logic.
- Limit control-flow nesting depth to two levels in the happy path. Count one level for each `if`/`if let`/`match`/loop that contains other control flow.
- When nesting exceeds two levels, reduce it using one or more of: guard clauses and early returns to invert conditions, extracting an inner block into a helper that returns `Result` or `Option`, or using `continue` to skip work in loops instead of wrapping the rest of the loop body.
- Use guard clauses and early returns to keep the happy path linear.
- Avoid complex `if let` or `match` guards. Extract a named boolean when logic grows.
- Use descriptive names and avoid single-letter locals except for trivial indices like `i`.
- Add explicit type annotations when inference spans multiple steps or reduces clarity.
- Use struct literals with named fields over `Default::default()` when fields matter.
- Avoid struct update syntax (`..`) unless the remaining fields are truly irrelevant.
- Keep boolean expressions short; extract them into named variables when they grow.
- When you need to specify a type explicitly, do so on `let` bindings or in function signatures. Use turbofish only when those locations cannot express the type.

Example (use):

```rust
for item in items {
	if !item.is_ready() {
		continue;
	}

	let parsed = parse(item.value())?;

	if parsed.is_empty() {
		return Err(color_eyre::eyre::eyre!("Parsed item must not be empty."));
	}

	process(&parsed)?;
}
```

Example (avoid):

```rust
for item in items {
	if item.is_ready() {
		let parsed = parse(item.value())?;
		if !parsed.is_empty() {
			process(&parsed)?;
		} else {
			return Err(color_eyre::eyre::eyre!("Parsed item must not be empty."));
		}
	}
}
```

## Functional Style

Default to functional style for collection transformations and queries.

- Iterator chains have no fixed maximum length.
- Do not split a pipeline solely because of its length.
- Closures must be single-expression and side-effect free.
- If a closure needs `if`, `match`, or multiple statements, extract a named function.
- Avoid combining `flat_map`, `zip`, and `fold`/`reduce` in a single iterator pipeline. Split the pipeline into named steps or a `for` loop.
- Do not use `.for_each(...)` for side effects. Use a `for` loop.
- Use `for` loops when iterator-based code would require complex control flow (`break` or `continue`), multiple mutable state variables, or multi-statement closures.

Example (use):

```rust
let result: Vec<_> = items
	.iter()
	.filter(|item| item.is_valid())
	.map(|item| build_item(item))
	.filter(|item| item.score > threshold)
	.collect();
```

Example (avoid):

```rust
let total: i64 = items
	.iter()
	.flat_map(|item| item.children())
	.zip(weights.iter())
	.map(|(child, weight)| score(child) * weight)
	.filter(|score| *score > threshold)
	.take(limit)
	.fold(0_i64, |acc, score| acc + score);
```

## Borrowing and Ownership

- Use borrowing with `&` over `.as_*()` conversions when both are applicable.
- Avoid `.clone()` unless it is required by ownership or lifetimes, or it clearly improves clarity.
- Use `into_iter()` when intentionally consuming collections.
- Do not use scope blocks solely to end a borrow.
- When an early release is required, use an explicit `drop`.
- When the value is a reference and you need to end a borrow without a drop warning, use `let _ = value;`.

## Vertical Spacing

This section exists because `rustfmt` does not enforce blank-line layout inside function bodies, and inconsistent spacing makes diffs hard to audit.

### Function Bodies

Rules:

- Use blank lines only to separate phases. Do not use blank lines as decoration.
- Never use more than one consecutive blank line.
- Do not add a blank line immediately after `{` or immediately before `}`.
- Within a phase, do not insert blank lines.
- If a function body has multiple phases, insert exactly one blank line before the final `return ...;` statement or the tail expression.

Phases (in order):

1. **Declarations:** `let` and `let mut` bindings and simple derived values.
2. **Guards:** validations and early-exit checks (`if`, `if let`, `match`) that return, break, or continue.
3. **Work:** the main control flow and side effects (loops, I/O, calls that perform the primary action).
4. **Return:** the final `return ...;` or tail expression.

Additional rules:

- Order declarations by data dependencies. A binding must appear after any binding it reads.
- Within that constraint, place immutable bindings before mutable bindings.
- Keep related `tracing::...!` calls contiguous with no blank lines between them, and keep them adjacent to the operation they describe.

Example (use, dependency order):

```rust
let mut buffer = Vec::new();
read_into(&mut buffer)?;
let size = buffer.len();
```

Example (use):

```rust
pub fn handle(input: &str) -> color_eyre::eyre::Result<()> {
	let parsed = parse(input)?;
	let normalized = normalize(&parsed);
	let mut stats = Stats::default();

	if normalized.is_empty() {
		return Err(color_eyre::eyre::eyre!(
			"Input must not be empty after normalization."
		));
	}

	tracing::info!(len = normalized.len(), "Processing input.");
	process(&normalized, &mut stats)?;
	tracing::info!(?stats, "Processing completed.");

	Ok(())
}
```

Example (avoid):

```rust
pub fn handle(input: &str) -> color_eyre::eyre::Result<()> {

	let parsed = parse(input)?;

	let normalized = normalize(&parsed);

	let mut stats = Stats::default();
	if normalized.is_empty() {
		return Err(color_eyre::eyre::eyre!(
			"Input must not be empty after normalization."
		));
	}

	tracing::info!(len = normalized.len(), "Processing input.");

	process(&normalized, &mut stats)?;

	tracing::info!(?stats, "Processing completed.");

	Ok(())
}
```

### Editing Checklist

When you edit a function body, apply this sequence:

1. Remove any decorative blank lines and collapse multiple blank lines to a single blank line.
2. Re-group the body into the phases above.
3. Ensure the final `return` or tail expression has exactly one blank line before it (unless the body is a single expression).

## Comments and Documentation

- Comments must be full sentences with proper punctuation.
- Use comments only when intent is not clear from names and types.
- Public items should have doc comments when the intent is not obvious.

## Tests

- Use descriptive test names in `snake_case` that encode the behavior and expected outcome.
- Tests must be deterministic to keep LLM reasoning and CI outcomes stable.
- Integration tests that require external services must be marked `#[ignore]` with a clear message about required dependencies.
- `#[cfg(test)] mod _test` is reserved for dev-dependency keep-alive imports such as `use some_crate as _;`. Do not place behavior tests in `_test`.

## LLM Readability Checklist

Before finalizing a Rust change, ensure the following:

- Functions follow the Readability Rules section.
- Iterator pipelines follow the Functional Style section.
- Error boundaries are explicit.
- Logging uses structured fields.
- Names convey intent without relying on comments.
- Imports and call sites follow the rules in the Imports and Paths section.

## Completion Checklist

When you claim a Rust change is complete, run the following tasks:

1. `cargo make fmt-rust`
2. `cargo make lint-rust`
3. `cargo make test-rust` when the change affects behavior, not just formatting or comments.
