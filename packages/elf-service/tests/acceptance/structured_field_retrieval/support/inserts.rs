use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

pub(crate) async fn insert_note<'e, E>(
	executor: E,
	note_id: Uuid,
	note_text: &str,
	embedding_version: &str,
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
	.bind("agent_private")
	.bind("fact")
	.bind(Option::<String>::None)
	.bind(note_text)
	.bind(0.4_f32)
	.bind(0.9_f32)
	.bind("active")
	.bind(now)
	.bind(now)
	.bind(Option::<OffsetDateTime>::None)
	.bind(embedding_version)
	.bind(serde_json::json!({}))
	.bind(0_i64)
	.bind(Option::<OffsetDateTime>::None)
	.execute(executor)
	.await
	.expect("Failed to insert memory note.");
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_chunk<'e, E>(
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

pub(crate) async fn insert_chunk_embedding<'e, E>(
	executor: E,
	chunk_id: Uuid,
	embedding_version: &str,
) where
	E: PgExecutor<'e>,
{
	let vec_text = vec_text_zeros();

	sqlx::query(
		"\
INSERT INTO note_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::text::vector)",
	)
	.bind(chunk_id)
	.bind(embedding_version)
	.bind(4_096_i32)
	.bind(vec_text.as_str())
	.execute(executor)
	.await
	.expect("Failed to insert chunk embedding.");
}

pub(crate) async fn insert_fact_field_row<'e, E>(
	executor: E,
	field_id: Uuid,
	note_id: Uuid,
	fact_text: &str,
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
	.bind("fact")
	.bind(0_i32)
	.bind(fact_text)
	.execute(executor)
	.await
	.expect("Failed to insert note field.");
}

pub(crate) async fn insert_fact_field_embedding<'e, E>(
	executor: E,
	field_id: Uuid,
	embedding_version: &str,
) where
	E: PgExecutor<'e>,
{
	let vec_text = vec_text_zeros();

	sqlx::query(
		"\
INSERT INTO note_field_embeddings (field_id, embedding_version, embedding_dim, vec)
VALUES ($1, $2, $3, $4::text::vector)",
	)
	.bind(field_id)
	.bind(embedding_version)
	.bind(4_096_i32)
	.bind(vec_text.as_str())
	.execute(executor)
	.await
	.expect("Failed to insert field embedding.");
}

fn vec_text_zeros() -> String {
	let mut buf = String::with_capacity(2 + (4_096 * 2));

	buf.push('[');

	for i in 0..4_096 {
		if i > 0 {
			buf.push(',');
		}

		buf.push('0');
	}

	buf.push(']');

	buf
}
