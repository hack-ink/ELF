# Live Baseline Benchmark

Goal: Run Docker-isolated, current-HEAD baseline checks against ELF and the external memory projects compared with ELF.
Read this when: You need evidence about which external projects actually run against a shared benchmark corpus.
Preconditions: Docker and Docker Compose are available on the host.
Depends on: `docker-compose.baseline.yml`, `scripts/live-baseline-benchmark.sh`,
`docs/spec/system_competitive_parity_gate_v1.md`, and
`docs/spec/production_corpus_manifest_v1.md`.
Verification: `cargo make baseline-live-docker` writes `tmp/live-baseline/live-baseline-report.json`; `cargo make baseline-live-report` can render that JSON into a checked-in Markdown report.

## Scope

This guide is for benchmark evidence, not for operating a personal production ELF service. For
single-user Docker Compose production start, stop, health, backup, restore, Qdrant rebuild,
rollback, and cleanup commands, use `docs/guide/single_user_production.md`.

The runner covers ELF plus the six external projects in the README comparison table:

- ELF
- agentmemory
- OpenViking
- mem0
- qmd
- claude-mem
- memsearch

For ELF, the runner uses Docker-owned Postgres and Qdrant, writes the shared corpus
through `add_note`, drains the worker indexing outbox into persisted chunks and
embeddings, rebuilds Qdrant from the worker-produced chunk tables, and verifies
`search_raw` against the shared query manifest. It also runs ELF service lifecycle
checks for note update, note delete, cold-start recovery, concurrent writes,
configurable soak stability, and a local resource envelope over the same Docker-owned
stores. By default these checks use the deterministic local embedding provider. Set
`ELF_BASELINE_ELF_EMBEDDING_MODE=provider` to run ELF through the configured
production embedding provider instead.

For external projects, the runner clones current upstream `main` inside Docker, records
the exact commit SHA, reads the same generated corpus and query manifest, and runs a
same-corpus retrieval adapter when the project exposes a local API or CLI that can run
without provider keys. Each project record includes adapter metadata that marks storage
and behavior surfaces as `real`, `mocked`, `unsupported`, `blocked`, `incomplete`, or
`not_encoded`.

Corpus profiles:

- `smoke`: default, 3 documents and 3 query cases.
- `scale`: 120 documents by default, 8 query cases, and generated distractor notes
  that make the check closer to a production retrieval benchmark.
- `stress`: 480 documents by default, 16 query cases, and alternate phrasings for
  every needle query.
- `production-synthetic`: checked-in synthetic coding-agent production corpus with
  issues, PRs, worktrees, runbooks, decisions, blockers, recovery notes, and
  task-oriented queries. Fixture:
  `apps/elf-eval/fixtures/production_corpus/synthetic_coding_agent_manifest.json`.
- `production-private`: local private/sanitized production corpus manifest supplied by
  `ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST`.
- `backfill`: 2000 documents by default, 16 query cases, alternate phrasings for
  every needle query, and ELF-only resumable backfill evidence.

