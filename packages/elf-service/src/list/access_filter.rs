use std::collections::HashSet;

use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{
	Result,
	access::{self},
	list::ListItem,
};
use elf_storage::models::MemoryNote;

pub(super) fn map_list_items(
	notes: Vec<MemoryNote>,
	agent_id: &str,
	non_private_scopes: Option<&[String]>,
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
	status_for_note_read: bool,
	now: OffsetDateTime,
) -> Vec<ListItem> {
	notes
		.into_iter()
		.filter(|note| {
			let Some(scopes) = non_private_scopes else {
				return true;
			};

			if status_for_note_read {
				return access::note_read_allowed(note, agent_id, scopes, shared_grants, now);
			}

			note.agent_id == agent_id
				|| shared_grants.contains(&crate::access::SharedSpaceGrantKey {
					scope: note.scope.clone(),
					space_owner_agent_id: note.agent_id.clone(),
				})
		})
		.map(|note| ListItem {
			note_id: note.note_id,
			r#type: note.r#type,
			key: note.key,
			scope: note.scope,
			status: note.status,
			text: note.text,
			importance: note.importance,
			confidence: note.confidence,
			updated_at: note.updated_at,
			expires_at: note.expires_at,
			source_ref: note.source_ref,
		})
		.collect()
}

pub(super) async fn list_shared_grants(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	non_private_scopes: &Option<Vec<String>>,
) -> Result<HashSet<access::SharedSpaceGrantKey>> {
	if non_private_scopes.is_none() || agent_id.is_empty() {
		return Ok(HashSet::new());
	}

	let org_shared_allowed =
		non_private_scopes.as_ref().is_some_and(|scopes| scopes.iter().any(|s| s == "org_shared"));

	access::load_shared_read_grants_with_org_shared(
		pool,
		tenant_id,
		project_id,
		agent_id,
		org_shared_allowed,
	)
	.await
}
