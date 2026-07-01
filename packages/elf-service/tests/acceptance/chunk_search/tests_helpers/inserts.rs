use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

pub(in super::super) async fn insert_note<'e, E>(
	executor: E,
	note_id: Uuid,
	note_text: &str,
	embedding_version: &str,
) where
	E: PgExecutor<'e>,
{
	insert_note_with_importance_and_source_ref(
		executor,
		note_id,
		note_text,
		embedding_version,
		0.4_f32,
		0.9_f32,
		"agent_private",
		serde_json::json!({}),
	)
	.await;
}

pub(in super::super) async fn insert_note_with_importance<'e, E>(
	executor: E,
	note_id: Uuid,
	note_text: &str,
	embedding_version: &str,
	importance: f32,
	confidence: f32,
	scope: &str,
) where
	E: PgExecutor<'e>,
{
	insert_note_with_importance_and_source_ref(
		executor,
		note_id,
		note_text,
		embedding_version,
		importance,
		confidence,
		scope,
		serde_json::json!({}),
	)
	.await;
}

#[allow(clippy::too_many_arguments)]
pub(in super::super) async fn insert_note_with_importance_and_source_ref<'e, E>(
	executor: E,
	note_id: Uuid,
	note_text: &str,
	embedding_version: &str,
	importance: f32,
	confidence: f32,
	scope: &str,
	source_ref: Value,
) where
	E: PgExecutor<'e>,
{
	let now = OffsetDateTime::now_utc();

	sqlx::query(
		"\
INSERT INTO memory_notes (
	note_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	type,
	key,
	text,
	importance,
	confidence,
	status,
	created_at,
	updated_at,
	expires_at,
	embedding_version,
	source_ref,
	hit_count,
	last_hit_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9,
	$10,
	$11,
	$12,
	$13,
	$14,
	$15,
	$16,
	$17,
	$18
)",
	)
	.bind(note_id)
	.bind("t")
	.bind("p")
	.bind("a")
	.bind(scope)
	.bind("fact")
	.bind(Option::<String>::None)
	.bind(note_text)
	.bind(importance)
	.bind(confidence)
	.bind("active")
	.bind(now)
	.bind(now)
	.bind(Option::<OffsetDateTime>::None)
	.bind(embedding_version)
	.bind(source_ref)
	.bind(0_i64)
	.bind(Option::<OffsetDateTime>::None)
	.execute(executor)
	.await
	.expect("Failed to insert memory note.");
}

#[allow(clippy::too_many_arguments)]
pub(in super::super) async fn insert_summary_field_row<'e, E>(
	executor: E,
	field_id: Uuid,
	note_id: Uuid,
	summary: &str,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO memory_note_fields (field_id, note_id, field_kind, item_index, text)
VALUES ($1, $2, $3, $4, $5)",
	)
	.bind(field_id)
	.bind(note_id)
	.bind("summary")
	.bind(0_i32)
	.bind(summary)
	.execute(executor)
	.await
	.expect("Failed to insert note summary field.");
}

#[allow(clippy::too_many_arguments)]
pub(in super::super) async fn insert_chunk<'e, E>(
	executor: E,
	chunk_id: Uuid,
	note_id: Uuid,
	chunk_index: i32,
	start_offset: i32,
	end_offset: i32,
	text: &str,
	embedding_version: &str,
) where
	E: PgExecutor<'e>,
{
	sqlx::query(
		"\
INSERT INTO memory_note_chunks (
	chunk_id,
	note_id,
	chunk_index,
	start_offset,
	end_offset,
	text,
	embedding_version
)
VALUES ($1, $2, $3, $4, $5, $6, $7)",
	)
	.bind(chunk_id)
	.bind(note_id)
	.bind(chunk_index)
	.bind(start_offset)
	.bind(end_offset)
	.bind(text)
	.bind(embedding_version)
	.execute(executor)
	.await
	.expect("Failed to insert chunk metadata.");
}
