use super::super::*;

#[derive(Clone, Debug)]
pub(in crate::search) struct NoteMeta {
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) note_type: String,
	pub(in crate::search) key: Option<String>,
	pub(in crate::search) scope: String,
	pub(in crate::search) agent_id: String,
	pub(in crate::search) importance: f32,
	pub(in crate::search) confidence: f32,
	pub(in crate::search) updated_at: OffsetDateTime,
	pub(in crate::search) expires_at: Option<OffsetDateTime>,
	pub(in crate::search) source_ref: Value,
	pub(in crate::search) embedding_version: String,
	pub(in crate::search) hit_count: i64,
	pub(in crate::search) last_hit_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct ChunkRow {
	pub(in crate::search) chunk_id: Uuid,
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) chunk_index: i32,
	pub(in crate::search) start_offset: i32,
	pub(in crate::search) end_offset: i32,
	pub(in crate::search) text: String,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct NoteVectorRow {
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) vec_text: String,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct SearchExplainTraceRow {
	pub(in crate::search) trace_id: Uuid,
	pub(in crate::search) tenant_id: String,
	pub(in crate::search) project_id: String,
	pub(in crate::search) agent_id: String,
	pub(in crate::search) read_profile: String,
	pub(in crate::search) query: String,
	pub(in crate::search) expansion_mode: String,
	pub(in crate::search) expanded_queries: Value,
	pub(in crate::search) allowed_scopes: Value,
	pub(in crate::search) candidate_count: i32,
	pub(in crate::search) top_k: i32,
	pub(in crate::search) config_snapshot: Value,
	pub(in crate::search) trace_version: i32,
	pub(in crate::search) created_at: OffsetDateTime,
	pub(in crate::search) item_id: Uuid,
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) chunk_id: Option<Uuid>,
	pub(in crate::search) rank: i32,
	pub(in crate::search) explain: Value,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct SearchRelationContextRow {
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) fact_id: Uuid,
	pub(in crate::search) scope: String,
	pub(in crate::search) subject_canonical: Option<String>,
	pub(in crate::search) subject_kind: Option<String>,
	pub(in crate::search) predicate: String,
	pub(in crate::search) object_entity_id: Option<Uuid>,
	pub(in crate::search) object_canonical: Option<String>,
	pub(in crate::search) object_kind: Option<String>,
	pub(in crate::search) object_value: Option<String>,
	pub(in crate::search) valid_from: OffsetDateTime,
	pub(in crate::search) valid_to: Option<OffsetDateTime>,
	pub(in crate::search) is_current: bool,
	pub(in crate::search) evidence_note_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct SearchTraceRow {
	pub(in crate::search) trace_id: Uuid,
	pub(in crate::search) tenant_id: String,
	pub(in crate::search) project_id: String,
	pub(in crate::search) agent_id: String,
	pub(in crate::search) read_profile: String,
	pub(in crate::search) query: String,
	pub(in crate::search) expansion_mode: String,
	pub(in crate::search) expanded_queries: Value,
	pub(in crate::search) allowed_scopes: Value,
	pub(in crate::search) candidate_count: i32,
	pub(in crate::search) top_k: i32,
	pub(in crate::search) config_snapshot: Value,
	pub(in crate::search) trace_version: i32,
	pub(in crate::search) created_at: OffsetDateTime,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct SearchTraceItemRow {
	pub(in crate::search) item_id: Uuid,
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) chunk_id: Option<Uuid>,
	pub(in crate::search) rank: i32,
	pub(in crate::search) explain: Value,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct SearchRecentTraceRow {
	pub(in crate::search) trace_id: Uuid,
	pub(in crate::search) tenant_id: String,
	pub(in crate::search) project_id: String,
	pub(in crate::search) agent_id: String,
	pub(in crate::search) read_profile: String,
	pub(in crate::search) query: String,
	pub(in crate::search) created_at: OffsetDateTime,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct TraceCandidateSnapshotRow {
	pub(in crate::search) candidate_snapshot: Value,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct StructuredFieldHitRow {
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) field_kind: String,
}

#[derive(Clone, Debug, FromRow)]
pub(in crate::search) struct BestChunkForNoteRow {
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) chunk_id: Uuid,
	pub(in crate::search) chunk_index: i32,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct ChunkMeta {
	pub(in crate::search) chunk_id: Uuid,
	pub(in crate::search) chunk_index: i32,
	pub(in crate::search) start_offset: i32,
	pub(in crate::search) end_offset: i32,
}

#[derive(Clone, Debug)]
pub(in crate::search) struct ChunkSnippet {
	pub(in crate::search) note: NoteMeta,
	pub(in crate::search) chunk: ChunkMeta,
	pub(in crate::search) snippet: String,
	pub(in crate::search) retrieval_rank: u32,
	pub(in crate::search) retrieval_score: Option<f32>,
}
