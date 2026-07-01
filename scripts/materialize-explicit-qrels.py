#!/usr/bin/env python3
"""Generate explicit relevance-judgment fixtures from real-world job fixtures."""

from __future__ import annotations

import argparse
import json
import shutil
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


SCHEMA = "elf.real_world_explicit_qrel_materialization/v1"
JOB_SCHEMA = "elf.real_world_job/v1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Copy real_world_job fixtures and derive expected_answer.relevance_judgments "
            "from checked-in evidence_links/required_evidence."
        )
    )
    parser.add_argument("--fixtures", required=True, type=Path, help="Input fixture directory.")
    parser.add_argument("--out-fixtures", required=True, type=Path, help="Generated fixture directory.")
    parser.add_argument(
        "--summary-out",
        required=True,
        type=Path,
        help="Write materialization summary JSON.",
    )
    parser.add_argument(
        "--ranked-candidates-source",
        choices=["none", "oracle"],
        default="none",
        help="Optionally add fixture-trace ranked candidates ordered by qrel grade.",
    )
    parser.add_argument(
        "--profile",
        choices=["preserve", "generated_public"],
        default="preserve",
        help="Preserve original corpus profile or mark generated jobs as generated_public.",
    )
    parser.add_argument(
        "--exclude-without-positive-qrels",
        action="store_true",
        help="Do not copy job JSON files that have no positive derived qrels.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Replace existing relevance_judgments instead of preserving explicit grades.",
    )

    return parser.parse_args()


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        json.dump(value, fh, indent=2, sort_keys=False)
        fh.write("\n")


