use std::collections::HashSet;

use crate::{
	access,
	core_blocks::rows::{CoreBlockJoinedRow, CoreBlockRow},
};

pub(in crate::core_blocks) fn filter_visible_rows(
	rows: Vec<CoreBlockJoinedRow>,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
) -> Vec<CoreBlockJoinedRow> {
	rows.into_iter()
		.filter(|row| {
			let block = CoreBlockRow {
				block_id: row.block_id,
				tenant_id: row.tenant_id.clone(),
				project_id: row.project_id.clone(),
				agent_id: row.agent_id.clone(),
				scope: row.scope.clone(),
				key: row.key.clone(),
				title: row.title.clone(),
				content: row.content.clone(),
				source_ref: row.source_ref.clone(),
				status: row.status.clone(),
				created_at: row.created_at,
				updated_at: row.updated_at,
			};

			block_read_allowed(
				&block,
				row.attachment_agent_id.as_str(),
				allowed_scopes,
				shared_grants,
			)
		})
		.collect()
}

pub(in crate::core_blocks) fn block_read_allowed(
	block: &CoreBlockRow,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<access::SharedSpaceGrantKey>,
) -> bool {
	if block.status != "active" {
		return false;
	}
	if !allowed_scopes.iter().any(|scope| scope == &block.scope) {
		return false;
	}
	if block.scope == "agent_private" {
		return block.agent_id == requester_agent_id;
	}
	if !matches!(block.scope.as_str(), "project_shared" | "org_shared") {
		return false;
	}
	if block.agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&access::SharedSpaceGrantKey {
		scope: block.scope.clone(),
		space_owner_agent_id: block.agent_id.clone(),
	})
}
