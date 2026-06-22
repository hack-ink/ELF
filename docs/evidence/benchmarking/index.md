# Benchmarking Evidence Index

Purpose: Route agents to checked-in benchmark reports, matrices, diagnostics, and
adoption evidence.
Read this when: You need public-safe evidence behind benchmark or production-readiness
claims.
Not this document: Commands for running benchmarks or governing benchmark schemas.
Routes to: Benchmarking evidence concepts under `docs/evidence/benchmarking/`.

## Concepts

- `2026-06-09-live-baseline-report.md`: Live Baseline Benchmark Report - 2026-06-09.
- `2026-06-09-operator-debugging-ux-report.md`: Real-World Job Benchmark Report.
- `2026-06-09-production-adoption-gate-report.md`: Production Adoption Gate Report - June 9, 2026.
- `2026-06-09-production-corpus-report.md`: Live Baseline Benchmark Report.
- `2026-06-10-live-real-world-sweep-report.md`: Live Real-World Adapter Sweep Report - June 10, 2026.
- `2026-06-10-production-adoption-refresh.md`: Post-Adapter Production Adoption Refresh - June 10, 2026.
- `2026-06-10-real-world-comparison-report.md`: Real-World Comparison Report - June 10, 2026.
- `2026-06-11-capture-write-policy-live-report.md`: Capture/Write-Policy Live Report - June 11, 2026.
- `2026-06-11-competitor-strength-adoption-report.md`: Competitor-Strength Adoption Report - June 11, 2026.
- `2026-06-11-competitor-strength-evidence-matrix.md`: Competitor-Strength Evidence Matrix - June 11, 2026.
- `2026-06-11-elf-iteration-direction-from-competitor-benchmarks.md`: ELF Iteration Direction From Competitor Benchmarks - June 11, 2026.
- `2026-06-11-elf-qmd-memory-evolution-diagnostic.md`: ELF/qmd Memory-Evolution Diagnostic - June 11, 2026.
- `2026-06-11-elf-qmd-retrieval-debug-profile.md`: ELF/qmd Retrieval-Debug Profile - June 11, 2026.
- `2026-06-11-elf-qmd-trace-replay-diagnostics-report.md`: ELF/qmd Trace Replay Diagnostics Report - June 11, 2026; qmd top-10/replay artifact evidence is compared with ELF trace/admin surfaces.
- `2026-06-11-first-generation-oss-adapter-promotion-report.md`: First-Generation OSS Adapter Promotion Report - June 11, 2026.
- `2026-06-11-first-generation-oss-continuity-source-store-report.md`: First-Generation OSS Continuity and Source-Store Report - June 11, 2026.
- `2026-06-11-graph-rag-scored-smoke-adapter-report.md`: Graph/RAG Scored Smoke Adapter Report - June 11, 2026.
- `2026-06-11-measurement-coverage-audit.md`: ELF Benchmark Measurement Coverage Audit - June 11, 2026.
- `2026-06-11-mem0-openmemory-history-ui-export-report.md`: mem0/OpenMemory History and UI Export Report - June 11, 2026.
- `2026-06-11-qmd-openviking-strength-profile-report.md`: qmd and OpenViking Strength-Profile Report - June 11, 2026; separates qmd retrieval quality from debug/replay ergonomics, preserves XY-928 OpenViking evidence, and keeps context-trajectory surfaces as blocked/not-tested until scored staged evidence exists.
- `2026-06-11-temporal-history-competitor-gap-report.md`: Temporal/History Competitor Gap Report - June 11, 2026.
- `2026-06-16-dreaming-readiness-stage-ledger.md`: Dreaming-Readiness Stage Ledger - June 16, 2026.
- `2026-06-16-live-consolidation-proposal-scoring-report.md`: Live Consolidation Proposal Scoring Report - June 16, 2026.
- `2026-06-16-live-temporal-reconciliation-report.md`: Live Temporal Reconciliation Report - June 16, 2026.
- `2026-06-16-proactive-brief-scoring-report.md`: Proactive Brief Scoring Report - June 16, 2026.
- `2026-06-16-scheduled-memory-task-scoring-report.md`: Real-World Job Benchmark Report.
- `2026-06-17-dreaming-competitor-strength-retest-report.md`: Dreaming Competitor-Strength Retest Report - June 17, 2026.
- `2026-06-19-graph-rag-citation-navigation-promotion-report.md`: Graph/RAG Citation and Navigation Promotion Report - June 19, 2026; refreshes the representative graph/RAG command and preserves the comparison as typed non-pass with graphify wrong_result, LightRAG incomplete, and RAGFlow/GraphRAG/Graphiti-Zep blockers.
- `2026-06-19-letta-core-archive-export-readback-report.md`: Letta Core/Archive Export-Readback Report - June 19, 2026; adds a Docker-contained Letta materialization/report command while preserving all six core/archive comparison scenarios as typed blockers until exported core block JSON, archival readback/search JSON, and source ids exist.
- `2026-06-19-openmemory-ui-export-product-readback-report.md`: OpenMemory UI/Export Product Readback Report - June 19, 2026; refreshes the product UI/export recheck and preserves the scenario as blocked because the export helper still needs Docker access to a running OpenMemory product container.
- `2026-06-19-openviking-trajectory-materialization-report.md`: OpenViking Trajectory Materialization Report - June 19, 2026; materializes the context-trajectory fixture slice through a dedicated repo task while preserving staged retrieval, hierarchy selection, and recursive/context expansion as typed blockers.
- `2026-06-19-operator-approved-public-proxy-production-private-addendum.md`: Operator-Approved Public-Proxy Production-Private Addendum - June 19, 2026; closes the current XY-930 proxy/simulated-corpus stage with 8/8 query pass, 0 wrong_result, and explicit boundaries that this is not real private-corpus or provider-backed proof.
- `2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.md`: qmd Debug-Ergonomics Dreaming Retest Report - June 19, 2026; confirms qmd's default top-k/replay edge is unchanged while ELF keeps the narrow operator-debug trace/stage visibility wins.
- `2026-06-19-service-native-dreaming-readback-report.md`: Service-Native Dreaming Readback Report - June 19, 2026; materializes memory summary, proactive brief, and scheduled-memory derived outputs through `ElfService` readback with 9 pass, 0 wrong_result, and 2 typed XY-930 blockers.
- `2026-06-20-dreaming-review-queue-report.md`: Dreaming Review Queue Report - June 20, 2026; adds the `elf.dreaming_review_queue/v1` source-backed queue over consolidation proposals with source refs, affected refs, lint, diff, policy, and review audit coverage for existing Dreaming suites plus tag, duplicate, page, memory-promotion, graph, and correction variants.
- `2026-06-20-graph-topic-map-report.md`: Graph Topic-Map Report - June 20, 2026; adds the ELF-native `elf.graph_report/v1` readback for Postgres graph-lite facts with sourced, inferred, ambiguous, stale, and superseded topic-map markers.
- `2026-06-20-knowledge-workspace-version-diff-report.md`: Knowledge Workspace Version-Diff Report - June 20, 2026; proves ELF knowledge pages now expose previous-version diff metadata without perturbing page content hashes while preserving citation, lint, and source-of-truth boundaries.
- `2026-06-20-live-knowledge-page-rebuild-lint-report.md`: Live Knowledge-Page Rebuild/Lint Report - June 20, 2026; adds a Docker-contained ELF service-native knowledge-page materialization command while preserving llm-wiki, gbrain, GraphRAG, RAGFlow, LightRAG, and graphify as separate comparison targets until they emit comparable scored page artifacts.
- `2026-06-20-recall-debug-panel-report.md`: Recall Debug Panel Report - June 20, 2026; adds `elf.recall_debug_panel/v1` as a typed cross-layer readback over memory traces, Source Library document candidates, Knowledge Workspace pages, graph facts, and Dreaming proposals, then extends the Memory Notes layer with qmd-style compact replay controls, stage movement, candidate replay, rerank effects, and selected-context artifacts while preserving not-requested and non-pass evidence classes.
- `2026-06-20-agent-knowledge-os-closeout-benchmark-report.md`: Agent Knowledge OS Closeout Benchmark Report - June 20, 2026; publishes the XY-1023 full product/scenario matrix, names ELF as the strongest measured integrated product, preserves qmd/OpenViking/mem0/OpenMemory/Letta/graph-RAG/VectifyAI strengths, and turns material non-pass or reference-only deltas into optimization queue items.
- `2026-06-22-p1-memory-authority-closeout-report.md`: P1 Memory Authority Closeout Report - June 22, 2026; adds `cargo make real-world-memory-p1-closeout`, scores the P1 Source Library -> Memory Candidate -> approved memory -> recall/debug -> correction/rollback chain as 4 pass, and keeps P2 queueing conditional on main-thread acceptance.
- `2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md`: P2 Knowledge Workspace PageIndex/OpenKB Closeout Report - June 22, 2026; adds `cargo make real-world-memory-p2-knowledge-closeout`, scores the Source Library and Knowledge Workspace fixture slices as pass, preserves PageIndex/OpenKB as `not_tested` reference-only rows, and keeps P3 adapter queueing behind main-thread acceptance.
- `2026-06-22-pageindex-openkb-same-corpus-adapter-report.md`: PageIndex/OpenKB Same-Corpus Adapter Report - June 22, 2026; adds `cargo make real-world-memory-pageindex-openkb`, emits checked-in same-corpus typed setup blockers for PageIndex and OpenKB, names source ids and required materialized outputs, and preserves no parity, win, tie, or loss claim.
- `2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.md`: mem0/OpenMemory and Letta Memory-History/Core-Archive Adapter Report - June 22, 2026; adds `cargo make real-world-memory-mem0-openmemory-letta`, maps mem0 SDK history/export outputs to source ids, preserves OpenMemory UI/export as a product blocker, preserves Letta core/archive readback as typed blockers, and makes no hosted/product parity claim.
- `2026-06-23-temporal-trajectory-adapter-coverage-report.md`: Temporal and Trajectory Adapter Coverage Report - June 23, 2026; refreshes Graphiti/Zep temporal-validity and OpenViking context-trajectory adapter evidence with trace-stage typed blockers, source ids, and explicit no-parity boundaries.
- `2026-06-23-graph-rag-adapter-matrix-report.md`: Graph/RAG Adapter Matrix Report - June 23, 2026; adds manifest-backed RAGFlow, GraphRAG, and LightRAG rows for retrieval, citation, navigation, stale-source behavior, answer faithfulness, and knowledge compilation while preserving 0 pass rows and no graph/RAG parity claim.
- `2026-06-23-p3-competitor-strength-absorption-report.md`: P3 Competitor-Strength Absorption Report - June 23, 2026; closes XY-1072 by naming which qmd, PageIndex/OpenKB, mem0/OpenMemory, Letta, Graphiti/Zep, OpenViking, RAGFlow, GraphRAG, and LightRAG strengths ELF absorbed, which remain stronger elsewhere or blocked, and which P4 optimization queue items are ready for main-thread inspection without applying a queue label.
- `2026-06-23-p4-production-readiness-evidence-gates-report.md`: P4 Production-Readiness Evidence Gates Report - June 23, 2026; adds `cargo make real-world-memory-p4-production-readiness`, records latency, cost, resource, cold-start, restore, and Qdrant rebuild evidence, separates local fixture, public-proxy, private-corpus, and provider-backed tiers, and preserves private/provider inputs as typed blockers.
- `2026-06-23-p4-quality-hardening-productization-readiness-report.md`: P4 Quality Hardening and Productization Readiness Report - June 23, 2026; adds `cargo make real-world-memory-p4-quality-hardening-closeout`, reruns adversarial, source-library, knowledge, and production-readiness slices, preserves private/provider blockers, and keeps P5 queueing behind main-thread acceptance with a narrowed productization scope.
