use super::*;

pub(super) async fn ingest_elf_corpus(
	service: &ElfService,
	loaded: &LoadedJob,
	adapter_id: &str,
	project_id: &str,
	corpus: &[CorpusText],
) -> color_eyre::Result<IngestedCorpus> {
	let mut ingested = IngestedCorpus::default();

	for item in corpus {
		if item.capture.action == LiveCaptureAction::Exclude {
			push_unique(&mut ingested.capture.excluded_evidence_ids, item.evidence_id.clone());

			continue;
		}

		push_unique(&mut ingested.capture.stored_evidence_ids, item.evidence_id.clone());

		if let Some(source_id) = item.capture.source_id.as_deref() {
			push_unique(&mut ingested.capture.source_ids, source_id.to_string());
		}

		if item.capture.write_policy.is_some() {
			let note_id = ingest_elf_corpus_item(
				service,
				loaded,
				adapter_id,
				project_id,
				item,
				item.evidence_id.clone(),
				item.text.clone(),
				0,
				1,
				&mut ingested.capture,
			)
			.await?;

			ingested
				.note_ids_by_evidence
				.entry(item.evidence_id.clone())
				.or_default()
				.push(note_id);

			continue;
		}

		let chunks = note_text_chunks(item.text.as_str());
		let chunk_count = chunks.len();

		for (chunk_index, text) in chunks.into_iter().enumerate() {
			let key = if chunk_count == 1 {
				item.evidence_id.clone()
			} else {
				format!("{}:chunk-{chunk_index:03}", item.evidence_id)
			};
			let note_id = ingest_elf_corpus_item(
				service,
				loaded,
				adapter_id,
				project_id,
				item,
				key,
				text,
				chunk_index,
				chunk_count,
				&mut ingested.capture,
			)
			.await?;

			ingested
				.note_ids_by_evidence
				.entry(item.evidence_id.clone())
				.or_default()
				.push(note_id);
		}
	}

	Ok(ingested)
}

#[allow(clippy::too_many_arguments)]
async fn ingest_elf_corpus_item(
	service: &ElfService,
	loaded: &LoadedJob,
	adapter_id: &str,
	project_id: &str,
	item: &CorpusText,
	key: String,
	text: String,
	chunk_index: usize,
	chunk_count: usize,
	capture: &mut CaptureMaterializationEvidence,
) -> color_eyre::Result<Uuid> {
	let write_policy = item
		.capture
		.write_policy
		.as_ref()
		.map(|policy| write_policy_from_value(policy, item.evidence_id.as_str()))
		.transpose()?;
	let response = service
		.add_note(AddNoteRequest {
			tenant_id: TENANT_ID.to_string(),
			project_id: project_id.to_string(),
			agent_id: AGENT_ID.to_string(),
			scope: SCOPE.to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some(key),
				text,
				structured: None,
				importance: 0.9,
				confidence: 0.95,
				ttl_days: None,
				source_ref: serde_json::json!({
					"schema": "real_world_live_adapter/v1",
					"adapter": adapter_id,
					"job_id": loaded.job.job_id,
					"evidence_id": item.evidence_id,
					"source_id": item.capture.source_id.as_deref(),
					"capture_action": capture_action_str(item.capture.action),
					"evidence_binding": item.capture.evidence_binding.as_deref(),
					"write_policy_applied": item.capture.write_policy.is_some(),
					"chunk_index": chunk_index,
					"chunk_count": chunk_count,
				}),
				write_policy,
			}],
		})
		.await
		.map_err(|err| eyre::eyre!("ELF add_note failed for {}: {err}", loaded.job.job_id))?;

	for result in &response.results {
		if let Some(audit) = &result.write_policy_audit
			&& (!audit.exclusions.is_empty() || !audit.redactions.is_empty())
		{
			capture.write_policy_audit_count += 1;
			capture.write_policy_exclusion_count += audit.exclusions.len();
			capture.write_policy_redaction_count += audit.redactions.len();
		}
	}

	response.results.iter().find_map(|result| result.note_id).ok_or_else(|| {
		eyre::eyre!(
			"ELF add_note did not persist evidence {} chunk {} for {}.",
			item.evidence_id,
			chunk_index,
			loaded.job.job_id
		)
	})
}
