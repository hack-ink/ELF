use crate::{BTreeMap, BackfillReport, CheckResult, QueryResult, WorkerRunEvidence};

pub(crate) fn outbox_done(counts: &BTreeMap<String, i64>, expected_note_count: usize) -> bool {
	let done = counts.get("DONE").copied().unwrap_or_default();
	let expected = i64::try_from(expected_note_count).unwrap_or(i64::MAX);
	let pending = counts.get("PENDING").copied().unwrap_or_default();
	let failed = counts.get("FAILED").copied().unwrap_or_default();
	let claimed = counts.get("CLAIMED").copied().unwrap_or_default();

	done >= expected && pending == 0 && failed == 0 && claimed == 0
}

pub(crate) fn retrieval_check(query_results: &[QueryResult]) -> CheckResult {
	let pass_count = query_results.iter().filter(|result| result.matched).count();
	let fail_count = query_results.len().saturating_sub(pass_count);
	let expected_evidence_ids = query_results
		.iter()
		.map(|result| {
			serde_json::json!({
				"query_id": result.id,
				"expected": result.expected_evidence_ids,
				"allowed_alternates": result.allowed_alternate_evidence_ids,
			})
		})
		.collect::<Vec<_>>();

	CheckResult {
		name: "same_corpus_retrieval",
		status: if fail_count == 0 { "pass" } else { "wrong_result" },
		reason: if fail_count == 0 {
			"All same-corpus retrieval queries returned expected evidence.".to_string()
		} else {
			format!("{fail_count} same-corpus retrieval query case(s) missed expected evidence.")
		},
		evidence: serde_json::json!({
			"total": query_results.len(),
			"pass": pass_count,
			"fail": fail_count,
			"wrong_result_count": fail_count,
			"expected_evidence_ids": expected_evidence_ids,
		}),
	}
}

pub(crate) fn worker_indexing_check(evidence: WorkerRunEvidence) -> CheckResult {
	let pass = outbox_done(&evidence.after, evidence.expected_note_count)
		&& evidence.chunk_rows >= i64::try_from(evidence.expected_note_count).unwrap_or(i64::MAX)
		&& evidence.chunk_embedding_rows >= evidence.chunk_rows;

	CheckResult {
		name: "async_worker_indexing_e2e",
		status: if pass { "pass" } else { "lifecycle_fail" },
		reason: if pass {
			"ELF worker processed corpus outbox jobs into persisted chunks and embeddings."
				.to_string()
		} else {
			"ELF worker did not fully process corpus outbox jobs into searchable chunks."
				.to_string()
		},
		evidence: serde_json::json!(evidence),
	}
}

pub(crate) fn resumable_backfill_check(report: &BackfillReport) -> CheckResult {
	let resume_pass = !report.resume.enabled
		|| (report.resume.interrupted
			&& report.resume.resume_attempts >= 2
			&& report.skipped_completed > 0);
	let pass = report.completed_count == report.source_count
		&& report.duplicate_source_notes.is_empty()
		&& resume_pass;

	CheckResult {
		name: "resumable_backfill_no_duplicates",
		status: if pass { "pass" } else { "lifecycle_fail" },
		reason: if pass {
			"Checkpointed backfill resumed from durable progress and did not duplicate source documents."
				.to_string()
		} else {
			"Checkpointed backfill did not complete cleanly, did not prove resume, or duplicated source documents."
				.to_string()
		},
		evidence: serde_json::json!(report),
	}
}
