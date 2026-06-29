use super::*;

pub(in crate::work_journal) fn work_journal_read_allowed(
	entry: &WorkJournalEntry,
	requester_agent_id: &str,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
) -> bool {
	if entry.status != "active" {
		return false;
	}
	if !allowed_scopes.iter().any(|scope| scope == &entry.scope) {
		return false;
	}
	if entry.scope == "agent_private" {
		return entry.agent_id == requester_agent_id;
	}
	if entry.agent_id == requester_agent_id {
		return true;
	}

	shared_grants.contains(&SharedSpaceGrantKey {
		scope: entry.scope.clone(),
		space_owner_agent_id: entry.agent_id.clone(),
	})
}

pub(in crate::work_journal) fn row_to_response(
	row: WorkJournalEntry,
) -> Result<WorkJournalEntryResponse> {
	let family = WorkJournalEntryFamily::parse(row.family.as_str())?;
	let redaction_audit = serde_json::from_value::<WritePolicyAudit>(row.redaction_audit.clone())
		.map_err(|err| Error::InvalidRequest {
		message: format!("stored redaction audit is invalid: {err}"),
	})?;

	Ok(WorkJournalEntryResponse {
		schema: ELF_WORK_JOURNAL_SCHEMA_V1.to_string(),
		entry_id: row.entry_id,
		tenant_id: row.tenant_id,
		project_id: row.project_id,
		agent_id: row.agent_id,
		scope: row.scope,
		session_id: row.session_id,
		family,
		status: row.status,
		title: row.title,
		body: row.body,
		source_refs: value_array(row.source_refs),
		explicit_next_steps: string_array(row.explicit_next_steps),
		inferred_next_steps: string_array(row.inferred_next_steps),
		rejected_options: string_array(row.rejected_options),
		promotion_boundary: row.promotion_boundary,
		redaction_audit,
		created_at: row.created_at,
		updated_at: row.updated_at,
	})
}

pub(in crate::work_journal) fn build_where_stopped(
	items: &[WorkJournalEntryResponse],
) -> Option<WorkJournalWhereStopped> {
	let latest = items.first()?;
	let explicit_next_steps = first_non_empty(items.iter().map(|item| &item.explicit_next_steps));
	let inferred_next_steps = first_non_empty(items.iter().map(|item| &item.inferred_next_steps));
	let rejected_options = first_non_empty(items.iter().map(|item| &item.rejected_options));

	Some(WorkJournalWhereStopped {
		latest_entry_id: latest.entry_id,
		latest_family: latest.family,
		source_refs: latest.source_refs.clone(),
		explicit_next_steps,
		inferred_next_steps,
		rejected_options,
		promotion_boundary: latest.promotion_boundary.clone(),
	})
}

pub(in crate::work_journal) async fn load_work_journal_shared_grants(
	service: &ElfService,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	allowed_scopes: &[String],
) -> Result<HashSet<SharedSpaceGrantKey>> {
	let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");

	access::load_shared_read_grants_with_org_shared(
		&service.db.pool,
		tenant_id,
		project_id,
		agent_id,
		org_shared_allowed,
	)
	.await
}

fn value_array(value: Value) -> Vec<Value> {
	match value {
		Value::Array(items) => items,
		_ => Vec::new(),
	}
}

fn string_array(value: Value) -> Vec<String> {
	match value {
		Value::Array(items) =>
			items.into_iter().filter_map(|item| item.as_str().map(str::to_string)).collect(),
		_ => Vec::new(),
	}
}

fn first_non_empty<'a>(mut lists: impl Iterator<Item = &'a Vec<String>>) -> Vec<String> {
	lists.find(|items| !items.is_empty()).cloned().unwrap_or_default()
}