Use `ELF_BASELINE_SCALE_DOCS` and `ELF_BASELINE_STRESS_DOCS` to raise or lower the
generated corpus sizes.
Use `ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST` to supply a local manifest that follows
`docs/spec/production_corpus_manifest_v1.md`. The private profile fails closed when the
manifest path is absent, the file is missing, a referenced `local_path` is missing, or a
query references an unknown evidence ID. It does not fall back to the checked-in
synthetic fixture.
Use `ELF_BASELINE_BACKFILL_DOCS` to set the generated corpus size for the backfill
profile; values such as `10000` are supported for operator-controlled stress runs.
Use `cargo make baseline-backfill-10k-docker` for the checked-in 10k operator profile.
Use `cargo make baseline-backfill-100k-docker` only with
`ELF_BASELINE_ENABLE_EXPENSIVE=1`; the task fails closed without that explicit guard.
Use `ELF_BASELINE_CONCURRENT_NOTES`, `ELF_BASELINE_MAX_ELF_SECONDS`, and
`ELF_BASELINE_MAX_ELF_RSS_KB` to tune ELF's concurrent-write and resource-envelope
checks.
Use `ELF_BASELINE_SOAK_SECONDS`, `ELF_BASELINE_SOAK_ROUNDS`, and
`ELF_BASELINE_SOAK_PROBE_INTERVAL_MS` to tune ELF's repeated write/search soak
window. The smoke profile does not run soak by default; the scale/full profiles run a
short 15-second soak by default, and the stress profile runs a 60-second soak by
default. Use `cargo make baseline-soak-docker` for an explicit one-hour ELF-only soak,
or override `ELF_BASELINE_SOAK_SECONDS` for a shorter or longer operator-controlled
window.
Use `ELF_BASELINE_ELF_EMBEDDING_MODE=provider` plus
`ELF_BASELINE_ELF_EMBEDDING_API_BASE`, `ELF_BASELINE_ELF_EMBEDDING_API_KEY`,
`ELF_BASELINE_ELF_EMBEDDING_MODEL`, and
`ELF_BASELINE_ELF_EMBEDDING_DIMENSIONS` to run ELF with a production embedding API.
The runner also accepts `QWEN_API_KEY`, `QWEN_EMBEDDING_API_BASE`,
`QWEN_EMBEDDING_MODEL`, `QWEN_EMBEDDING_DIMENSIONS`, and `QWEN_EMBEDDING_PATH` for
Qwen-compatible embedding configuration. Generic aliases `EMBEDDING_API_BASE`,
`EMBEDDING_API_KEY`, `EMBEDDING_MODEL`, `EMBEDDING_DIMENSIONS`,
`EMBEDDING_PROVIDER_ID`, `EMBEDDING_PATH`, and `EMBEDDING_TIMEOUT_MS` are also
supported. Provider-mode runs default to a 30-second embedding timeout unless an
explicit timeout env var is set. For Qwen3 production embedding runs, use
`Qwen3-Embedding-8B` with `EMBEDDING_DIMENSIONS=4096`. The aggregate report records
ELF's embedding mode, provider id, model, dimensions, timeout, API base, and path; it
never records the API key.
For ELF backfill runs, the runner writes a durable checkpoint file under the report
directory by default, intentionally interrupts the first pass unless
`ELF_BASELINE_BACKFILL_RESUME_PROBE=0`, then resumes from the checkpoint. Tune
`ELF_BASELINE_BACKFILL_BATCH_SIZE`, `ELF_BASELINE_BACKFILL_INTERRUPT_AFTER`,
`ELF_BASELINE_BACKFILL_CHECKPOINT`, and `ELF_BASELINE_WORKER_CONCURRENCY` when
measuring import and indexing throughput.
Set `ELF_BASELINE_COST_PER_1K_TOKENS_USD` to attach a planning-only cost proxy to
ELF reports. The proxy estimates input tokens from primary corpus note text plus
declared same-corpus query text; it is not a billing statement.

The ELF report records:

- duplicate source-note count and checkpoint resume state;
- query latency mean, P50, P95, P99, and max;
- local RSS, Postgres database bytes, corpus bytes, report-directory bytes, and
  checkpoint-file bytes;
- the optional cost proxy described above;
- operator-case commands for private addendum, 10k/100k resume, provider outage,
  Docker Compose start/stop/upgrade, migration rollback, Postgres restore, Qdrant
  rebuild, and unattended soak.

Current external same-corpus adapters:

- agentmemory: writes every corpus document through `mem::remember`, queries through
  `mem::search`, exercises `mem::forget` delete suppression, and probes
  superseding by writing a revised memory through `mem::remember`. The current
  adapter uses an in-memory SDK/KV mock, so behavior metadata is `mocked` and durable
  cold-start recovery is recorded as `blocked` until a persistent agentmemory KV/index
  path or hosted runtime is wired into the harness.
- qmd: adds the corpus as a collection, embeds it locally, and runs structured hybrid
  `query --json` for every query case. It also rewrites and deletes corpus files,
  then reruns `qmd update`, `qmd embed -f`, and fresh `qmd query` processes.
- memsearch: indexes the corpus with the local ONNX embedder and runs CLI search.
  It also rewrites and deletes corpus files, then reruns `memsearch index` and
  fresh `memsearch search` processes.
- mem0: writes the corpus with `infer=false` and searches local FastEmbed + Qdrant
  path storage. It also runs public `Memory.update`, `Memory.delete`, and a new
  `Memory.from_config` over the same local paths. No LLM inference is required.
- claude-mem: writes every corpus document into the SQLite memory repository and runs
  repository search for every query case.

Current deeper checks:

- ELF: same-corpus retrieval through worker-produced chunks, async worker indexing
  completion, resumable checkpointed backfill without duplicate source notes, service
  update replacement through the worker, service delete suppression through the worker,
  cold-start search recovery after constructing a fresh service over the same Postgres
  and Qdrant stores, concurrent write/search E2E, configurable repeated write/search
  soak stability, and a configurable local resource envelope.
