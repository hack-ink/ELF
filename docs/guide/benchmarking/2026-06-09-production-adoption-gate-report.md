# Production Adoption Gate Report - June 9, 2026

Goal: Record the XY-836 full comparison gate and personal production adoption decision.
Read this when: You need the fresh evidence behind the June 9, 2026 ELF production
adoption claim.
Inputs: P0 benchmark and runbook PRs, live Docker benchmark reports, provider-backed
benchmark runs, and the single-user restore proof.
Depends on: `live_baseline_benchmark.md`, `single_user_production.md`,
`comparison_external_projects.md`, `research_projects_inventory.md`, and
`Makefile.toml`.
Outputs: Production adoption verdict, exact benchmark commands, run ids, limitations,
and README-level claim boundaries.

## Decision

ELF is ready for personal production use with bounded caveats.

The gate supports use as a single-user, self-hosted memory service when operated through
the checked-in Docker Compose production runbook, with backups enabled, Qdrant treated as
rebuildable, and retrieval debugging done through search traces and viewer/admin trace
surfaces rather than raw SQL.

The caveats are material:

- No private production corpus manifest was available in this lane. The
  `baseline-production-private` task failed closed at its manifest guard, so this report
  does not claim a private-corpus pass.
- External comparison remains an objective adapter matrix, not an overall superiority
  claim. qmd and ELF passed the encoded smoke checks; agentmemory, memsearch, mem0,
  OpenViking, and claude-mem retained typed failures or incomplete states.
- The 2,000-document provider backfill passed but took 2,804 seconds end to end. Large
  imports should be planned as batch jobs, not interactive operations.

Because the private-corpus criterion allows an explicitly bounded result, this gate does
not create a new P0 blocker. If private-corpus proof is required before a specific
deployment, supply `ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST` and rerun
`cargo make baseline-production-private` before relying on private retrieval quality.

## P0 Inputs

The current branch is based on the post-observability mainline. The named P0 lanes were
merged before this gate:

| Issue | PR | Evidence read |
| --- | --- | --- |
| `XY-819` | `#126` | Single-user production backup and restore runbook. |
| `XY-818` | `#127` | Private production corpus benchmark task and manifest guard. |
| `XY-817` | `#128` | Resumable batch ingest and backfill benchmark. |
| `XY-820` | `#130` | Typed lifecycle and adapter failure states. |
| `XY-825` | `#131` | Additional single-user restore and Qdrant rebuild proof. |
| `XY-27` | `#132` | Retrieval observability panels and trace candidate precision repair. |

## Fresh Commands

Provider credentials were loaded from an untracked local environment file. Secret values
were not printed or committed. The command forms below assume equivalent provider
environment variables are present in the shell.

Private manifest guard:

```sh
cargo make baseline-production-private
```

Result: failed closed before the benchmark runner because
`ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST` was not set.

Production-synthetic provider run:

```sh
set -a
source .env
set +a
EMBEDDING_MODEL=Qwen3-Embedding-8B \
EMBEDDING_DIMENSIONS=4096 \
EMBEDDING_TIMEOUT_MS=30000 \
ELF_BASELINE_ELF_EMBEDDING_MODE=provider \
ELF_BASELINE_PROJECTS=ELF \
ELF_BASELINE_MAX_ELF_SECONDS=1200 \
cargo make baseline-production-synthetic
```

All-project smoke provider run:

```sh
set -a
source .env
set +a
EMBEDDING_MODEL=Qwen3-Embedding-8B \
EMBEDDING_DIMENSIONS=4096 \
EMBEDDING_TIMEOUT_MS=30000 \
ELF_BASELINE_ELF_EMBEDDING_MODE=provider \
ELF_BASELINE_PROFILE=smoke \
cargo make baseline-live-docker
```

ELF provider stress run:

```sh
set -a
source .env
set +a
EMBEDDING_MODEL=Qwen3-Embedding-8B \
EMBEDDING_DIMENSIONS=4096 \
EMBEDDING_TIMEOUT_MS=30000 \
ELF_BASELINE_PROJECTS=ELF \
ELF_BASELINE_PROFILE=stress \
ELF_BASELINE_MAX_ELF_SECONDS=1800 \
ELF_BASELINE_ELF_TIMEOUT_SECONDS=1800 \
ELF_BASELINE_ELF_EMBEDDING_MODE=provider \
cargo make baseline-live-docker
```

ELF provider backfill run:

