use sqlx::{PgPool, QueryBuilder};
use time::OffsetDateTime;

use crate::{Result, access::ORG_PROJECT_ID, list::ListRequest};
use elf_storage::models::MemoryNote;

pub(super) async fn list_notes(
	pool: &PgPool,
	req: &ListRequest,
	tenant_id: &str,
	project_id: &str,
	requested_status: Option<&str>,
	agent_id: &str,
	now: OffsetDateTime,
) -> Result<Vec<MemoryNote>> {
	let mut builder = QueryBuilder::new(
		"SELECT note_id, tenant_id, project_id, agent_id, scope, type, key, text, importance, confidence, status, created_at, updated_at, expires_at, embedding_version, source_ref, hit_count, last_hit_at \
					FROM memory_notes WHERE tenant_id = ",
	);

	builder.push_bind(tenant_id);

	let include_org_shared = match req.scope.as_deref().map(str::trim) {
		None => true,
		Some("org_shared") => true,
		Some(_) => false,
	};

	if include_org_shared {
		builder.push(" AND (project_id = ");
		builder.push_bind(project_id);
		builder.push(" OR (project_id = ");
		builder.push_bind(ORG_PROJECT_ID);
		builder.push(" AND scope = ");
		builder.push_bind("org_shared");
		builder.push("))");
	} else {
		builder.push(" AND project_id = ");
		builder.push_bind(project_id);
	}

	if let Some(scope) = &req.scope {
		builder.push(" AND scope = ");
		builder.push_bind(scope);

		if scope == "agent_private" {
			builder.push(" AND agent_id = ");
			builder.push_bind(agent_id);
		}
	} else {
		builder.push(" AND scope != ");
		builder.push_bind("agent_private");
	}
	if let Some(status) = requested_status {
		builder.push(" AND status = ");
		builder.push_bind(status);
	} else {
		builder.push(" AND status = ");
		builder.push_bind("active");
	}

	if requested_status.unwrap_or("active").eq_ignore_ascii_case("active") {
		builder.push(" AND (expires_at IS NULL OR expires_at > ");
		builder.push_bind(now);
		builder.push(")");
	}

	if let Some(note_type) = &req.r#type {
		builder.push(" AND type = ");
		builder.push_bind(note_type);
	}

	builder.build_query_as().fetch_all(pool).await.map_err(Into::into)
}
