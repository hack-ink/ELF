use super::*;

pub(super) async fn run_lifecycle_checks_impl(
	runtime: &BaselineRuntime,
	service: &ElfService,
	notes: &[CorpusNote],
	note_ids: &[Uuid],
) -> color_eyre::Result<Vec<CheckResult>> {
	let Some(update_note) = notes.first() else {
		return Ok(vec![incomplete_check(
			"update_replaces_note_text",
			"Corpus has no note to update.",
		)]);
	};
	let Some(update_note_id) = note_ids.first().copied() else {
		return Ok(vec![incomplete_check(
			"update_replaces_note_text",
			"ELF add_note returned no note_id for lifecycle update.",
		)]);
	};
	let Some(delete_note) = notes.get(1) else {
		return Ok(vec![incomplete_check(
			"delete_suppresses_retrieval",
			"Corpus has no note to delete.",
		)]);
	};
	let Some(delete_note_id) = note_ids.get(1).copied() else {
		return Ok(vec![incomplete_check(
			"delete_suppresses_retrieval",
			"ELF add_note returned no note_id for lifecycle delete.",
		)]);
	};
	let Some(recovery_note) = notes.get(2) else {
		return Ok(vec![incomplete_check(
			"cold_start_recovery_search",
			"Corpus has no stable note for recovery search.",
		)]);
	};

	Ok(vec![
		run_update_replacement_check(runtime, service, update_note, update_note_id).await?,
		run_delete_suppression_check(runtime, service, delete_note, delete_note_id).await?,
		run_cold_start_recovery_check(runtime, service, recovery_note).await?,
	])
}

async fn run_update_replacement_check(
	runtime: &BaselineRuntime,
	service: &ElfService,
	update_note: &CorpusNote,
	update_note_id: Uuid,
) -> color_eyre::Result<CheckResult> {
	let update_text = "\
	Rotated auth middleware validates JWT tokens with key id `kid-v4` under \
	`RotatedJwtKeyPlan`. It still requires tenant scope `project_shared` for deployment \
	operations after the emergency key rotation."
		.to_string();
	let update_response = service
		.update(UpdateRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			note_id: update_note_id,
			text: Some(update_text.clone()),
			importance: None,
			confidence: None,
			ttl_days: None,
		})
		.await?;
	let update_worker =
		run_worker_until_indexed(runtime, service, &[update_note_id], "lifecycle_update").await?;
	let update_query = run_single_query(
		service,
		QueryCase::generated(
			"lifecycle-update-new-marker".to_string(),
			"Which rotated JWT key id does the auth middleware require?".to_string(),
			update_note.source_doc.clone(),
			vec!["kid-v4".to_string(), "RotatedJwtKeyPlan".to_string()],
		),
	)
	.await?;
	let old_marker_absent = update_query
		.top_snippet
		.as_deref()
		.is_some_and(|snippet| !contains_case_insensitive(snippet, "kid-v3"));
	let update_pass = update_query.matched
		&& old_marker_absent
		&& outbox_done(&update_worker.after, update_worker.expected_note_count);

	Ok(CheckResult {
		name: "update_replaces_note_text",
		status: if update_pass { "pass" } else { "lifecycle_fail" },
		reason: if update_pass {
			"Service update plus worker indexing returned the new marker and removed the old marker from the top snippet.".to_string()
		} else {
			"Service update plus worker indexing did not produce a clean search result for the replacement marker.".to_string()
		},
		evidence: serde_json::json!({
			"note_id": update_note_id,
			"op": update_response.op,
			"worker": update_worker,
			"query": update_query,
			"old_marker_absent": old_marker_absent,
		}),
	})
}

async fn run_delete_suppression_check(
	runtime: &BaselineRuntime,
	service: &ElfService,
	delete_note: &CorpusNote,
	delete_note_id: Uuid,
) -> color_eyre::Result<CheckResult> {
	let delete_response = service
		.delete(DeleteRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: PROJECT_ID.to_string(),
			agent_id: AGENT_ID.to_string(),
			note_id: delete_note_id,
		})
		.await?;
	let delete_worker =
		run_worker_until_indexed(runtime, service, &[delete_note_id], "lifecycle_delete").await?;
	let delete_query = run_single_query(
		service,
		QueryCase::generated(
			"lifecycle-delete-suppresses-note".to_string(),
			delete_note.text.clone(),
			delete_note.source_doc.clone(),
			distinctive_terms(&delete_note.text, 2),
		),
	)
	.await?;
	let delete_pass = !delete_query.matched
		&& outbox_done(&delete_worker.after, delete_worker.expected_note_count);

	Ok(CheckResult {
		name: "delete_suppresses_retrieval",
		status: if delete_pass { "pass" } else { "lifecycle_fail" },
		reason: if delete_pass {
			"Service delete suppressed the deleted note from subsequent search results.".to_string()
		} else {
			"Deleted note was still retrievable after service delete and worker indexing."
				.to_string()
		},
		evidence: serde_json::json!({
			"note_id": delete_note_id,
			"op": delete_response.op,
			"worker": delete_worker,
			"query": delete_query,
		}),
	})
}

async fn run_cold_start_recovery_check(
	runtime: &BaselineRuntime,
	service: &ElfService,
	recovery_note: &CorpusNote,
) -> color_eyre::Result<CheckResult> {
	let recovery_service = build_service(runtime).await?;
	let recovery_query = run_single_query(
		&recovery_service,
		QueryCase::generated(
			"lifecycle-cold-start-recovery".to_string(),
			recovery_note.text.clone(),
			recovery_note.source_doc.clone(),
			distinctive_terms(&recovery_note.text, 2),
		),
	)
	.await?;
	let outbox_counts = pending_outbox_counts(service).await?;

	Ok(CheckResult {
		name: "cold_start_recovery_search",
		status: if recovery_query.matched { "pass" } else { "lifecycle_fail" },
		reason: if recovery_query.matched {
			"A newly constructed service over the same Postgres and Qdrant stores retrieved persisted evidence.".to_string()
		} else {
			"A newly constructed service over the same stores could not retrieve persisted evidence.".to_string()
		},
		evidence: serde_json::json!({
			"query": recovery_query,
			"pending_outbox_by_op": outbox_counts,
			"note": recovery_note.source_doc,
		}),
	})
}

async fn pending_outbox_counts(service: &ElfService) -> color_eyre::Result<BTreeMap<String, i64>> {
	let rows = sqlx::query_as::<_, (String, i64)>(
		"\
SELECT op, COUNT(*)::bigint
FROM indexing_outbox
WHERE status = 'PENDING'
GROUP BY op
ORDER BY op",
	)
	.fetch_all(&service.db.pool)
	.await?;

	Ok(rows.into_iter().collect())
}
