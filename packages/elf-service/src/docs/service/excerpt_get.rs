use super::*;

impl ElfService {
	/// Resolves and verifies an excerpt window from quote, position, or chunk selectors.
	pub async fn docs_excerpts_get(
		&self,
		req: DocsExcerptsGetRequest,
	) -> Result<DocsExcerptResponse> {
		let explain = req.explain.unwrap_or(false);
		let trace_id = Uuid::new_v4();
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();
		let read_profile = req.read_profile.trim();
		let mut trajectory = DocTrajectoryBuilder::new(explain);

		trajectory.push(
			"request_validation",
			serde_json::json!({
				"doc_id": req.doc_id,
				"read_profile": read_profile,
			}),
		);

		validate_docs_excerpts_get(
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			req.quote.as_ref(),
		)?;

		let doc = load_docs_excerpt_context(
			&self.cfg,
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			read_profile,
			req.doc_id,
		)
		.await?;
		let level_max = excerpt_level_max(req.level.as_str())?;

		trajectory.push(
			"level_selection",
			serde_json::json!({
				"level": req.level,
				"max_bytes": level_max,
			}),
		);

		let mut verified = true;
		let mut verification_errors = Vec::new();
		let DocExcerptRange {
			selector_kind,
			match_start_offset,
			match_end_offset,
			start_offset,
			end_offset,
		} = docs_excerpts_resolve_windowed_match(
			&self.db.pool,
			&doc,
			&req,
			level_max,
			&mut trajectory,
			&mut verified,
			&mut verification_errors,
		)
		.await?;
		let excerpt = doc.content.get(start_offset..end_offset).unwrap_or("").to_string();

		if excerpt.is_empty() {
			verified = false;

			verification_errors.push("EMPTY_EXCERPT".to_string());
		}

		let excerpt_hash = blake3::hash(excerpt.as_bytes()).to_hex().to_string();

		trajectory.push(
			"verification",
			serde_json::json!({
				"verified": verified,
				"error_count": verification_errors.len(),
			}),
		);

		Ok(DocsExcerptResponse {
			trace_id,
			doc_id: doc.doc_id,
			excerpt,
			start_offset,
			end_offset,
			locator: docs_excerpt_locator(
				&req,
				&selector_kind,
				match_start_offset,
				match_end_offset,
				doc.content_hash.as_str(),
			),
			verification: DocsExcerptVerification {
				verified,
				verification_errors,
				content_hash: doc.content_hash.clone(),
				excerpt_hash,
			},
			trajectory: trajectory.into_trajectory(),
		})
	}
}