def stable_unique(values: list[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if value and value not in seen:
            seen.add(value)
            result.append(value)

    return result


def evidence_link_ids(value: Any) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, list):
        return [item for item in value if isinstance(item, str)]

    return []


def corpus_evidence_ids(job: dict[str, Any]) -> list[str]:
    return [
        item["evidence_id"]
        for item in job.get("corpus", {}).get("items", [])
        if isinstance(item, dict) and isinstance(item.get("evidence_id"), str)
    ]


def derive_positive_grades(job: dict[str, Any]) -> dict[str, float]:
    grades: dict[str, float] = {}
    expected = job.get("expected_answer", {})

    for link in expected.get("evidence_links", {}).values():
        for evidence_id in evidence_link_ids(link):
            grades[evidence_id] = max(grades.get(evidence_id, 0.0), 2.0)

    for evidence in job.get("required_evidence", []):
        if isinstance(evidence, dict) and isinstance(evidence.get("evidence_id"), str):
            grades[evidence["evidence_id"]] = max(grades.get(evidence["evidence_id"], 0.0), 1.0)

    return grades


def existing_qrel_grades(job: dict[str, Any]) -> dict[str, float]:
    grades: dict[str, float] = {}
    expected = job.get("expected_answer", {})
    for judgment in expected.get("relevance_judgments", []):
        if not isinstance(judgment, dict) or not isinstance(judgment.get("evidence_id"), str):
            continue
        grade = judgment.get("grade", 1.0)
        if isinstance(grade, (int, float)):
            grades[judgment["evidence_id"]] = float(grade)

    return grades


def materialized_qrels(job: dict[str, Any], overwrite: bool) -> list[dict[str, Any]]:
    evidence_ids = corpus_evidence_ids(job)
    grades = derive_positive_grades(job)

    if not overwrite:
        grades.update(existing_qrel_grades(job))

    if not any(grade > 0.0 for grade in grades.values()):
        return []

    return [
        {"evidence_id": evidence_id, "grade": grades.get(evidence_id, 0.0)}
        for evidence_id in evidence_ids
        if evidence_id in grades
    ]


def ranked_candidates_from_qrels(qrels: list[dict[str, Any]]) -> list[str]:
    return [
        judgment["evidence_id"]
        for judgment in sorted(
            qrels,
            key=lambda judgment: (
                -float(judgment.get("grade", 0.0)),
                str(judgment.get("evidence_id", "")),
            ),
        )
        if judgment.get("evidence_id")
    ]


def add_oracle_ranked_candidates(job: dict[str, Any], qrels: list[dict[str, Any]]) -> bool:
    answer = job.get("corpus", {}).get("adapter_response", {}).get("answer")
    if not isinstance(answer, dict):
        return False

    trace = answer.setdefault("trace_explainability", {})
    trace["ranked_candidate_evidence_ids"] = ranked_candidates_from_qrels(qrels)
    trace.setdefault("trace_id", f"{job.get('job_id', 'unknown')}-explicit-qrel-oracle")

    return True


def materialize_job(
    source: Path,
    target: Path,
    args: argparse.Namespace,
) -> dict[str, Any]:
    job = read_json(source)
    if not isinstance(job, dict) or job.get("schema") != JOB_SCHEMA:
        shutil.copy2(source, target)
        return {"kind": "copied_non_job_json"}

    qrels = materialized_qrels(job, overwrite=args.overwrite)
    if not qrels and args.exclude_without_positive_qrels:
        return {
            "kind": "excluded_without_positive_qrels",
            "job_id": job.get("job_id"),
        }

    ranked_candidate_added = False
    if qrels:
        expected = job.setdefault("expected_answer", {})
        had_existing_qrels = bool(expected.get("relevance_judgments"))
        expected["relevance_judgments"] = qrels
        tags = stable_unique([*job.get("tags", []), "explicit_qrels_generated"])
        job["tags"] = tags

        if args.profile == "generated_public":
            job.setdefault("corpus", {})["profile"] = "generated_public"

        if args.ranked_candidates_source == "oracle":
            ranked_candidate_added = add_oracle_ranked_candidates(job, qrels)

        write_json(target, job)
        return {
            "kind": "materialized_job",
            "job_id": job.get("job_id"),
			"judgment_count": len(qrels),
			"positive_judgment_count": sum(1 for judgment in qrels if judgment["grade"] > 0.0),
			"zero_grade_judgment_count": sum(1 for judgment in qrels if judgment["grade"] == 0.0),
			"unjudged_corpus_evidence_count": len(corpus_evidence_ids(job)) - len(qrels),
			"had_existing_qrels": had_existing_qrels,
			"ranked_candidate_added": ranked_candidate_added,
		}

    shutil.copy2(source, target)
    return {
        "kind": "copied_without_positive_qrels",
        "job_id": job.get("job_id"),
    }


def materialize(args: argparse.Namespace) -> dict[str, Any]:
    if not args.fixtures.is_dir():
        raise SystemExit(f"{args.fixtures} is not a directory")

    if args.out_fixtures.exists():
        shutil.rmtree(args.out_fixtures)
    args.out_fixtures.mkdir(parents=True)

    records: list[dict[str, Any]] = []
    for source in sorted(args.fixtures.rglob("*")):
        rel = source.relative_to(args.fixtures)
        target = args.out_fixtures / rel
        if source.is_dir():
            target.mkdir(parents=True, exist_ok=True)
            continue
        if source.suffix == ".json":
            records.append(materialize_job(source, target, args))
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

    materialized = [record for record in records if record["kind"] == "materialized_job"]
    excluded = [record for record in records if record["kind"] == "excluded_without_positive_qrels"]

    summary = {
        "schema": SCHEMA,
        "generated_at": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "input_fixture_dir": str(args.fixtures),
        "output_fixture_dir": str(args.out_fixtures),
        "ranked_candidates_source": args.ranked_candidates_source,
        "profile": args.profile,
        "exclude_without_positive_qrels": args.exclude_without_positive_qrels,
        "overwrite": args.overwrite,
        "job_count": len(materialized),
        "excluded_without_positive_qrels_count": len(excluded),
		"judgment_count": sum(record["judgment_count"] for record in materialized),
		"positive_judgment_count": sum(record["positive_judgment_count"] for record in materialized),
		"zero_grade_judgment_count": sum(record["zero_grade_judgment_count"] for record in materialized),
		"unjudged_corpus_evidence_count": sum(
			record["unjudged_corpus_evidence_count"] for record in materialized
		),
		"existing_qrel_job_count": sum(1 for record in materialized if record["had_existing_qrels"]),
        "ranked_candidate_job_count": sum(
            1 for record in materialized if record["ranked_candidate_added"]
        ),
        "excluded_job_ids": [record.get("job_id") for record in excluded],
        "claim_boundary": (
			"Derived qrels are deterministic benchmark labels from checked-in evidence links and "
			"required_evidence. Unmentioned corpus evidence remains unjudged instead of being "
			"converted into synthetic negative labels. Oracle ranked candidates test metric "
			"mechanics only; they are not product-runtime retrieval evidence or leaderboard proof."
		),
	}

    write_json(args.summary_out, summary)
    return summary


def main() -> None:
    args = parse_args()
    summary = materialize(args)
    print(
        "materialized explicit qrels: "
        f"{summary['job_count']} jobs, "
        f"{summary['judgment_count']} judgments, "
        f"{summary['ranked_candidate_job_count']} ranked-candidate traces"
    )


if __name__ == "__main__":
    main()
