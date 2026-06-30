//! Note listing APIs.

mod access_filter;
mod query;
mod request;
mod types;

pub use self::types::{ListItem, ListRequest, ListResponse};

use time::OffsetDateTime;

use crate::{ElfService, Result};

impl ElfService {
	/// Lists notes visible to the caller under the requested filters.
	pub async fn list(&self, req: ListRequest) -> Result<ListResponse> {
		let now = OffsetDateTime::now_utc();
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.as_ref().map(|value| value.trim()).unwrap_or("");
		let requested_status = request::requested_list_status(req.status.as_ref());
		let status_for_note_read =
			requested_status.unwrap_or("active").eq_ignore_ascii_case("active");
		let non_private_scopes = match req.scope.as_deref().map(str::trim) {
			Some("agent_private") => None,
			Some(scope) => Some(vec![scope.to_string()]),
			None => Some(
				self.cfg.scopes.allowed.iter().filter(|s| *s != "agent_private").cloned().collect(),
			),
		};

		request::validate_list_request(
			&req,
			tenant_id,
			project_id,
			agent_id,
			&self.cfg.scopes.allowed,
		)?;

		let shared_grants = access_filter::list_shared_grants(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			&non_private_scopes,
		)
		.await?;
		let notes = query::list_notes(
			&self.db.pool,
			&req,
			tenant_id,
			project_id,
			requested_status,
			agent_id,
			now,
		)
		.await?;
		let items = access_filter::map_list_items(
			notes,
			agent_id,
			non_private_scopes.as_deref(),
			&shared_grants,
			status_for_note_read,
			now,
		);

		Ok(ListResponse { items })
	}
}
