use crate::search::{
	self, ElfService, HashMap, Result, StructuredFieldHitArgs, StructuredFieldRetrievalArgs,
	StructuredFieldRetrievalResult,
};

impl ElfService {
	pub(in crate::search::retrieval) async fn retrieve_structured_field_candidates(
		&self,
		args: StructuredFieldRetrievalArgs<'_>,
	) -> Result<StructuredFieldRetrievalResult> {
		let StructuredFieldRetrievalArgs {
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			query_vec,
			candidate_k,
			now,
		} = args;

		if query_vec.is_empty() {
			return Ok(StructuredFieldRetrievalResult {
				candidates: Vec::new(),
				structured_matches: HashMap::new(),
			});
		}

		let embed_version = crate::embedding_version(&self.cfg);
		let vec_text = crate::vector_to_pg(query_vec);
		let private_allowed = allowed_scopes.iter().any(|scope| scope == "agent_private");
		let non_private_scopes: Vec<String> =
			allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
		let retrieval_limit = i64::from(candidate_k.saturating_mul(4).clamp(16, 400));
		let rows = self
			.fetch_structured_field_hits(StructuredFieldHitArgs {
				embed_version: embed_version.as_str(),
				tenant_id,
				project_id,
				agent_id,
				now,
				vec_text: vec_text.as_str(),
				retrieval_limit,
				private_allowed,
				non_private_scopes: non_private_scopes.as_slice(),
			})
			.await?;
		let (ordered_note_ids, structured_matches_out) =
			search::build_structured_field_matches(rows);

		if ordered_note_ids.is_empty() {
			return Ok(StructuredFieldRetrievalResult {
				candidates: Vec::new(),
				structured_matches: structured_matches_out,
			});
		}

		let best_by_note = self
			.fetch_best_chunks_for_notes(
				embed_version.as_str(),
				ordered_note_ids.as_slice(),
				vec_text.as_str(),
			)
			.await?;
		let structured_candidates = search::build_structured_field_candidates(
			candidate_k,
			ordered_note_ids,
			best_by_note,
			embed_version.as_str(),
		);

		Ok(StructuredFieldRetrievalResult {
			candidates: structured_candidates,
			structured_matches: structured_matches_out,
		})
	}
}
