# Dependency Upgrade Workflow

This guide standardizes how to upgrade Rust dependencies while keeping version requirements consistent and low-risk.

## Version format policy

- Use `major.minor` in version requirements when possible.
- Avoid patch pins unless a specific patch is required for correctness or security.
- For `0.x` dependencies, prefer minor-capped ranges to avoid overly broad upgrades.
- In `Cargo.toml`, normalize dependency entries to inline table form with an explicit `version` key, even when no features are required.
- Do not edit lockfiles by hand. Regenerate them with the appropriate tool.

Exception: If a minimum patch is required, document the reason and use an explicit range such as `>=X.Y.Z,<X.(Y+1)`.

## Rust (Cargo)

1. Normalize dependency entries to inline table form with an explicit `version` key.
2. Keep dependency requirements in `Cargo.toml` at `major.minor` unless a patch pin is required.
3. Run `cargo update -w` from the repository root to refresh `Cargo.lock`.

## Verification

- Run `cargo make test` or targeted Rust tests when Rust dependencies change.