- qmd, memsearch, and mem0: same-corpus retrieval, update replacement, delete
  suppression, and cold-start search recovery through their local public API or CLI
  surfaces.
- agentmemory: same-corpus retrieval and delete suppression are exercised; update
  replacement is probed through superseding `mem::remember`; cold-start recovery is
  `blocked` because the current adapter runs against an in-memory SDK/KV mock.
- claude-mem and OpenViking: same-corpus retrieval only when their local runtime path
  can complete. Update, delete, and recovery checks are `not_encoded` for these two
  adapters.
- Concurrent write, soak stability, and resource-envelope checks are currently encoded
  for ELF. They are not yet encoded for the external adapters. Multi-hour production
  soak is still operator-controlled through `ELF_BASELINE_SOAK_SECONDS`; the checked-in
  stress default is a bounded 60-second signal.

OpenViking attempts the official `.[local-embed]` path plus `OpenViking.add_resource`
and `OpenViking.find`. If the Docker platform cannot build or import
`llama-cpp-python`, the project is recorded as `incomplete` with
`retrieval_status = "local_embed_install_failed"` rather than as a retrieval failure.
The adapter metadata includes retry guidance to pin or provide a Docker-compatible
local embedding dependency before scaling the OpenViking profile.

## Checked-In Reports

- `docs/guide/benchmarking/2026-06-09-live-baseline-report.md`: June 9, 2026
  production-provider ELF stress run and all-project smoke comparison.

## Run

```sh
cargo make baseline-live-docker
```

To run the scale profile:

```sh
ELF_BASELINE_PROFILE=scale cargo make baseline-live-docker
ELF_BASELINE_PROFILE=scale ELF_BASELINE_SCALE_DOCS=240 cargo make baseline-live-docker
ELF_BASELINE_PROFILE=stress cargo make baseline-live-docker
ELF_BASELINE_PROJECTS=ELF ELF_BASELINE_PROFILE=backfill cargo make baseline-live-docker
cargo make baseline-backfill-docker
cargo make baseline-backfill-10k-docker
ELF_BASELINE_ENABLE_EXPENSIVE=1 cargo make baseline-backfill-100k-docker
ELF_BASELINE_SOAK_SECONDS=3600 cargo make baseline-soak-docker
```

To iterate on one or more project adapters without rerunning the full matrix:

```sh
ELF_BASELINE_PROJECTS=qmd cargo make baseline-live-docker
ELF_BASELINE_PROJECTS=ELF,memsearch cargo make baseline-live-docker
```

To run the checked-in synthetic production-style corpus through ELF:

```sh
cargo make baseline-production-synthetic
```

To run a private local production corpus without committing private content:

```sh
ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST=tmp/private-production-corpus/manifest.json \
cargo make baseline-production-private
```

The private manifest can contain sanitized inline `text` fields or `local_path` fields
that point to local sanitized text/Markdown files. Keep private manifests and local
evidence under `tmp/` or outside the repository. `tmp/` is ignored by git.
The manifest `manifest_id`, evidence IDs, and query IDs are report-visible labels; keep
them lower-case ASCII identifiers and do not encode private text in those fields.

To run the same private profile and publish a safe Markdown addendum under `tmp/`:

```sh
ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST=tmp/private-production-corpus/manifest.json \
cargo make baseline-production-private-addendum
```

The default addendum path is:

```text
tmp/live-baseline/private-production-addendum.md
```

Override it with `ELF_BASELINE_PRIVATE_ADDENDUM`. The addendum intentionally reports
manifest id, evidence ids, task labels, checks, latency, backfill, resource, cost
proxy, and operator-case fields without embedding private evidence text or local
private file paths. Raw JSON and logs remain under `tmp/live-baseline/` and must be
reviewed before any manual copy into durable docs.

The only host artifact is:

```text
tmp/live-baseline/
```

