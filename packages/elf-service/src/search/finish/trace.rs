use super::super::*;

impl ElfService {
	pub(in crate::search) async fn build_items_and_write_trace(
		&self,
		args: BuildTraceArgs<'_>,
	) -> Result<(Vec<SearchItem>, SearchTrajectorySummary)> {
		let trace_id = args.trace_id;
		let (items, trajectory_summary, trace_payload) = self.build_items_and_trace_payload(args);

		self.write_trace_payload(trace_id, trace_payload).await?;

		Ok((items, trajectory_summary))
	}

	pub(in crate::search) fn build_trace_candidates(
		&self,
		scored: &[ScoredChunk],
		now: OffsetDateTime,
	) -> Vec<TraceCandidateRecord> {
		if !self.cfg.search.explain.capture_candidates || scored.is_empty() {
			return Vec::new();
		}

		let candidate_expires_at =
			now + Duration::days(self.cfg.search.explain.candidate_retention_days);

		scored
			.iter()
			.map(|scored_chunk| {
				build_trace_candidate_record(scored_chunk, now, candidate_expires_at)
			})
			.collect()
	}

	pub(in crate::search) fn build_items_and_trace_payload(
		&self,
		args: BuildTraceArgs<'_>,
	) -> (Vec<SearchItem>, SearchTrajectorySummary, TracePayload) {
		let mut trajectory_stages = build_trace_trajectory_stages(&args);
		let trace_context = TraceContext {
			trace_id: args.trace_id,
			tenant_id: args.tenant_id,
			project_id: args.project_id,
			agent_id: args.agent_id,
			read_profile: args.read_profile,
			query: args.query,
			expansion_mode: args.expansion_mode,
			expanded_queries: args.expanded_queries.clone(),
			allowed_scopes: args.allowed_scopes,
			candidate_count: args.candidate_count,
			top_k: args.top_k,
		};
		let mut config_snapshot = ranking::build_config_snapshot(
			&self.cfg,
			&args.policies.blend_policy,
			&args.policies.diversity_policy,
			&args.policies.retrieval_sources_policy,
			args.ranking_override.as_ref(),
			args.policies.policy_id.as_str(),
			&args.policies.policy_snapshot,
		);

		if let Some(object) = config_snapshot.as_object_mut() {
			object.insert("audit".to_string(), build_trace_audit(args.agent_id, args.token_id));
		}

		let mut items = Vec::with_capacity(args.selected_results.len());
		let mut trace_builder = SearchTraceBuilder::new(
			trace_context,
			config_snapshot,
			self.cfg.search.explain.retention_days,
			args.now,
		);
		let mut final_stage_items = Vec::new();

		for candidate in args.trace_candidates {
			trace_builder.push_candidate(candidate);
		}
		for (idx, scored_chunk) in args.selected_results.into_iter().enumerate() {
			let rank = idx as u32 + 1;
			let (item, trace_item) = build_search_item_and_trace_item(BuildSearchItemArgs {
				cfg: &self.cfg,
				policy_id: args.policies.policy_id.as_str(),
				blend_policy: &args.policies.blend_policy,
				diversity_policy: &args.policies.diversity_policy,
				diversity_decisions: args.diversity_decisions,
				query_tokens: args.query_tokens,
				structured_matches: args.structured_matches,
				relation_contexts: &args.relation_contexts,
				scored_chunk,
				rank,
			});
			let item = apply_payload_level_to_search_item(item, args.payload_level);

			final_stage_items.push(TraceTrajectoryStageItemRecord {
				id: Uuid::new_v4(),
				item_id: Some(item.result_handle),
				note_id: Some(item.note_id),
				chunk_id: Some(item.chunk_id),
				metrics: serde_json::json!({
					"rank": rank,
					"final_score": item.final_score,
				}),
			});
			items.push(item);
			trace_builder.push_item(trace_item);
		}

		if let Some(stage) =
			trajectory_stages.iter_mut().find(|stage| stage.stage_name == "selection.final")
		{
			stage.items = final_stage_items;
		}

		let trajectory_summary = build_trajectory_summary_from_stages(
			&trajectory_stages
				.iter()
				.map(|stage| SearchTrajectoryStage {
					stage_order: stage.stage_order,
					stage_name: stage.stage_name.clone(),
					stage_payload: stage.stage_payload.clone(),
					items: stage
						.items
						.iter()
						.map(|item| SearchTrajectoryStageItem {
							item_id: item.item_id,
							note_id: item.note_id,
							chunk_id: item.chunk_id,
							metrics: item.metrics.clone(),
						})
						.collect(),
				})
				.collect::<Vec<_>>(),
		);

		for stage in trajectory_stages {
			trace_builder.push_stage(stage);
		}

		(items, trajectory_summary, trace_builder.build())
	}

	pub(in crate::search) async fn write_trace_payload(
		&self,
		trace_id: Uuid,
		trace_payload: TracePayload,
	) -> Result<()> {
		match self.cfg.search.explain.write_mode.trim().to_ascii_lowercase().as_str() {
			"inline" => {
				let mut tx = self.db.pool.begin().await?;

				persist_trace_inline(&mut tx, trace_payload).await?;

				tx.commit().await?;
			},
			_ =>
				if let Err(err) = enqueue_trace(&self.db.pool, trace_payload).await {
					tracing::error!(
						error = %err,
						trace_id = %trace_id,
						"Failed to enqueue search trace."
					);
				},
		}

		Ok(())
	}
}
