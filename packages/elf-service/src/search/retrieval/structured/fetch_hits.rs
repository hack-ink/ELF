use crate::search::{
	ElfService, FieldHit, ORG_PROJECT_ID, Result, StructuredFieldHitArgs, StructuredFieldHitRow,
};

impl ElfService {
	pub(in crate::search::retrieval) async fn fetch_structured_field_hits(
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
}
