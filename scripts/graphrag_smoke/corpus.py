from __future__ import annotations

import csv
from pathlib import Path

from .common import slug
from .context import MAX_DOCS, MAX_INPUT_CHARS, REPORT_DIR



def generated_corpus() -> list[dict[str, str]]:
    """Return the bounded generated-public corpus."""

    docs = [
        {
            "evidence_id": "graphrag-smoke-nova-observatory",
            "title": "Nova Observatory memo",
            "text": (
                "Evidence ID graphrag-smoke-nova-observatory. Nova Observatory "
                "operates the public Aurora Index review. The Aurora Index links "
                "skyglow measurements to open weather station readings for civic "
                "science audits. The GraphRAG smoke must map this source document "
                "and its text unit back to the Nova Observatory evidence id."
            ),
        },
        {
            "evidence_id": "graphrag-smoke-aurora-index",
            "title": "Aurora Index field note",
            "text": (
                "Evidence ID graphrag-smoke-aurora-index. The Aurora Index uses "
                "Nova Observatory calibration notes when explaining why a public "
                "skyglow reading changed. The GraphRAG smoke must keep the Aurora "
                "Index source document and text unit evidence id recoverable."
            ),
        },
        {
            "evidence_id": "graphrag-smoke-stale-trap",
            "title": "Retired skyglow note",
            "text": (
                "Evidence ID graphrag-smoke-stale-trap. Retired note: Nova "
                "Observatory previously used the obsolete Zenith Ledger. This note "
                "is a distractor and must not be used as the primary answer."
            ),
        },
    ]
    trimmed: list[dict[str, str]] = []
    used_chars = 0

    for doc in docs[:MAX_DOCS]:
        remaining = MAX_INPUT_CHARS - used_chars

        if remaining <= 0:
            break

        text = doc["text"][:remaining].strip()
        used_chars += len(text)
        trimmed.append({**doc, "text": text})

    return trimmed


def write_corpus(project_dir: Path, corpus: list[dict[str, str]]) -> Path:
    """Write GraphRAG plain text input plus a CSV mapping copy."""

    input_dir = project_dir / "input"
    input_dir.mkdir(parents=True, exist_ok=True)
    csv_path = REPORT_DIR / "generated-corpus.csv"

    with csv_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=("evidence_id", "title", "text"))
        writer.writeheader()

        for item in corpus:
            writer.writerow(item)

    for item in corpus:
        file_name = f"{slug(item['evidence_id'])}.txt"
        (input_dir / file_name).write_text(
            f"Title: {item['title']}\nEvidence ID: {item['evidence_id']}\n\n{item['text']}\n",
            encoding="utf-8",
        )

    return csv_path
