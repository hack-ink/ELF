use std::str::FromStr;

use serde_json;
use sqlx::PgExecutor;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	Error, Result,
	progressive_search::types::{
		SearchSessionMode,
		session::{
			NewSearchSession, SESSION_ABSOLUTE_TTL_HOURS, SESSION_SLIDING_TTL_HOURS, SearchSession,
			SearchSessionItemRecord, SearchSessionRow,
		},
	},
};

pub(crate) async fn store_search_session<'e, E>(
	executor: E,
	session: NewSearchSession<'_>,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let items_json = serde_json::to_value(session.items).map_err(|err| Error::Storage {
		message: format!("Failed to encode search session items: {err}"),
	})?;
	let query_plan_json =
		session.query_plan.map(serde_json::to_value).transpose().map_err(|err| Error::Storage {
			message: format!("Failed to encode search session query plan: {err}"),
		})?;
	let trajectory_summary_json =
		session.trajectory_summary.map(serde_json::to_value).transpose().map_err(|err| {
			Error::Storage {
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

pub(crate) async fn load_search_session<'e, E>(
	executor: E,
	search_session_id: Uuid,
	now: OffsetDateTime,
) -> Result<SearchSession>
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
		return Err(Error::InvalidRequest { message: "Unknown search_session_id.".to_string() });
	};
	let expires_at: OffsetDateTime = row.expires_at;

	if expires_at <= now {
		return Err(Error::InvalidRequest { message: "Search session expired.".to_string() });
	}

	let items: Vec<SearchSessionItemRecord> = serde_json::from_value(row.items).map_err(|err| {
		Error::Storage { message: format!("Failed to decode search session items: {err}") }
	})?;
	let mode = SearchSessionMode::from_str(row.mode.as_str())?;
	let query_plan = match row.query_plan {
		Some(value) => Some(serde_json::from_value(value).map_err(|err| Error::Storage {
			message: format!("Failed to decode search session query_plan: {err}"),
		})?),
		None => None,
	};
	let trajectory_summary = match row.trajectory_summary {
		Some(value) => Some(serde_json::from_value(value).map_err(|err| Error::Storage {
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

pub(crate) async fn touch_search_session<'e, E>(
	executor: E,
	session: &SearchSession,
	now: OffsetDateTime,
) -> Result<OffsetDateTime>
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
