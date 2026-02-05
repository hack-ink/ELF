# Dependency Upgrade Workflow

This guide standardizes how to upgrade Rust dependencies while keeping version requirements consistent and low-risk.

## Version format policy

- Use `major.minor` in version requirements when possible.
- Avoid patch pins unless a specific patch is required for correctness or security.
- For `0.x` dependencies, prefer minor-capped ranges to avoid overly broad upgrades.
- In the root `Cargo.toml`, normalize workspace dependency entries to inline table form with an explicit `version` key, even when no features are required.
- In workspace member `Cargo.toml` files, use `workspace = true` for dependencies and do not use `version` or `path` keys.
- In `Cargo.toml`, group dependency entries by origin and separate groups with a single blank line.
- Do not edit lockfiles by hand. Regenerate them with the appropriate tool.

Exception: If a minimum patch is required, document the reason and use an explicit range such as `>=X.Y.Z,<X.(Y+1)`.

## Rust (Cargo)

1. In the root `Cargo.toml`, normalize workspace dependency entries to inline table form with an explicit `version` key.
2. In workspace member `Cargo.toml` files, use `workspace = true` for dependencies and do not use `version` or `path` keys.
3. Keep dependency requirements in the root `Cargo.toml` at `major.minor` unless a patch pin is required.
4. Run `cargo update -w` from the repository root to refresh `Cargo.lock`.

## Verification

- Run `cargo make test` or targeted Rust tests when Rust dependencies change.