```sh
set -a
source .env
set +a
EMBEDDING_MODEL=Qwen3-Embedding-8B \
EMBEDDING_DIMENSIONS=4096 \
EMBEDDING_TIMEOUT_MS=30000 \
ELF_BASELINE_ELF_EMBEDDING_MODE=provider \
ELF_BASELINE_ELF_TIMEOUT_SECONDS=3600 \
ELF_BASELINE_MAX_ELF_SECONDS=3600 \
cargo make baseline-backfill-docker
```

Single-user restore proof:

```sh
awk '/^bash <<'\''EOF'\''$/{flag=1; next} flag && /^EOF$/{exit} flag {print}' \
  docs/guide/single_user_production.md \
  | perl -0pe 's#tmp/single-user-restore-proof#tmp/xy836-single-user-restore-proof#g; s/51988/52988/g; s/51989/52989/g; s/51990/52990/g; s/51991/52991/g; s/51992/52992/g; s/51993/52993/g; s/elf-restore-proof/elf-xy836-restore-proof/g' \
  > tmp/xy836-restore-proof.sh
bash tmp/xy836-restore-proof.sh
```

The proof used alternate local ports because the default proof port range was occupied
on this machine.

## ELF Evidence

All provider-backed ELF runs used:

- Provider id: `provider`
- Embedding model: `Qwen3-Embedding-8B`
- Embedding dimensions: `4096`
- Timeout: `30000` ms
- API path: `/embeddings`

| Run | Profile | Corpus | Status | Checks | Retrieval | Elapsed | Query result | Backfill and resume |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `live-baseline-20260609083644` | `production-synthetic` | `synthetic-coding-agent-prod-corpus-2026-06-09`, 8 docs, 6 queries | `pass` | `8/8` | `retrieval_pass` | 59 s | 6/6 pass, mean 937.120 ms | 8/8 completed in 8.134 s, resume 4 -> 8, 0 duplicates |
| `live-baseline-20260609090719` | `stress` | generated public, 480 docs, 16 queries | `pass` | `9/9` | `retrieval_pass` | 779 s | 16/16 pass, mean 1128.144 ms | 480/480 completed in 508.835 s, resume 240 -> 480, 0 duplicates |
| `live-baseline-20260609092144` | `backfill` | generated public, 2000 docs, 16 queries | `pass` | `9/9` | `retrieval_pass` | 2804 s | 16/16 pass, mean 1214.454 ms | 2000/2000 completed in 2061.396 s, resume 1000 -> 2000, 0 duplicates |

The 2,000-document backfill also passed:

- `resumable_backfill_no_duplicates`
- `same_corpus_retrieval`
- `async_worker_indexing_e2e`
- `update_replaces_note_text`
- `delete_suppresses_retrieval`
- `cold_start_recovery_search`
- `concurrent_write_search_e2e`
- `soak_stability_e2e`
- `resource_envelope`

The resource envelope check measured 2,793.629 seconds against a 3,600-second limit and
167,652 KB RSS against a 1,500,000 KB limit.

## Recovery Evidence

The single-user production proof wrote a note, searched it, recreated the Docker
Compose dependency stack from backup, rebuilt Qdrant from Postgres-held vectors, and
searched again.

| Step | Evidence |
| --- | --- |
| Note ingest | `ADD`, `remember`, note id `bfaa2f40-e076-490e-ae5a-dd88cf6b6179` |
| Search before restore | 1 result, key `single_user_restore_probe`, trace `535e49be-250f-483c-8845-b4116e591dac`, score 1.148 |
| Qdrant rebuild after restore | `rebuilt_count=1`, `missing_vector_count=0`, `error_count=0` |
| Search after restore | 1 result, key `single_user_restore_probe`, trace `e995263d-8f0e-4472-9a32-354d5cceed33`, score 1.1479998 |

This satisfies the adoption criterion that Postgres backups, restore, and Qdrant rebuild
are tested without treating Qdrant as a source of truth.

## External Comparison

Fresh all-project smoke run: `live-baseline-20260609083814`.

Corpus: generated public smoke, 3 docs, 3 queries.

Aggregate verdict: `fail`, because the matrix is strict and external adapters retained
typed failures. The strict failure is useful evidence; it prevents hiding incomplete
adapter states.

Full encoded check summary: 26 total, 16 pass, 3 fail, 2 wrong-result, 1 lifecycle-fail,
2 incomplete, 1 blocked, 4 not encoded.

