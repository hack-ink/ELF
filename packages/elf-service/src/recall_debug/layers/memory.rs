use super::*;

impl ElfService {
	pub(super) async fn recall_memory_layer(
		&self,
		req: &RecallDebugPanelRequest,
		limit: u32,
	) -> Result<RecallDebugLayer> {
		let Some(trace_id) = req.trace_id else {
			return Ok(not_requested_layer(
				"memory_notes",
				"Supply trace_id to show selected and dropped Memory Note candidates.",
			));
		};

		if !req.allow_project_trace_debug {
			self.ensure_public_recall_trace_allowed(req, trace_id).await?;
		}

		let bundle = self
			.trace_bundle_get(TraceBundleGetRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				agent_id: req.agent_id.clone(),
				trace_id,
				mode: TraceBundleMode::Bounded,
				stage_items_limit: Some(limit),
				candidates_limit: Some(limit.saturating_mul(4).min(400)),
			})
			.await?;
		let selected_note_ids =
			bundle.items.iter().map(|item| item.note_id).collect::<BTreeSet<_>>();
		let selected_candidate_keys =
			bundle.items.iter().filter_map(search_item_candidate_key).collect::<BTreeSet<_>>();
		let candidate_note_ids =
			bundle.candidates.as_ref().into_iter().flatten().map(|candidate| candidate.note_id);
		let all_note_ids =
			selected_note_ids.iter().copied().chain(candidate_note_ids).collect::<BTreeSet<_>>();
		let source_refs = self
			.load_memory_note_debug_sources(req, all_note_ids.iter().copied().collect())
			.await?;
		let replay_command = format!("elf_admin_trace_bundle_get trace_id={trace_id} mode=bounded");
		let visible_items = bundle
			.items
			.iter()
			.filter(|item| source_refs.contains_key(&item.note_id))
			.collect::<Vec<_>>();
		let dropped_candidates = bundle
			.candidates
			.as_deref()
			.unwrap_or_default()
			.iter()
			.filter(|candidate| !candidate_is_selected(&selected_candidate_keys, candidate))
			.filter(|candidate| source_refs.contains_key(&candidate.note_id))
			.collect::<Vec<_>>();
		let compact_replay = serde_json::json!({
			"compact_replay": memory_compact_replay_artifact(
				&bundle.trace,
				bundle.stages.as_slice(),
				bundle.candidates.as_deref().unwrap_or_default(),
				visible_items.as_slice(),
				&selected_candidate_keys,
				&source_refs,
				replay_command.as_str(),
			),
		});
		let selected_cap = if !dropped_candidates.is_empty() && limit > 1 {
			limit as usize - 1
		} else {
			limit as usize
		};
		let mut rows = Vec::new();

		for item in visible_items.iter().take(selected_cap) {
			let source = source_refs.get(&item.note_id);

			rows.push(RecallDebugRow {
				layer: "memory_notes".to_string(),
				item_ref: serde_json::json!({
					"trace_id": trace_id,
					"result_handle": item.result_handle,
					"note_id": item.note_id,
					"chunk_id": item.chunk_id,
				}),
				selection_state: "selected".to_string(),
				authority_layer: "memory_note".to_string(),
				freshness_state: freshness_from_note_source(source),
				source_refs: source_ref_from_note_source(source),
				score: Some(item.explain.ranking.final_score),
				rank: Some(item.rank),
				rationale: Some("final ranked search result".to_string()),
				stage_reason: last_stage_name(bundle.stages.as_slice())
					.or_else(|| Some("final_ranking".to_string())),
				replay_command: Some(replay_command.clone()),
				evidence_class: "pass".to_string(),
				debug_artifacts: serde_json::json!({
					"ranking_explain": item.explain,
					"note_updated_at": source.map(|row| row.updated_at),
				}),
			});
		}

		let dropped_cap = limit.saturating_sub(rows.len() as u32) as usize;

		for candidate in dropped_candidates.into_iter().take(dropped_cap) {
			rows.push(candidate_debug_row(
				trace_id,
				candidate,
				source_refs.get(&candidate.note_id),
				replay_command.as_str(),
			));
		}

		Ok(layer_from_rows_with_artifacts(
			"memory_notes",
			"pass",
			Some(trace_id.to_string()),
			"Search trace bundle with selected results and replay candidates.",
			rows,
			compact_replay,
		))
	}

	async fn ensure_public_recall_trace_allowed(
		&self,
		req: &RecallDebugPanelRequest,
		trace_id: Uuid,
	) -> Result<()> {
		let row: Option<(i64,)> = sqlx::query_as(
			"\
SELECT 1
FROM search_traces
WHERE trace_id = $1
  AND tenant_id = $2
  AND project_id = $3
  AND agent_id = $4
  AND read_profile = $5",
		)
		.bind(trace_id)
		.bind(req.tenant_id.trim())
		.bind(req.project_id.trim())
		.bind(req.agent_id.trim())
		.bind(req.read_profile.trim())
		.fetch_optional(&self.db.pool)
		.await?;

		if row.is_some() {
			Ok(())
		} else {
			Err(Error::InvalidRequest {
				message: "Unknown trace_id for this recall context.".to_string(),
			})
		}
	}

	async fn load_memory_note_debug_sources(
		&self,
		req: &RecallDebugPanelRequest,
		note_ids: Vec<Uuid>,
	) -> Result<BTreeMap<Uuid, NoteDebugSourceRow>> {
		if note_ids.is_empty() {
			return Ok(BTreeMap::new());
		}

		let rows = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
	WHERE tenant_id = $1
	  AND note_id = ANY($3::uuid[])
	  AND (
	    project_id = $2
	    OR (project_id = $4 AND scope = 'org_shared')
	  )",
		)
		.bind(req.tenant_id.as_str())
		.bind(req.project_id.as_str())
		.bind(note_ids)
		.bind(ORG_PROJECT_ID)
		.fetch_all(&self.db.pool)
		.await?;

		if req.allow_project_trace_debug {
			return Ok(rows.into_iter().map(note_debug_source_pair).collect());
		}

		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.trim())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			req.tenant_id.trim(),
			req.project_id.trim(),
			req.agent_id.trim(),
			org_shared_allowed,
		)
		.await?;
		let now = OffsetDateTime::now_utc();

		Ok(rows
			.into_iter()
			.filter(|note| {
				note_debug_read_allowed(
					note,
					req.agent_id.trim(),
					&allowed_scopes,
					&shared_grants,
					now,
				)
			})
			.map(note_debug_source_pair)
			.collect())
	}
}
