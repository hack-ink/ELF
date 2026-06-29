use crate::{
	access,
	work_journal::validation::{
		self, Config, Error, Map, MemoryNote, ORG_PROJECT_ID, OffsetDateTime, PgConnection, Result,
		Uuid, Value, WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1, consolidation, serde_json,
	},
};

pub(in crate::work_journal) fn normalize_promotion_boundary(input: &Value) -> Result<Value> {
	let map = match input {
		Value::Null => Map::new(),
		Value::Object(map) => map.clone(),
		_ => {
			return Err(Error::InvalidRequest {
				message: "promotion_boundary must be a JSON object.".to_string(),
			});
		},
	};

	validation::validate_json_strings(&Value::Object(map.clone()), "$.promotion_boundary")?;

	let accepted_memory_authority_ref = map.get("accepted_memory_authority_ref").cloned();
	let accepted_dreaming_review_ref = map.get("accepted_dreaming_review_ref").cloned();

	if accepted_memory_authority_ref
		.as_ref()
		.is_some_and(|value| !value.is_null() && !is_valid_memory_authority_ref(value))
	{
		return Err(Error::InvalidRequest {
			message:
				"accepted_memory_authority_ref must be an active elf.memory_record_ref/v1 note ref."
					.to_string(),
		});
	}
	if accepted_dreaming_review_ref
		.as_ref()
		.is_some_and(|value| !value.is_null() && !is_valid_dreaming_review_ref(value))
	{
		return Err(Error::InvalidRequest {
			message:
				"accepted_dreaming_review_ref must be an accepted elf.dreaming_review_queue/v1 ref."
					.to_string(),
		});
	}

	Ok(serde_json::json!({
		"schema": WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1,
		"journal_entry_authority": "source_adjacent_only",
		"authoritative_memory_allowed": false,
		"promotion_required_for_current_facts": true,
		"accepted_memory_authority_ref": accepted_memory_authority_ref.unwrap_or(Value::Null),
		"accepted_dreaming_review_ref": accepted_dreaming_review_ref.unwrap_or(Value::Null),
		"requested_authoritative_memory_allowed": map
			.get("authoritative_memory_allowed")
			.and_then(Value::as_bool)
			.unwrap_or(false),
	}))
}

pub(in crate::work_journal) async fn resolve_promotion_boundary_authority(
	executor: &mut PgConnection,
	cfg: &Config,
	mut boundary: Value,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	now: OffsetDateTime,
) -> Result<Value> {
	let memory_ref = boundary.get("accepted_memory_authority_ref").cloned();
	let dreaming_ref = boundary.get("accepted_dreaming_review_ref").cloned();
	let mut has_accepted_ref = false;

	if let Some(memory_ref) =
		memory_ref.as_ref().filter(|value| is_valid_memory_authority_ref(value))
	{
		if !accepted_memory_authority_ref_is_readable(
			&mut *executor,
			cfg,
			memory_ref,
			tenant_id,
			project_id,
			agent_id,
			now,
		)
		.await?
		{
			return Err(Error::InvalidRequest {
				message: "accepted_memory_authority_ref was not found or is not readable."
					.to_string(),
			});
		}

		has_accepted_ref = true;
	}
	if let Some(dreaming_ref) =
		dreaming_ref.as_ref().filter(|value| is_valid_dreaming_review_ref(value))
	{
		if !accepted_dreaming_review_ref_exists(&mut *executor, dreaming_ref, tenant_id, project_id)
			.await?
		{
			return Err(Error::InvalidRequest {
				message: "accepted_dreaming_review_ref was not found or is not accepted."
					.to_string(),
			});
		}

		has_accepted_ref = true;
	}

	boundary["authoritative_memory_allowed"] = Value::Bool(has_accepted_ref);
	boundary["promotion_required_for_current_facts"] = Value::Bool(!has_accepted_ref);

	Ok(boundary)
}

fn is_valid_memory_authority_ref(value: &Value) -> bool {
	let Some(map) = value.as_object() else {
		return false;
	};
	let Some(id) = validation::object_string(map, "id") else {
		return false;
	};

	validation::object_string(map, "schema") == Some("elf.memory_record_ref/v1")
		&& validation::object_string(map, "kind") == Some("note")
		&& validation::object_string(map, "status") == Some("active")
		&& Uuid::parse_str(id).is_ok()
}

fn memory_ref_id(value: &Value) -> Option<Uuid> {
	Uuid::parse_str(validation::object_string(value.as_object()?, "id")?).ok()
}

fn is_valid_dreaming_review_ref(value: &Value) -> bool {
	let Some(map) = value.as_object() else {
		return false;
	};
	let Some(proposal_id) = validation::object_string(map, "proposal_id") else {
		return false;
	};
	let review_state = validation::object_string(map, "review_state");

	validation::object_string(map, "schema") == Some("elf.dreaming_review_queue/v1")
		&& Uuid::parse_str(proposal_id).is_ok()
		&& matches!(review_state, Some("approved" | "applied"))
}

fn dreaming_ref_proposal_id(value: &Value) -> Option<Uuid> {
	Uuid::parse_str(validation::object_string(value.as_object()?, "proposal_id")?).ok()
}

async fn accepted_memory_authority_ref_is_readable(
	executor: &mut PgConnection,
	cfg: &Config,
	memory_ref: &Value,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	now: OffsetDateTime,
) -> Result<bool> {
	let Some(note_id) = memory_ref_id(memory_ref) else {
		return Ok(false);
	};
	let note = sqlx::query_as::<_, MemoryNote>(
		"\
SELECT *
FROM memory_notes
WHERE note_id = $1
	AND tenant_id = $2
	AND project_id IN ($3, $4)
LIMIT 1",
	)
	.bind(note_id)
	.bind(tenant_id)
	.bind(project_id)
	.bind(ORG_PROJECT_ID)
	.fetch_optional(&mut *executor)
	.await?;
	let Some(note) = note else {
		return Ok(false);
	};
	let org_shared_allowed = cfg.scopes.allowed.iter().any(|scope| scope == "org_shared");
	let shared_grants = access::load_shared_read_grants_with_org_shared(
		&mut *executor,
		tenant_id,
		project_id,
		agent_id,
		org_shared_allowed,
	)
	.await?;

	Ok(access::note_read_allowed(&note, agent_id, &cfg.scopes.allowed, &shared_grants, now))
}

async fn accepted_dreaming_review_ref_exists(
	executor: &mut PgConnection,
	dreaming_ref: &Value,
	tenant_id: &str,
	project_id: &str,
) -> Result<bool> {
	let Some(proposal_id) = dreaming_ref_proposal_id(dreaming_ref) else {
		return Ok(false);
	};
	let Some(proposal) = consolidation::get_consolidation_proposal(
		&mut *executor,
		tenant_id,
		project_id,
		proposal_id,
	)
	.await?
	else {
		return Ok(false);
	};
	let Some(map) = dreaming_ref.as_object() else {
		return Ok(false);
	};

	Ok(matches!(proposal.review_state.as_str(), "approved" | "applied")
		&& validation::object_string(map, "review_state") == Some(proposal.review_state.as_str()))
}