| Project | Status | Retrieval | Checks | Elapsed | Storage | Interpretation |
| --- | --- | --- | --- | --- | --- | --- |
| ELF | `pass` | `retrieval_pass` | `8/8` | 33 s | real | Added corpus, rebuilt Qdrant, returned expected evidence, and passed lifecycle checks. |
| qmd | `pass` | `retrieval_pass` | `4/4` | 59 s | real | Passed same-corpus retrieval, update, delete, and cold-start checks through persisted local collection files. |
| agentmemory | `lifecycle_fail` | `retrieval_pass` | `2/4` | 46 s | mocked | Same-corpus retrieval passed, but update left old text searchable and cold-start recovery is blocked by in-memory harness storage. |
| memsearch | `incomplete` | `invalid_json_result` | `0/1` | 432 s | real | Command completed but did not produce a valid benchmark result. |
| mem0 | `incomplete` | `invalid_json_result` | `2/4` | 462 s | real | Local FastEmbed/Qdrant search missed expected same-corpus results; delete remains not encoded. |
| OpenViking | `incomplete` | `local_embed_install_failed` | `0/1` | 513 s | incomplete | Local embedding install hit a llama-cpp-python build/import failure, so same-corpus local retrieval could not run. |
| claude-mem | `incomplete` | `invalid_json_result` | `0/4` | 107 s | mocked | Repository search missed expected same-corpus results and lifecycle behaviors remain mostly not encoded. |

## Observability Evidence

The gate is based on main after `XY-27`, which added read-only viewer retrieval
observability panels and a precision repair for trace candidate scores. The fresh
benchmark runs returned trace ids for every ELF search, and the search responses include
retrieval trajectory summaries.

Representative provider stress traces:

| Query | Trace id |
| --- | --- |
| `q-auth` | `7be1b5ce-3676-4625-8221-dcf0204669bf` |
| `q-auth-alt` | `79585c67-cdb8-46f8-bad1-d277295c1e0f` |
| `q-database` | `0cc7d130-fe51-436e-a5b0-971997ba8cb7` |
| `q-database-alt` | `4ffaf8cd-4b0d-4b3d-8154-56551538e81a` |
| `q-deploy` | `c770346e-d563-4ad0-aae6-f56dff334669` |
| `q-deploy-alt` | `84121528-c038-490b-bbc5-3352bcb9a2f5` |

Representative restore proof traces:

- Before restore: `535e49be-250f-483c-8845-b4116e591dac`
- After restore: `e995263d-8f0e-4472-9a32-354d5cceed33`

This is sufficient for the personal production gate: a wrong result can be debugged via
the returned trace id, trajectory stages, trace bundle/admin endpoints, and the viewer
panels without raw SQL.

## Adoption Criteria

| Criterion | Result | Evidence and limitation |
| --- | --- | --- |
| Private production corpus benchmark has a passing or explicitly bounded result. | Bounded caveat | `cargo make baseline-production-private` failed closed because `ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST` was unset. No private-corpus pass is claimed. |
| Backfill/resume proves predictable large import behavior. | Pass | `live-baseline-20260609092144`: 2000/2000 completed, resume 1000 -> 2000, zero duplicates, resource envelope passed. |
| Docker Compose backup, restore, and Qdrant rebuild are tested. | Pass | Single-user restore proof rebuilt 1 Qdrant point with 0 missing vectors and recovered searchable results. |
| Retrieval observability can debug wrong results without raw SQL. | Pass | `XY-27` landed, trace ids are returned in benchmark and restore runs, and trajectory summaries are present in search responses. |
| External comparison uses typed failure states and does not rely on mocked adapter results as proof. | Pass | `live-baseline-20260609083814` reports real, mocked, blocked, incomplete, wrong-result, and lifecycle-fail states explicitly. |

## Follow-Up Queue

No P0 Decodex lane needs to be requeued from this gate.

Recommended non-blocking follow-ups:

- Rerun `baseline-production-private` when an operator-owned private manifest is
  available, and publish a private-corpus addendum that does not expose private text.
- Keep qmd as the strongest external local baseline for routing/fusion/debuggability
  comparison work.
- Treat agentmemory, memsearch, mem0, OpenViking, and claude-mem adapter failures as
  typed benchmark improvement opportunities only if external parity coverage remains a
  roadmap goal.

## Runner Repairs Made By This Gate

Two small runner fixes were required to collect the fresh evidence:

- `build.rs` now provides a fallback `VERGEN_GIT_SHA=unknown` before vergen emits git
  metadata, so Docker benchmark builds work when the copied context is not a usable git
  checkout.
- `baseline-backfill-docker` now resolves default environment values inside the shell
  instead of relying on `${VAR:-default}` in the `cargo-make` TOML string, which avoided
  malformed values such as `-backfill`.
