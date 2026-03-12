# Guide Index

Goal: Route agents to procedural documents that tell them how to execute work safely and
repeatably.
Read this when: You know the question is operational and need the best execution path.
Inputs: The current task shape, subsystem, and whether you need background research.
Depends on: `docs/index.md` and `docs/governance.md`.
Outputs: The smallest guide or guide subfolder needed to continue execution.

Question this index answers: "what should I do?"

## Use this index when

- You need a runbook, how-to, migration sequence, validation flow, troubleshooting
  path, or maintenance procedure.
- You already know the relevant spec and need the operational steps.
- You need a bounded sequence with prerequisites and verification.
- You need external comparisons or research notes that inform an implementation choice.

## Do not use this index when

- You need the authoritative contract, schema, or invariant.
- You need a planning-tool artifact or a saved execution plan under `docs/plans/`.
- You need broad documentation policy or repo task-entrypoint rules; read
  `docs/governance.md` or `Makefile.toml` instead.

## What belongs in `docs/guide/`

- Task-oriented runbooks.
- Validation and test procedures.
- Migration, rollout, rollback, and recovery sequences.
- Troubleshooting flows and operator checklists.
- Short implementation recipes that depend on a governing spec.
- Decision-support research and external comparisons that inform implementation choices.

## Guide document contract

Start each guide with a compact routing header:

- `Goal`
- `Read this when`
- `Inputs` or `Preconditions`
- `Depends on`
- `Outputs` or `Verification`

Then structure the body for execution:

- Write steps in the order an agent should perform them.
- Keep commands, checks, and rollback points explicit.
- Link to specs for normative truth instead of restating contracts.
- Include failure branches only when they change the next action.
- End with verification so an agent can tell whether the guide succeeded.

## Structure policy

- Group guides by workflow or subsystem only when multiple guides exist and the grouping
  improves retrieval.
- Do not create empty category folders or placeholder section headings.
- Prefer titles that encode the task or outcome, such as `validate_release.md` or
  `rerun_ingest_job.md`.
- Keep the guide index as a router, not a dumping ground for long explanations.

## Guide subfolders

- `docs/guide/development/` for repository-development workflows.
- `docs/guide/research/` for external comparisons and decision-support materials that are
  non-normative.