That directory contains the aggregate report, per-project logs, and the shared query
fixture used by the run. The aggregate report records `corpus.profile`,
`corpus.track`, `corpus.manifest_id`, `corpus.document_count`, and
`corpus.query_count` so generated public corpus results are not confused with
synthetic or private production-corpus results. Each project record includes
`elapsed_seconds` for rough local runtime comparison and an `adapter` metadata object
that distinguishes real, mocked, unsupported, blocked, incomplete, and not-encoded
behavior surfaces. ELF project records also include an `embedding` summary so
deterministic local and production-provider runs are not confused. ELF query records
include task, trace ID, expected evidence IDs, allowed alternate evidence IDs, top
evidence ID, wrong-result count, and per-query latency. Each ELF trace ID can be opened
from the admin viewer at `/viewer` by loading it in the Traces panel; the full trace
bundle shows stage-level candidates, rerank terms, relation context, and provider
metadata without raw SQL. Each project record also includes
`backfill` evidence with source count, completed count, batch size, worker
concurrency, resume state, duplicate-source count, and backfill elapsed seconds. Each
project record also includes `checks` and `check_summary`; the aggregate
`full_check_summary` is the adoption-relevant multi-check count.

Production-ready claims must cite a concrete report path. A claim based only on
generated public `smoke`, `scale`, or `stress` profiles is not enough for personal
production adoption. Cite a `production-synthetic` report for fixture coverage, and
cite a `production-private` report when making a private-corpus production-readiness
claim.
If no operator-owned private manifest is supplied, the private-corpus path is a
bounded failure, not a pass.

For job-level production-ops coverage under the real-world benchmark contract, run:

```sh
cargo make real-world-memory-production-ops
```

That target parses checked-in fixture evidence for interrupted backfill resume,
backup/restore readback, cold-start recovery, resource-envelope interpretation, and
typed private-manifest, credential, and dependency boundaries. It does not run Docker,
private corpus data, or provider-backed credentials, and it must not be used as a
substitute for `baseline-production-private` when making a private-corpus readiness
claim.

## Publish A Markdown Report

After a run writes `tmp/live-baseline/live-baseline-report.json`, render a durable
Markdown summary:

```sh
cargo make baseline-live-report
```

By default the task prints Markdown to stdout. To write a checked-in report:

```sh
ELF_BASELINE_MARKDOWN_REPORT=docs/guide/benchmarking/YYYY-MM-DD-live-baseline-report.md \
cargo make baseline-live-report
```

The publisher summarizes one generated aggregate JSON report. For a combined report
that compares multiple runs, use the generated Markdown as input evidence and then add
the interpretation manually under `docs/guide/benchmarking/`.

## Real-World Job Smoke

