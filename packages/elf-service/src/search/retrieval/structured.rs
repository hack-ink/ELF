use crate::search::{
	self, BestChunkForNoteRow, ElfService, FieldHit, HashMap, ORG_PROJECT_ID, Result,
	StructuredFieldHitArgs, StructuredFieldHitRow, StructuredFieldRetrievalArgs,
	StructuredFieldRetrievalResult, Uuid,
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

	async fn fetch_structured_field_hits(
		&self,
		args: StructuredFieldHitArgs<'_>,
	) -> Result<Vec<FieldHit>> {
		if args.private_allowed && args.non_private_scopes.is_empty() {
			self.fetch_structured_field_hits_private_only(args).await
		} else if !args.private_allowed {
			self.fetch_structured_field_hits_non_private_only(args).await
		} else {
			self.fetch_structured_field_hits_mixed(args).await
		}
	}

	async fn fetch_structured_field_hits_private_only(
		&self,
		args: StructuredFieldHitArgs<'_>,
	) -> Result<Vec<FieldHit>> {
		let rows = sqlx::query_as::<_, StructuredFieldHitRow>(
			"\
SELECT
	f.note_id,
	f.field_kind
FROM memory_note_fields f
JOIN note_field_embeddings e
	ON e.field_id = f.field_id
	AND e.embedding_version = $1
JOIN memory_notes n
	ON n.note_id = f.note_id
WHERE n.tenant_id = $2
	AND n.project_id = $3
	AND n.status = 'active'
	AND (n.expires_at IS NULL OR n.expires_at > $4)
	AND n.scope = 'agent_private'
	AND n.agent_id = $5
ORDER BY e.vec <=> $6::text::vector ASC
LIMIT $7",
		)
		.bind(args.embed_version)
		.bind(args.tenant_id)
		.bind(args.project_id)
		.bind(args.now)
		.bind(args.agent_id)
		.bind(args.vec_text)
		.bind(args.retrieval_limit)
		.fetch_all(&self.db.pool)
		.await?;

		Ok(rows
			.into_iter()
			.map(|row| FieldHit { note_id: row.note_id, field_kind: row.field_kind })
			.collect())
	}

	async fn fetch_structured_field_hits_non_private_only(
		&self,
		args: StructuredFieldHitArgs<'_>,
	) -> Result<Vec<FieldHit>> {
		let rows = sqlx::query_as::<_, StructuredFieldHitRow>(
			"\
SELECT
	f.note_id,
	f.field_kind
FROM memory_note_fields f
JOIN note_field_embeddings e
	ON e.field_id = f.field_id
	AND e.embedding_version = $1
JOIN memory_notes n
	ON n.note_id = f.note_id
WHERE n.tenant_id = $2
	AND (n.project_id = $3 OR (n.project_id = $8 AND n.scope = 'org_shared'))
	AND n.status = 'active'
	AND (n.expires_at IS NULL OR n.expires_at > $4)
	AND n.scope = ANY($5::text[])
ORDER BY e.vec <=> $6::text::vector ASC
LIMIT $7",
		)
		.bind(args.embed_version)
		.bind(args.tenant_id)
		.bind(args.project_id)
		.bind(args.now)
		.bind(args.non_private_scopes)
		.bind(args.vec_text)
		.bind(args.retrieval_limit)
		.bind(ORG_PROJECT_ID)
		.fetch_all(&self.db.pool)
		.await?;

		Ok(rows
			.into_iter()
			.map(|row| FieldHit { note_id: row.note_id, field_kind: row.field_kind })
			.collect())
	}

	async fn fetch_structured_field_hits_mixed(
		&self,
		args: StructuredFieldHitArgs<'_>,
	) -> Result<Vec<FieldHit>> {
		let rows = sqlx::query_as::<_, StructuredFieldHitRow>(
			"\
SELECT
	f.note_id,
	f.field_kind
FROM memory_note_fields f
JOIN note_field_embeddings e
	ON e.field_id = f.field_id
	AND e.embedding_version = $1
JOIN memory_notes n
	ON n.note_id = f.note_id
WHERE n.tenant_id = $2
	AND (n.project_id = $3 OR (n.project_id = $9 AND n.scope = 'org_shared'))
	AND n.status = 'active'
	AND (n.expires_at IS NULL OR n.expires_at > $4)
	AND (
		(n.scope = 'agent_private' AND n.agent_id = $5)
		OR n.scope = ANY($6::text[])
	)
ORDER BY e.vec <=> $7::text::vector ASC
LIMIT $8",
		)
		.bind(args.embed_version)
		.bind(args.tenant_id)
		.bind(args.project_id)
		.bind(args.now)
		.bind(args.agent_id)
		.bind(args.non_private_scopes)
		.bind(args.vec_text)
		.bind(args.retrieval_limit)
		.bind(ORG_PROJECT_ID)
		.fetch_all(&self.db.pool)
		.await?;

		Ok(rows
			.into_iter()
			.map(|row| FieldHit { note_id: row.note_id, field_kind: row.field_kind })
			.collect())
	}

	async fn fetch_best_chunks_for_notes(
		&self,
		embed_version: &str,
		ordered_note_ids: &[Uuid],
		vec_text: &str,
	) -> Result<HashMap<Uuid, (Uuid, i32)>> {
		let best_chunks = sqlx::query_as::<_, BestChunkForNoteRow>(
			"\
SELECT DISTINCT ON (c.note_id)
	c.note_id,
	c.chunk_id,
	c.chunk_index
FROM memory_note_chunks c
JOIN note_chunk_embeddings e
	ON e.chunk_id = c.chunk_id
	AND e.embedding_version = $1
WHERE c.note_id = ANY($2::uuid[])
ORDER BY c.note_id ASC, e.vec <=> $3::text::vector ASC",
		)
		.bind(embed_version)
		.bind(ordered_note_ids)
		.bind(vec_text)
		.fetch_all(&self.db.pool)
		.await?;
		let mut best_by_note = HashMap::new();

		for row in best_chunks {
			best_by_note.insert(row.note_id, (row.chunk_id, row.chunk_index));
		}

		Ok(best_by_note)
	}
}
