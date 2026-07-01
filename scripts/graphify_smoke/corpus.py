from __future__ import annotations

import csv
import shutil
from pathlib import Path

from .context import CORPUS_DIR, REPORT_DIR
from .models import CorpusItem



def generated_corpus() -> list[CorpusItem]:
    """Return the bounded generated-public graphify corpus."""

    return [
        CorpusItem(
            evidence_id="graphify-smoke-memory-service",
            claim_id="memory_service_graph",
            title="ELF Memory Service Graph Note",
            file_name="elf_memory_service.py",
            text=(
                '"""Evidence ID graphify-smoke-memory-service.\n'
                "ELF stores evidence-linked facts as notes and keeps Postgres as the "
                "source of truth for graph/report validation.\n"
                '"""\n\n'
                "class ElfMemoryService:\n"
                "    \"\"\"Evidence ID graphify-smoke-memory-service maps memory notes "
                "to source-backed graph nodes.\"\"\"\n\n"
                "    def attach_evidence(self, note_id: str, source_ref: str) -> tuple[str, str]:\n"
                "        \"\"\"Attach source_ref evidence to a note before retrieval.\"\"\"\n"
                "        return note_id, source_ref\n"
            ),
            expected=True,
        ),
        CorpusItem(
            evidence_id="graphify-smoke-qdrant-rebuild",
            claim_id="qdrant_rebuild_graph",
            title="Qdrant Rebuild Graph Note",
            file_name="qdrant_rebuild.py",
            text=(
                '"""Evidence ID graphify-smoke-qdrant-rebuild.\n'
                "Qdrant is a derived, rebuildable index. The graphify smoke should "
                "connect Qdrant rebuild evidence to the ELF memory service node and "
                "preserve this source file as evidence for scoring.\n"
                '"""\n\n'
                "class QdrantRebuildIndex:\n"
                "    \"\"\"Evidence ID graphify-smoke-qdrant-rebuild maps rebuildable "
                "index behavior to source evidence.\"\"\"\n\n"
                "    def rebuild_from_postgres_vectors(self, collection: str) -> str:\n"
                "        \"\"\"Rebuild the derived Qdrant collection from Postgres vectors.\"\"\"\n"
                "        return collection\n"
            ),
            expected=True,
        ),
        CorpusItem(
            evidence_id="graphify-smoke-report-mapping",
            claim_id="graph_report_mapping",
            title="Graph Report Mapping Note",
            file_name="graph_report_mapping.py",
            text=(
                '"""Evidence ID graphify-smoke-report-mapping.\n'
                "GRAPH_REPORT.md and graph.json must be captured as derived adapter "
                "artifacts, then mapped back to real_world_job evidence ids.\n"
                '"""\n\n'
                "def map_graph_report_to_evidence(graph_json: str, graph_report: str) -> str:\n"
                "    \"\"\"Return graphify-smoke-report-mapping when graph artifacts cite sources.\"\"\"\n"
                "    return f\"{graph_json}:{graph_report}\"\n"
            ),
            expected=True,
        ),
        CorpusItem(
            evidence_id="graphify-smoke-stale-trap",
            claim_id="stale_authority_trap",
            title="Stale Graph Authority Trap",
            file_name="stale_vector_authority.py",
            text=(
                '"""Evidence ID graphify-smoke-stale-trap.\n'
                "Stale trap: graphify output is an authoritative ELF memory store. "
                "This is intentionally false; graphify is only a derived graph/report adapter.\n"
                '"""\n\n'
                "def stale_authority_claim() -> str:\n"
                "    \"\"\"Return the stale claim that must not drive the answer.\"\"\"\n"
                "    return \"graphify is authoritative\"\n"
            ),
            expected=False,
        ),
    ]


def write_corpus(corpus: list[CorpusItem]) -> Path:
    """Write graphify input files plus a CSV mapping copy."""

    if CORPUS_DIR.exists():
        shutil.rmtree(CORPUS_DIR)
    CORPUS_DIR.mkdir(parents=True, exist_ok=True)
    csv_path = REPORT_DIR / "generated-corpus.csv"

    with csv_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=("evidence_id", "claim_id", "title", "file_name", "line", "text"),
        )
        writer.writeheader()

        for item in corpus:
            line = evidence_line(item.text, item.evidence_id)
            item.line = line
            writer.writerow(
                {
                    "evidence_id": item.evidence_id,
                    "claim_id": item.claim_id,
                    "title": item.title,
                    "file_name": item.file_name,
                    "line": line,
                    "text": item.text,
                }
            )
            (CORPUS_DIR / item.file_name).write_text(item.text, encoding="utf-8")

    (CORPUS_DIR / ".graphifyignore").write_text(
        "graphify-out/\n__pycache__/\n*.pyc\n",
        encoding="utf-8",
    )

    return csv_path


def evidence_line(text: str, evidence_id: str) -> int:
    """Return the first line containing an evidence id."""

    for index, line in enumerate(text.splitlines(), start=1):
        if evidence_id in line:
            return index

    return 1


def expected_ids(corpus: list[CorpusItem]) -> list[str]:
    """Return expected evidence ids for pass scoring."""

    return [item.evidence_id for item in corpus if item.expected]