The live-baseline runner and real-world job runner publish separate report schemas.
Live-baseline reports remain evidence for Docker retrieval and lifecycle checks only.
They are not real-world suite wins.
The real-world runner loads
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`
by default and records live-baseline-only external adapter evidence under
`external_adapters`; those records preserve the typed setup/run evidence but still
leave real-world suites as `not_encoded`, `blocked`, `incomplete`, `wrong_result`, or
`lifecycle_fail` until an adapter actually executes `real_world_job` prompts and
scoring.

The targeted live real-world adapter slice for ELF and qmd is separate from the
same-corpus live baseline:

```sh
cargo make real-world-memory-live-adapters
```

This task runs in `docker-compose.baseline.yml`, materializes generated
`adapter_response` fixtures through ELF's service runtime and qmd's local CLI
retrieval path, then scores and publishes:

```text
tmp/real-world-memory/live-adapters/elf-report.json
tmp/real-world-memory/live-adapters/elf-report.md
tmp/real-world-memory/live-adapters/qmd-report.json
tmp/real-world-memory/live-adapters/qmd-report.md
tmp/real-world-memory/live-adapters/summary.json
```

To run the checked-in real-world job smoke fixture and render its Markdown report:

```sh
cargo make real-world-job-smoke
```

To run the checked-in work-resume, source-of-truth, lifecycle, redaction,
capture-boundary, and personalization real-world memory fixtures:

```sh
cargo make real-world-memory
```

Artifacts:

```text
tmp/real-world-job/real-world-job-smoke-report.json
tmp/real-world-job/real-world-job-smoke-report.md
tmp/real-world-memory/real-world-memory-report.json
tmp/real-world-memory/real-world-memory-report.md
```

The smoke fixture suite lives under
`apps/elf-eval/fixtures/real_world_memory/work_resume/` and uses
`docs/spec/real_world_agent_memory_benchmark_v1.md` status terms, including
`unsupported_claim`. The checked-in slice includes work-resume continuity jobs and one
capture/integration boundary job. Suites without checked-in jobs are reported as
`not_encoded`.

The broader real-world memory fixture set lives under
`apps/elf-eval/fixtures/real_world_memory/` and adds summary counters for evidence
coverage, source-ref coverage, quote coverage, stale retrievals, scope correctness,
redaction leaks, and Qdrant rebuild coverage.

The memory evolution suite is a separate checked-in real-world job fixture set:

```sh
cargo make real-world-memory-evolution
```

It lives under `apps/elf-eval/fixtures/real_world_memory/evolution/` and reports
stale-answer count, conflict detection count, update rationale availability, temporal
validity encoding, and unsupported claims. Its relation-temporal fixture is encoded as
a normal pass/fail check for current versus historical graph-lite relation context.

To run the checked-in retrieval-quality real-world fixtures:

```sh
cargo make real-world-memory-retrieval
```

Artifacts:

```text
tmp/real-world-memory/retrieval-report.json
tmp/real-world-memory/retrieval-report.md
```

The retrieval fixture lives under
`apps/elf-eval/fixtures/real_world_memory/retrieval/` and covers alternate phrasing,
distractor-heavy corpora, multi-hop routing questions, current-versus-obsolete context
selection, minimal sufficient context, and stage-level wrong-result explainability.
It is still an offline fixture report. qmd has a separate targeted live adapter slice
through `cargo make real-world-memory-live-adapters`; OpenViking remains a reference
system unless an adapter actually runs and records typed evidence.

To run the checked-in proposal-only consolidation fixtures:

```sh
cargo make real-world-memory-consolidation
```

Artifacts:

```text
tmp/real-world-memory/consolidation/report.json
tmp/real-world-memory/consolidation/report.md
```

The consolidation fixtures live under
`apps/elf-eval/fixtures/real_world_memory/consolidation/`. They score reviewable
proposal payloads, source lineage, review action outcomes, executable gaps, and source
mutation count. They do not claim live scheduled consolidation-worker generation.

To run the checked-in knowledge-compilation and page-rebuild fixtures:

```sh
cargo make real-world-memory-knowledge
```

Artifacts:

```text
tmp/real-world-memory/knowledge-report.json
tmp/real-world-memory/knowledge-report.md
```

The knowledge fixtures live under
`apps/elf-eval/fixtures/real_world_memory/knowledge/`. They score derived page
citation coverage, stale-claim linting, rebuild determinism, backlink coverage, page
usefulness, and explicitly flagged unsupported summaries. Generated pages are
benchmark artifacts, not source-truth replacements.

## Clean Up

```sh
cargo make baseline-live-docker-clean
```

This removes Docker-managed Postgres, Qdrant, npm, pip, cargo, and target volumes used
by the live baseline runner. It does not remove the host report directory.

## Result Semantics

The result terms below belong to the current Docker live baseline. For the future
job-level suite contract, including `unsupported_claim`, see
`docs/spec/real_world_agent_memory_benchmark_v1.md`.

- `pass`: the project installed and every encoded check for that project passed in the
  selected corpus profile.
- `wrong_result`: a retrieval check completed but returned the wrong memory or missed
  expected evidence.
- `lifecycle_fail`: same-corpus retrieval may pass, but an encoded update, delete,
  cold-start, persistence, or related lifecycle check failed.
- `incomplete`: setup or a declared check could not complete because install, runtime,
  dependency, or adapter wiring failed in Docker.
- `blocked`: a safe check cannot run without external credentials, manual setup,
  durable runtime wiring, or host integration outside this run.
- `not_encoded`: the capability is not covered by the current adapter, so no pass/fail
  claim is allowed.

The top-level `verdict` is intentionally stricter than the per-project `status`: it
only returns `pass` when every selected project has `status = "pass"` and
`retrieval_status = "retrieval_pass"`. The `same_corpus_summary` field is the
retrieval count and does not treat lifecycle failures as retrieval failures. For
multi-check comparisons, read `full_check_summary`, each project's `checks`, and the
adapter behavior metadata.

`incomplete`, `blocked`, and `not_encoded` are not passes. Treat them as evidence that
more benchmark wiring or upstream/runtime support is needed.

## Failure Conditions

A project status should be `wrong_result` when same-corpus retrieval runs but does not
return the expected evidence. A project status should be `lifecycle_fail` when
retrieval is not the failing condition but an encoded update, delete, cold-start,
persistence, concurrent, soak, or resource-envelope check completes and proves the
project did not meet the selected benchmark contract.

Use `incomplete` when the runner cannot execute the declared check fairly because clone,
install, import, build, adapter wiring, native dependency support, or local runtime
setup failed. Use `blocked` when the check needs credentials, manual setup, durable
runtime integration, or host integration outside the issue scope. Use `not_encoded`
when the adapter simply does not cover the capability yet.
