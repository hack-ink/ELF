use super::*;

pub(super) async fn fetch_cache_payload<'e, E>(
	executor: E,
	kind: CacheKind,
	key: &str,
	now: OffsetDateTime,
) -> Result<Option<CachePayload>>
where
	E: PgExecutor<'e>,
{
	let payload: Option<Value> = sqlx::query_scalar(
		"\
WITH updated AS (
	UPDATE llm_cache
	SET
		last_accessed_at = $3,
		hit_count = hit_count + 1
	WHERE
		cache_kind = $1
		AND cache_key = $2
		AND expires_at > $3
	RETURNING payload
)
	SELECT payload
FROM updated",
	)
	.bind(kind.as_str())
	.bind(key)
	.bind(now)
	.fetch_optional(executor)
	.await?;
	let Some(payload) = payload else {
		return Ok(None);
	};
	let size_bytes = serde_json::to_vec(&payload)
		.map_err(|err| crate::Error::Storage {
			message: format!("Failed to encode cache payload: {err}"),
		})?
		.len();

	Ok(Some(CachePayload { value: payload, size_bytes }))
}

pub(super) async fn store_cache_payload<'e, E>(
	executor: E,
	kind: CacheKind,
	key: &str,
	payload: Value,
	now: OffsetDateTime,
	expires_at: OffsetDateTime,
	max_payload_bytes: Option<u64>,
) -> Result<Option<usize>>
where
	E: PgExecutor<'e>,
{
	let payload_bytes = serde_json::to_vec(&payload).map_err(|err| crate::Error::Storage {
		message: format!("Failed to encode cache payload: {err}"),
	})?;
	let payload_size = payload_bytes.len();

	if let Some(max) = max_payload_bytes
		&& payload_size as u64 > max
	{
		return Ok(None);
	}

	sqlx::query(
		"\
	INSERT INTO llm_cache (
	cache_id,
	cache_kind,
	cache_key,
	payload,
	created_at,
	last_accessed_at,
	expires_at,
	hit_count
)
VALUES ($1, $2, $3, $4, $5, $5, $6, 0)
ON CONFLICT (cache_kind, cache_key) DO UPDATE SET
payload = EXCLUDED.payload,
	last_accessed_at = EXCLUDED.last_accessed_at,
	expires_at = EXCLUDED.expires_at,
	hit_count = 0",
	)
	.bind(Uuid::new_v4())
	.bind(kind.as_str())
	.bind(key)
	.bind(payload)
	.bind(now)
	.bind(expires_at)
	.execute(executor)
	.await?;

	Ok(Some(payload_size))
}
