# Python Development and Style Guide

These rules apply to Python code and Python development workflows in this repository.

## Scope

These rules apply to Python services, libraries, and tooling in this repository, including `apps/entityphrase` and `packages/python`.
Do not apply them to non-Python projects.

## Tooling and Workflow

- Use the shared workspace virtual environment at the repository root.
- Activate the shared environment before running Poetry commands.
- Do not create per-project virtual environments or override `apps/entityphrase/poetry.toml`.

Setup:

1. From the repository root, create the shared environment: `python -m venv .venv`.
2. Activate the environment for your shell.
3. From `apps/entityphrase`, run `poetry sync --with dev`.

## Checks

Use `cargo make` tasks from the repository root when checks are required.

- `cargo make lint-python`
- `cargo make typecheck-python`
- `cargo make fmt-python`
- `cargo make test-python`

## Error Handling

- Do not swallow exceptions.
- Raise or return errors with clear, actionable context at module and service boundaries.
- Avoid broad exception catches unless the error is logged or re-raised with context.
