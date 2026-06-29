use super::*;

impl ElfService {
	/// Runs L0 document retrieval with access filtering and optional explain output.
	pub async fn docs_search_l0(&self, req: DocsSearchL0Request) -> Result<DocsSearchL0Response> {
		let trace_id = Uuid::new_v4();
		let filters = validate_docs_search_l0(&req)?;
		let mut prepared = self.prepare_docs_search_l0_request(&req, &filters).await?;
		let scored = run_doc_fusion_query(
			&self.qdrant.client,
			self.cfg.storage.qdrant.docs_collection.as_str(),
			req.query.as_str(),
			&prepared.vector,
			&prepared.filter,
			prepared.sparse_mode,
			prepared.candidate_k,
		)
		.await?;

		self.record_docs_search_l0_vector_stats(
			&mut prepared.trajectory,
			&scored,
			prepared.sparse_enabled,
			prepared.sparse_mode,
		);

		let scored_chunks =
			docs_search_l0_deduplicated_chunks(&scored, prepared.candidate_k as usize)?;
		let chunk_ids: Vec<Uuid> = scored_chunks.iter().map(|(chunk_id, _)| *chunk_id).collect();
		let rows = self
			.load_doc_search_rows(&req, &prepared.status, &chunk_ids, &mut prepared.trajectory)
			.await?;
		let mut items = self.build_docs_search_l0_items(
			&req,
			&scored_chunks,
			&rows,
			&prepared.allowed_scopes,
			&prepared.shared_grants,
			&mut prepared.trajectory,
		);

		apply_doc_recency_boost(
			&mut items,
			prepared.now,
			self.cfg.ranking.recency_tau_days,
			self.cfg.ranking.tie_breaker_weight,
		);

		items.sort_by(|a, b| b.score.total_cmp(&a.score));
		items.truncate(prepared.top_k as usize);

		record_result_projection_stage(
			&mut prepared.trajectory,
			rows.len(),
			items.len(),
			self.cfg.ranking.recency_tau_days,
			self.cfg.ranking.tie_breaker_weight,
		);

		Ok(DocsSearchL0Response {
			trace_id,
			items,
			trajectory: prepared.trajectory.into_trajectory(),
		})
	}

	async fn load_doc_search_rows(
		&self,
		req: &DocsSearchL0Request,
		status: &str,
		chunk_ids: &[Uuid],
		trajectory: &mut DocTrajectoryBuilder,
	) -> Result<HashMap<Uuid, DocSearchRow>> {
		let rows = load_doc_search_rows(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			status,
			chunk_ids,
		)
		.await?;

		trajectory.push(
			"chunk_lookup",
			serde_json::json!({
				"requested_chunks": chunk_ids.len(),
				"loaded_chunks": rows.len(),
			}),
		);

		Ok(rows)
	}

	fn build_docs_search_l0_items(
		&self,
		req: &DocsSearchL0Request,
		scored_chunks: &[(Uuid, f32)],
		rows: &HashMap<Uuid, DocSearchRow>,
		allowed_scopes: &[String],
		shared_grants: &HashSet<SharedSpaceGrantKey>,
		trajectory: &mut DocTrajectoryBuilder,
	) -> Vec<DocsSearchL0Item> {
		let items = docs_search_l0_project_items(
			scored_chunks,
			rows,
			req.caller_agent_id.as_str(),
			allowed_scopes,
			shared_grants,
		);

		trajectory.push(
			"dedupe",
			serde_json::json!({
				"raw_candidates": scored_chunks.len(),
				"deduped_candidates": items.len(),
			}),
		);

		items
	}

	async fn prepare_docs_search_l0_request(
		&self,
		req: &DocsSearchL0Request,
		filters: &DocsSearchL0Filters,
	) -> Result<DocsSearchL0Prepared> {
		let explain = req.explain.unwrap_or(false);
		let top_k = req.top_k.unwrap_or(12).min(MAX_TOP_K);
		let candidate_k = req.candidate_k.unwrap_or(60).min(MAX_CANDIDATE_K);
		let sparse_mode = filters.sparse_mode;
		let sparse_enabled = docs_search_sparse_enabled(sparse_mode, req.query.as_str());
		let now = OffsetDateTime::now_utc();
		let mut trajectory = DocTrajectoryBuilder::new(explain);

		trajectory.push(
			"request_validation",
			serde_json::json!({
				"query_len": req.query.len(),
				"top_k": top_k,
				"candidate_k": candidate_k,
				"sparse_mode": sparse_mode.as_str(),
				"doc_type": filters
					.doc_type
					.as_ref()
				.map(|doc_type| doc_type.as_str())
				.unwrap_or("<default>"),
				"status": &filters.status,
			}),
		);

		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.as_str())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.caller_agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;
		let filter = build_doc_search_filter(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.caller_agent_id.as_str(),
			&allowed_scopes,
			filters,
		);
		let embedded = self
			.providers
			.embedding
			.embed(&self.cfg.providers.embedding, slice::from_ref(&req.query))
			.await?;

		trajectory.push("query_embedding", serde_json::json!({ "provider": "embedding" }));

		let vector = embedded.first().ok_or_else(|| Error::Provider {
			message: "Embedding provider returned no vectors.".to_string(),
		})?;

		trajectory.push(
			"vector_dimension_check",
			serde_json::json!({
				"provided_dim": vector.len(),
				"expected_dim": self.cfg.storage.qdrant.vector_dim as usize,
			}),
		);

		if vector.len() != self.cfg.storage.qdrant.vector_dim as usize {
			return Err(Error::Provider {
				message: "Embedding vector dimension mismatch.".to_string(),
			});
		}

		Ok(DocsSearchL0Prepared {
			top_k,
			candidate_k,
			sparse_mode,
			sparse_enabled,
			now,
			trajectory,
			allowed_scopes,
			shared_grants,
			filter,
			vector: vector.to_vec(),
			status: filters.status.clone(),
		})
	}

	fn record_docs_search_l0_vector_stats(
		&self,
		trajectory: &mut DocTrajectoryBuilder,
		scored: &[ScoredPoint],
		sparse_enabled: bool,
		sparse_mode: DocsSparseMode,
	) {
		let channels = if sparse_enabled { vec!["dense", "sparse"] } else { vec!["dense"] };

		trajectory.push(
			"vector_search",
			serde_json::json!({
				"raw_points": scored.len(),
				"sparse_mode": sparse_mode.as_str(),
				"channels": channels,
			}),
		);
	}
}
