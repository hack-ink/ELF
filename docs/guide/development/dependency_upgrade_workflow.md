# Dependency Upgrade Workflow

This repository uses a Rust-only dependency stack for active package management.

## Version format policy

- Use `major.minor` in version requirements when possible.
- Avoid patch pins unless a specific patch is required for correctness or security.
- For `0.x` dependencies, prefer minor-capped ranges to avoid overly broad upgrades.
- In the root `Cargo.toml`, normalize workspace dependency entries to inline table form with an explicit `version` key, even when no features are required.
- In workspace member `Cargo.toml` files, use `workspace = true` for dependencies and do not use `version` or `path` keys.
- In `Cargo.toml`, group dependency entries by origin and separate groups with a single blank line.
- Do not edit lockfiles by hand. Regenerate them with the appropriate tool.

Exception: If a minimum patch is required, document the reason and use an explicit range such as `>=X.Y.Z,<X.(Y+1)`.

## Scope and order of operations

For this repository, only Rust manifests are in use for dependency upgrades.
Sections on JavaScript/Python tooling are intentionally not applicable.

1. Update dependency constraints in manifests first.
2. Regenerate lockfiles second.
3. Verify with tests and lint checks.
4. Check remaining Dependabot PRs and classify each one as:
   - already covered by the upgrade commit,
   - blocked by compatibility constraints,
   - intentionally deferred (must be documented).

## Rust (Cargo)

1. In the root `Cargo.toml`, normalize workspace dependency entries to inline table form with an explicit `version` key.
2. In workspace member `Cargo.toml` files, use `workspace = true` for dependencies and do not use `version` or `path` keys.
3. Keep dependency requirements in the root `Cargo.toml` at `major.minor` unless a patch pin is required.
4. Run `cargo update -w` from the repository root to refresh `Cargo.lock`.
5. When updates do not apply, run `cargo update -w --verbose` and record whether Rust toolchain compatibility is the blocker.

## Verification

- Run `cargo make test` when Rust dependencies change.
- Run `cargo make test` or targeted Rust tests when dependency behavior changes are suspected.
- Run `gh pr list --state open --search "author:app/dependabot"` and reconcile each remaining PR.

## Dependabot alignment

- If a dependency is intentionally capped (especially `0.x` minor caps), Dependabot will still propose newer minors.
- To avoid repeated noise, either:
  1. upgrade the capped dependency line, or
  2. add an explicit ignore rule in `.github/dependabot.yml` with a documented reason.
- Do not assume lockfile-only updates will close manifest bump PRs. Dependabot compares manifests as well as lockfiles.
