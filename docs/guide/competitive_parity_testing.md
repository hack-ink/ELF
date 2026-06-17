# Competitive Parity Testing

Goal: Run the Docker-only parity gate that decides whether ELF has enough evidence to be considered against external memory systems.
Read this when: You need to prove ELF meets the minimum adoption bar instead of relying on architecture claims.
Preconditions: Docker and Docker Compose are available on the host.
Depends on: `docs/spec/system_competitive_parity_gate_v1.md`, `docs/guide/research/agentmemory_adapter.md`, and `Makefile.toml`.
Verification: `cargo make parity-docker` exits successfully and writes `tmp/parity/competitive-parity-report.json` with `verdict = "pass"`.

## Run

Start the gate from the repository root:

```sh
cargo make parity-docker
```

This command invokes Docker Compose on the host. The actual adapter check,
service-backed ELF run, Postgres database, Qdrant vector store, Cargo registry cache,
and Rust build target all run inside Docker-managed containers or volumes.

The report is written to:

```text
tmp/parity/competitive-parity-report.json
```

## Clean Up

Remove parity containers and Docker-managed volumes:

```sh
cargo make clean-parity-docker
```

The cleanup command removes Postgres, Qdrant, Cargo cache, and Rust target volumes
for the parity environment. It does not remove the host report directory under
`tmp/parity/`.

## Current Gate Coverage

The checked-in gate currently proves this minimum set:

- the agentmemory fixture adapter maps the sanitized sample into 2 note candidates,
  2 doc candidates, 1 baseline query, and 1 explicit ignored item;
- note candidate source references keep the agentmemory fixture resolver and origin
  identifiers;
- unsupported agentmemory memory kinds are rejected with the preserved reason
  `unsupported_memory_kind`;
- ELF can run a Postgres/Qdrant-backed retrieval and consolidation harness in Docker;
- consolidation preserves or improves recall while keeping retrieved context size no
  larger than the baseline run;
- the local admin viewer route returns 200 during the Docker service run.

This is not enough for personal production adoption by itself. It is the required
floor that prevents subjective comparisons from being mistaken for evidence.

## Production Adoption Expansion

Before using ELF as personal production memory infrastructure, extend the same gate
with private data and live baselines:

1. Build a sanitized private fixture pack from real personal coding-agent memory
   cases. Keep the source fixture out of the repository unless it has been reviewed
   for secrets and sensitive content.
2. Run the adapter/import/retrieval path against that private fixture pack inside
   Docker.
3. Add at least one live containerized external baseline, starting with agentmemory,
   against the same retrieval cases.
4. Keep the acceptance decision strict: ELF is not adopted if it loses on retrieval
   quality, migration fidelity, operator inspectability, or failure recovery without
   a documented compensating advantage.

## Failure Handling

When `cargo make parity-docker` fails:

- keep `tmp/parity/competitive-parity-report.json` if it was written;
- inspect `tmp/parity/consolidation-harness.log` for service-backed failures;
- fix the failing gate dimension before expanding to broader baselines;
- do not lower thresholds to make a comparison pass.
