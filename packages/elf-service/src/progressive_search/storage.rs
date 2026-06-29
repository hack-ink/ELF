use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
	str::FromStr,
};

use serde_json;
use sqlx::PgExecutor;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use super::types::{
	HitItem, NewSearchSession, SESSION_ABSOLUTE_TTL_HOURS, SESSION_SLIDING_TTL_HOURS,
	SearchSession, SearchSessionItemRecord, SearchSessionMode, SearchSessionRow,
};
use elf_domain::english_gate;

fn hash_query(query: &str) -> String {
	let mut hasher = DefaultHasher::new();

	Hash::hash(query, &mut hasher);

	format!("{:x}", hasher.finish())
}

pub(super) async fn store_search_session<'e, E>(
	executor: E,
	session: NewSearchSession<'_>,
) -> crate::Result<()>
where
	E: PgExecutor<'e>,
{
	let items_json = serde_json::to_value(session.items).map_err(|err| crate::Error::Storage {
		message: format!("Failed to encode search session items: {err}"),
	})?;
	let query_plan_json =
		session.query_plan.map(serde_json::to_value).transpose().map_err(|err| {
			crate::Error::Storage {
				message: format!("Failed to encode search session query plan: {err}"),
			}
		})?;
	let trajectory_summary_json =
		session.trajectory_summary.map(serde_json::to_value).transpose().map_err(|err| {
			crate::Error::Storage {
				message: format!("Failed to encode search session trajectory summary: {err}"),
			}
		})?;

	sqlx::query(
		"\
INSERT INTO search_sessions (
	search_session_id,
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	mode,
	trajectory_summary,
	query_plan,
	items,
	created_at,
	expires_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
	)
	.bind(session.search_session_id)
	.bind(session.trace_id)
	.bind(session.tenant_id.trim())
	.bind(session.project_id.trim())
	.bind(session.agent_id.trim())
	.bind(session.read_profile)
	.bind(session.query)
	.bind(session.mode.as_str())
	.bind(trajectory_summary_json)
	.bind(query_plan_json)
	.bind(items_json)
	.bind(session.created_at)
	.bind(session.expires_at)
	.execute(executor)
	.await?;

	Ok(())
}

pub(super) async fn load_search_session<'e, E>(
	executor: E,
	search_session_id: Uuid,
	now: OffsetDateTime,
) -> crate::Result<SearchSession>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, SearchSessionRow>(
		"\
SELECT
	search_session_id,
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	mode,
	trajectory_summary,
	query_plan,
	items,
	created_at,
	expires_at
FROM search_sessions
WHERE search_session_id = $1",
	)
	.bind(search_session_id)
	.fetch_optional(executor)
	.await?;
	let Some(row) = row else {
		return Err(crate::Error::InvalidRequest {
			message: "Unknown search_session_id.".to_string(),
		});
	};
	let expires_at: OffsetDateTime = row.expires_at;

	if expires_at <= now {
		return Err(crate::Error::InvalidRequest {
			message: "Search session expired.".to_string(),
		});
	}

	let items: Vec<SearchSessionItemRecord> = serde_json::from_value(row.items).map_err(|err| {
		crate::Error::Storage { message: format!("Failed to decode search session items: {err}") }
	})?;
	let mode = SearchSessionMode::from_str(row.mode.as_str())?;
	let query_plan = match row.query_plan {
		Some(value) =>
			Some(serde_json::from_value(value).map_err(|err| crate::Error::Storage {
				message: format!("Failed to decode search session query_plan: {err}"),
			})?),
		None => None,
	};
	let trajectory_summary = match row.trajectory_summary {
		Some(value) =>
			Some(serde_json::from_value(value).map_err(|err| crate::Error::Storage {
				message: format!("Failed to decode search session trajectory summary: {err}"),
			})?),
		None => None,
	};

	Ok(SearchSession {
		search_session_id: row.search_session_id,
		trace_id: row.trace_id,
		tenant_id: row.tenant_id,
		project_id: row.project_id,
		agent_id: row.agent_id,
		read_profile: row.read_profile,
		query: row.query,
		items,
		mode,
		trajectory_summary,
		query_plan,
		created_at: row.created_at,
		expires_at,
	})
}

pub(super) async fn touch_search_session<'e, E>(
	executor: E,
	session: &SearchSession,
	now: OffsetDateTime,
) -> crate::Result<OffsetDateTime>
where
	E: PgExecutor<'e>,
{
	let absolute_expires_at = session.created_at + Duration::hours(SESSION_ABSOLUTE_TTL_HOURS);
	let sliding_expires_at = now + Duration::hours(SESSION_SLIDING_TTL_HOURS);
	let touched = if sliding_expires_at < absolute_expires_at {
		sliding_expires_at
	} else {
		absolute_expires_at
	};

	if touched <= session.expires_at {
		return Ok(session.expires_at);
	}

	sqlx::query(
		"UPDATE search_sessions SET expires_at = $1 WHERE search_session_id = $2 AND expires_at < $1",
	)
	.bind(touched)
	.bind(session.search_session_id)
	.execute(executor)
	.await?;

	Ok(touched)
}

pub(super) async fn record_detail_hits<'e, E>(
	executor: E,
	query: &str,
	items: &[HitItem],
	now: OffsetDateTime,
) -> crate::Result<()>
where
	E: PgExecutor<'e>,
{
	if !english_gate::is_english_natural_language(query) {
		return Err(crate::Error::NonEnglishInput { field: "$.query".to_string() });
	}

	let query_hash = hash_query(query);
	let mut hit_ids = Vec::with_capacity(items.len());
	let mut note_ids = Vec::with_capacity(items.len());
	let mut chunk_ids = Vec::with_capacity(items.len());
	let mut ranks = Vec::with_capacity(items.len());
	let mut final_scores = Vec::with_capacity(items.len());

	for item in items {
		let rank = i32::try_from(item.rank).map_err(|_| crate::Error::InvalidRequest {
			message: "Search session rank is out of range.".to_string(),
		})?;

		hit_ids.push(Uuid::new_v4());
		note_ids.push(item.note_id);
		chunk_ids.push(item.chunk_id);
		ranks.push(rank);
		final_scores.push(item.final_score);
	}

	sqlx::query(
		"\
WITH hits AS (
	SELECT *
	FROM unnest(
	$1::uuid[],
	$2::uuid[],
	$3::uuid[],
	$4::int4[],
	$5::real[]
) AS t(hit_id, note_id, chunk_id, rank, final_score)
),
updated AS (
UPDATE memory_notes
SET
	hit_count = hit_count + 1,
	last_hit_at = $6
WHERE note_id = ANY($2)
)
INSERT INTO memory_hits (
	hit_id,
	note_id,
	chunk_id,
	query_hash,
	rank,
	final_score,
	ts
)
SELECT
	hit_id,
	note_id,
	chunk_id,
	$7,
	rank,
	final_score,
	$6
FROM hits",
	)
	.bind(&hit_ids)
	.bind(&note_ids)
	.bind(&chunk_ids)
	.bind(&ranks)
	.bind(&final_scores)
	.bind(now)
	.bind(query_hash.as_str())
	.execute(executor)
	.await?;

	Ok(())
}
