//! Source-adjacent Work Journal capture and readback APIs.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Value};
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	access::{self, ORG_PROJECT_ID, SharedSpaceGrantKey},
	search,
};
use elf_config::Config;
use elf_domain::{
	english_gate,
	writegate::{self, WritePolicy, WritePolicyAudit},
};
use elf_storage::{
	consolidation,
	models::{MemoryNote, WorkJournalEntry},
	work_journal,
};

/// Schema identifier for Work Journal readback.
pub const ELF_WORK_JOURNAL_SCHEMA_V1: &str = "elf.work_journal/v1";

const WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1: &str = "elf.work_journal.promotion_boundary/v1";
const DEFAULT_SESSION_READBACK_LIMIT: u32 = 20;
const MAX_SESSION_READBACK_LIMIT: u32 = 100;
const MAX_STORAGE_SCAN_ROWS: i64 = 500;
const MAX_BODY_CHARS: usize = 16_384;
const MAX_SIDE_LIST_ITEMS: usize = 64;

/// Work Journal entry family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkJournalEntryFamily {
	/// Session log captured alongside source work.
	SessionLog,
	/// Handoff brief for another agent or future session.
	HandoffBrief,
	/// Janitor or cleanup report.
	JanitorReport,
	/// Explicit next step stated in the source.
	ExplicitNextStep,
	/// Inferred next step retained as a non-authoritative hint.
	InferredNextStep,
	/// Option that was considered and rejected.
	RejectedOption,
}
impl WorkJournalEntryFamily {
	/// Returns the canonical API/storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::SessionLog => "session_log",
			Self::HandoffBrief => "handoff_brief",
			Self::JanitorReport => "janitor_report",
			Self::ExplicitNextStep => "explicit_next_step",
			Self::InferredNextStep => "inferred_next_step",
			Self::RejectedOption => "rejected_option",
		}
	}

	fn parse(raw: &str) -> Result<Self> {
		match raw {
			"session_log" => Ok(Self::SessionLog),
			"handoff_brief" => Ok(Self::HandoffBrief),
			"janitor_report" => Ok(Self::JanitorReport),
			"explicit_next_step" => Ok(Self::ExplicitNextStep),
			"inferred_next_step" => Ok(Self::InferredNextStep),
			"rejected_option" => Ok(Self::RejectedOption),
			_ => Err(Error::InvalidRequest {
				message: "family must be one of: session_log, handoff_brief, janitor_report, explicit_next_step, inferred_next_step, rejected_option.".to_string(),
			}),
		}
	}
}

impl ElfService {
	/// Captures one source-adjacent Work Journal entry.
	pub async fn work_journal_entry_create(
		&self,
		req: WorkJournalEntryCreateRequest,
	) -> Result<WorkJournalEntryCreateResponse> {
		let mut validated = validate_work_journal_create(&self.cfg, &req)?;
		let now = OffsetDateTime::now_utc();
		let effective_project_id = if validated.scope == "org_shared" {
			ORG_PROJECT_ID.to_string()
		} else {
			req.project_id.trim().to_string()
		};
		let mut tx = self.db.pool.begin().await?;

		validated.promotion_boundary = resolve_promotion_boundary_authority(
			&mut tx,
			&self.cfg,
			validated.promotion_boundary,
			req.tenant_id.trim(),
			req.project_id.trim(),
			req.agent_id.trim(),
			now,
		)
		.await?;

		let entry = WorkJournalEntry {
			entry_id: validated.entry_id,
			tenant_id: req.tenant_id.trim().to_string(),
			project_id: effective_project_id.clone(),
			agent_id: req.agent_id.trim().to_string(),
			scope: validated.scope,
			session_id: validated.session_id,
			family: req.family.as_str().to_string(),
			status: "active".to_string(),
			title: validated.title,
			body: validated.body,
			source_refs: validated.source_refs,
			explicit_next_steps: validated.explicit_next_steps,
			inferred_next_steps: validated.inferred_next_steps,
			rejected_options: validated.rejected_options,
			promotion_boundary: validated.promotion_boundary,
			redaction_audit: serde_json::to_value(validated.redaction_audit).map_err(|err| {
				Error::InvalidRequest { message: format!("redaction audit is invalid: {err}") }
			})?,
			created_at: now,
			updated_at: now,
		};

		work_journal::insert_work_journal_entry(&mut *tx, &entry).await?;

		if entry.scope != "agent_private" {
			access::ensure_active_project_scope_grant(
				&mut *tx,
				entry.tenant_id.as_str(),
				effective_project_id.as_str(),
				entry.scope.as_str(),
				entry.agent_id.as_str(),
			)
			.await?;
		}

		tx.commit().await?;

		Ok(WorkJournalEntryCreateResponse { entry: row_to_response(entry)? })
	}

	/// Reads one source-adjacent Work Journal entry.
	pub async fn work_journal_entry_get(
		&self,
		req: WorkJournalEntryGetRequest,
	) -> Result<WorkJournalEntryResponse> {
		validate_read_context(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			req.read_profile.as_str(),
		)?;

		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.trim())?;
		let shared_grants = load_work_journal_shared_grants(
			self,
			req.tenant_id.trim(),
			req.project_id.trim(),
			req.agent_id.trim(),
			&allowed_scopes,
		)
		.await?;
		let row =
			work_journal::get_work_journal_entry(&self.db.pool, req.tenant_id.trim(), req.entry_id)
				.await?
				.ok_or_else(|| Error::NotFound {
					message: "Work Journal entry not found.".to_string(),
				})?;

		if row.project_id != req.project_id.trim() && row.project_id != ORG_PROJECT_ID {
			return Err(Error::NotFound { message: "Work Journal entry not found.".to_string() });
		}
		if !work_journal_read_allowed(&row, req.agent_id.trim(), &allowed_scopes, &shared_grants) {
			return Err(Error::ScopeDenied {
				message: "Work Journal entry is not readable by this agent.".to_string(),
			});
		}

		row_to_response(row)
	}

	/// Reads newest-first Work Journal entries for one session.
	pub async fn work_journal_session_readback(
		&self,
		req: WorkJournalSessionReadbackRequest,
	) -> Result<WorkJournalSessionReadbackResponse> {
		validate_read_context(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			req.read_profile.as_str(),
		)?;
		validate_identifier(req.session_id.as_str(), "$.session_id")?;

		let limit = req
			.limit
			.unwrap_or(DEFAULT_SESSION_READBACK_LIMIT)
			.clamp(1, MAX_SESSION_READBACK_LIMIT);
		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.trim())?;
		let shared_grants = load_work_journal_shared_grants(
			self,
			req.tenant_id.trim(),
			req.project_id.trim(),
			req.agent_id.trim(),
			&allowed_scopes,
		)
		.await?;
		let family_filter =
			req.families.iter().copied().collect::<HashSet<WorkJournalEntryFamily>>();
		let rows = work_journal::list_work_journal_entries_for_session(
			&self.db.pool,
			req.tenant_id.trim(),
			req.project_id.trim(),
			ORG_PROJECT_ID,
			req.session_id.trim(),
			MAX_STORAGE_SCAN_ROWS,
		)
		.await?;
		let mut items = Vec::new();

		for row in rows {
			if !family_filter.is_empty()
				&& !family_filter.contains(&WorkJournalEntryFamily::parse(row.family.as_str())?)
			{
				continue;
			}
			if !work_journal_read_allowed(
				&row,
				req.agent_id.trim(),
				&allowed_scopes,
				&shared_grants,
			) {
				continue;
			}

			items.push(row_to_response(row)?);

			if items.len() >= limit as usize {
				break;
			}
		}

		let where_stopped = build_where_stopped(&items);

		Ok(WorkJournalSessionReadbackResponse {
			schema: ELF_WORK_JOURNAL_SCHEMA_V1.to_string(),
			session_id: req.session_id.trim().to_string(),
			items,
			where_stopped,
		})
	}
}

/// Request payload for source-adjacent Work Journal capture.
#[derive(Clone, Debug, Deserialize)]
pub struct WorkJournalEntryCreateRequest {
	/// Tenant that owns the entry.
	pub tenant_id: String,
	/// Project that owns the entry.
	pub project_id: String,
	/// Agent capturing the entry.
	pub agent_id: String,
	/// Optional caller-supplied stable identifier.
	pub entry_id: Option<Uuid>,
	/// Visibility scope for readback.
	pub scope: String,
	/// Stable session identifier for grouping entries.
	pub session_id: String,
	/// Entry family.
	pub family: WorkJournalEntryFamily,
	/// Optional display title.
	pub title: Option<String>,
	/// Journal body. This is source-adjacent, not authoritative memory.
	pub body: String,
	/// Source refs that support the journal entry.
	pub source_refs: Vec<Value>,
	/// Redaction/exclusion policy applied before persistence.
	pub write_policy: Option<WritePolicy>,
	#[serde(default)]
	/// Explicit next steps stated by the captured source.
	pub explicit_next_steps: Vec<String>,
	#[serde(default)]
	/// Inferred next steps retained as non-authoritative hints.
	pub inferred_next_steps: Vec<String>,
	#[serde(default)]
	/// Options considered and rejected during the captured work.
	pub rejected_options: Vec<String>,
	#[serde(default = "empty_object")]
	/// Promotion boundary metadata.
	pub promotion_boundary: Value,
}

/// Response payload after Work Journal capture.
#[derive(Clone, Debug, Serialize)]
pub struct WorkJournalEntryCreateResponse {
	/// Stored Work Journal entry.
	pub entry: WorkJournalEntryResponse,
}

/// Request payload for one Work Journal entry lookup.
#[derive(Clone, Debug, Deserialize)]
pub struct WorkJournalEntryGetRequest {
	/// Tenant that owns the entry.
	pub tenant_id: String,
	/// Project used for read-profile and shared-grant checks.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Entry identifier.
	pub entry_id: Uuid,
}

/// Request payload for session-level Work Journal readback.
#[derive(Clone, Debug, Deserialize)]
pub struct WorkJournalSessionReadbackRequest {
	/// Tenant that owns the session.
	pub tenant_id: String,
	/// Project used for read-profile and shared-grant checks.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Stable session identifier to read.
	pub session_id: String,
	#[serde(default)]
	/// Optional family filter.
	pub families: Vec<WorkJournalEntryFamily>,
	/// Maximum number of returned entries.
	pub limit: Option<u32>,
}

/// Session-level Work Journal readback.
#[derive(Clone, Debug, Serialize)]
pub struct WorkJournalSessionReadbackResponse {
	/// Readback schema identifier.
	pub schema: String,
	/// Stable session identifier.
	pub session_id: String,
	/// Newest-first journal entries.
	pub items: Vec<WorkJournalEntryResponse>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Compact "where did we stop" projection from the returned entries.
	pub where_stopped: Option<WorkJournalWhereStopped>,
}

/// One source-adjacent Work Journal entry returned by readback.
#[derive(Clone, Debug, Serialize)]
pub struct WorkJournalEntryResponse {
	/// Readback schema identifier.
	pub schema: String,
	/// Journal entry identifier.
	pub entry_id: Uuid,
	/// Tenant that owns the entry.
	pub tenant_id: String,
	/// Project that owns the entry.
	pub project_id: String,
	/// Agent that captured the entry.
	pub agent_id: String,
	/// Visibility scope for readback.
	pub scope: String,
	/// Stable session identifier.
	pub session_id: String,
	/// Entry family.
	pub family: WorkJournalEntryFamily,
	/// Lifecycle status.
	pub status: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional display title.
	pub title: Option<String>,
	/// Redacted durable journal body.
	pub body: String,
	/// Source refs supporting the entry.
	pub source_refs: Vec<Value>,
	/// Explicit next steps stated by the captured source.
	pub explicit_next_steps: Vec<String>,
	/// Inferred next steps retained as non-authoritative hints.
	pub inferred_next_steps: Vec<String>,
	/// Rejected options captured by the journal.
	pub rejected_options: Vec<String>,
	/// Promotion boundary metadata.
	pub promotion_boundary: Value,
	/// Redaction audit for the durable journal body.
	pub redaction_audit: WritePolicyAudit,
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Compact "where did we stop" projection for one journal session.
#[derive(Clone, Debug, Serialize)]
pub struct WorkJournalWhereStopped {
	/// Latest returned entry identifier.
	pub latest_entry_id: Uuid,
	/// Latest returned entry family.
	pub latest_family: WorkJournalEntryFamily,
	/// Source refs associated with the latest returned entry.
	pub source_refs: Vec<Value>,
	/// Most recent explicit next steps in returned entries.
	pub explicit_next_steps: Vec<String>,
	/// Most recent inferred next steps in returned entries.
	pub inferred_next_steps: Vec<String>,
	/// Most recent rejected options in returned entries.
	pub rejected_options: Vec<String>,
	/// Promotion boundary for the latest returned entry.
	pub promotion_boundary: Value,
}

struct ValidatedWorkJournalCreate {
	entry_id: Uuid,
	scope: String,
	session_id: String,
	title: Option<String>,
	body: String,
	source_refs: Value,
	explicit_next_steps: Value,
	inferred_next_steps: Value,
	rejected_options: Value,
	promotion_boundary: Value,
	redaction_audit: WritePolicyAudit,
}

fn validate_work_journal_create(
	cfg: &Config,
	req: &WorkJournalEntryCreateRequest,
) -> Result<ValidatedWorkJournalCreate> {
	validate_write_context(
		cfg,
		req.tenant_id.as_str(),
		req.project_id.as_str(),
		req.agent_id.as_str(),
		req.scope.as_str(),
	)?;
	validate_identifier(req.session_id.as_str(), "$.session_id")?;

	if req.body.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "body must be non-empty.".to_string() });
	}
	if req.body.chars().count() > MAX_BODY_CHARS {
		return Err(Error::InvalidRequest {
			message: "body exceeds max journal size.".to_string(),
		});
	}

	let title = req.title.as_ref().map(|title| title.trim().to_string()).filter(|s| !s.is_empty());

	if let Some(title) = title.as_ref() {
		validate_natural_language(title.as_str(), "$.title")?;

		if writegate::contains_secrets(title.as_str()) {
			return Err(Error::InvalidRequest { message: "title contains secrets.".to_string() });
		}
	}

	let policy_result = writegate::apply_write_policy(req.body.as_str(), req.write_policy.as_ref())
		.map_err(|err| Error::InvalidRequest {
			message: format!("write_policy is invalid: {err:?}"),
		})?;
	let body = policy_result.transformed;

	if body.trim().is_empty() {
		return Err(Error::InvalidRequest { message: "body must be non-empty.".to_string() });
	}

	validate_natural_language(body.as_str(), "$.body")?;

	if writegate::contains_secrets(body.as_str()) {
		return Err(Error::InvalidRequest { message: "body contains secrets.".to_string() });
	}

	validate_text_list(&req.explicit_next_steps, "$.explicit_next_steps")?;
	validate_text_list(&req.inferred_next_steps, "$.inferred_next_steps")?;
	validate_text_list(&req.rejected_options, "$.rejected_options")?;

	let source_refs = validate_source_refs(&req.source_refs)?;
	let promotion_boundary = normalize_promotion_boundary(&req.promotion_boundary)?;
	let explicit_next_steps = serde_json::to_value(&req.explicit_next_steps).map_err(|err| {
		Error::InvalidRequest { message: format!("explicit_next_steps are invalid: {err}") }
	})?;
	let inferred_next_steps = serde_json::to_value(&req.inferred_next_steps).map_err(|err| {
		Error::InvalidRequest { message: format!("inferred_next_steps are invalid: {err}") }
	})?;
	let rejected_options = serde_json::to_value(&req.rejected_options).map_err(|err| {
		Error::InvalidRequest { message: format!("rejected_options are invalid: {err}") }
	})?;

	Ok(ValidatedWorkJournalCreate {
		entry_id: req.entry_id.unwrap_or_else(Uuid::new_v4),
		scope: req.scope.trim().to_string(),
		session_id: req.session_id.trim().to_string(),
		title,
		body,
		source_refs,
		explicit_next_steps,
		inferred_next_steps,
		rejected_options,
		promotion_boundary,
		redaction_audit: policy_result.audit,
	})
}

fn validate_write_context(
	cfg: &Config,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
) -> Result<()> {
	if tenant_id.trim().is_empty()
		|| project_id.trim().is_empty()
		|| agent_id.trim().is_empty()
		|| scope.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, agent_id, and scope are required.".to_string(),
		});
	}

	validate_identifier(tenant_id, "$.tenant_id")?;
	validate_identifier(project_id, "$.project_id")?;
	validate_identifier(agent_id, "$.agent_id")?;

	if !cfg.scopes.allowed.iter().any(|allowed| allowed == scope.trim()) {
		return Err(Error::ScopeDenied { message: "scope is not allowed.".to_string() });
	}
	if !scope_write_allowed(cfg, scope.trim()) {
		return Err(Error::ScopeDenied { message: "scope is not writable.".to_string() });
	}

	Ok(())
}

fn validate_read_context(
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	read_profile: &str,
) -> Result<()> {
	if tenant_id.trim().is_empty()
		|| project_id.trim().is_empty()
		|| agent_id.trim().is_empty()
		|| read_profile.trim().is_empty()
	{
		return Err(Error::InvalidRequest {
			message: "tenant_id, project_id, agent_id, and read_profile are required.".to_string(),
		});
	}

	validate_identifier(tenant_id, "$.tenant_id")?;
	validate_identifier(project_id, "$.project_id")?;
	validate_identifier(agent_id, "$.agent_id")?;
	validate_identifier(read_profile, "$.read_profile")?;

	Ok(())
}

fn validate_text_list(values: &[String], path: &str) -> Result<()> {
	if values.len() > MAX_SIDE_LIST_ITEMS {
		return Err(Error::InvalidRequest { message: format!("{path} has too many items.") });
	}

	for (index, value) in values.iter().enumerate() {
		if value.trim().is_empty() {
			return Err(Error::InvalidRequest {
				message: format!("{path}[{index}] must be non-empty."),
			});
		}

		validate_natural_language(value.as_str(), format!("{path}[{index}]").as_str())?;

		if writegate::contains_secrets(value.as_str()) {
			return Err(Error::InvalidRequest {
				message: format!("{path}[{index}] contains secrets."),
			});
		}
	}

	Ok(())
}

fn validate_source_refs(source_refs: &[Value]) -> Result<Value> {
	if source_refs.is_empty() {
		return Err(Error::InvalidRequest {
			message: "source_refs must be non-empty.".to_string(),
		});
	}
	if source_refs.len() > MAX_SIDE_LIST_ITEMS {
		return Err(Error::InvalidRequest {
			message: "source_refs has too many items.".to_string(),
		});
	}

	for (index, source_ref) in source_refs.iter().enumerate() {
		match source_ref {
			Value::Object(map) if !map.is_empty() => {},
			_ => {
				return Err(Error::InvalidRequest {
					message: format!("source_refs[{index}] must be a non-empty object."),
				});
			},
		}
	}

	let value = Value::Array(source_refs.to_vec());

	validate_json_strings(&value, "$.source_refs")?;

	Ok(value)
}

fn validate_json_strings(value: &Value, path: &str) -> Result<()> {
	match value {
		Value::String(text) => {
			validate_identifier(text.as_str(), path)?;

			if writegate::contains_secrets(text.as_str()) {
				return Err(Error::InvalidRequest { message: format!("{path} contains secrets.") });
			}
		},
		Value::Array(items) =>
			for (index, item) in items.iter().enumerate() {
				validate_json_strings(item, format!("{path}[{index}]").as_str())?;
			},
		Value::Object(map) =>
			for (key, item) in map {
				validate_identifier(key.as_str(), format!("{path}.{key}").as_str())?;
				validate_json_strings(item, format!("{path}.{key}").as_str())?;
			},
		Value::Null | Value::Bool(_) | Value::Number(_) => {},
	}

	Ok(())
}

fn normalize_promotion_boundary(input: &Value) -> Result<Value> {
	let map = match input {
		Value::Null => Map::new(),
		Value::Object(map) => map.clone(),
		_ => {
			return Err(Error::InvalidRequest {
				message: "promotion_boundary must be a JSON object.".to_string(),
			});
		},
	};

	validate_json_strings(&Value::Object(map.clone()), "$.promotion_boundary")?;

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

fn is_valid_memory_authority_ref(value: &Value) -> bool {
	let Some(map) = value.as_object() else {
		return false;
	};
	let Some(id) = object_string(map, "id") else {
		return false;
	};

	object_string(map, "schema") == Some("elf.memory_record_ref/v1")
		&& object_string(map, "kind") == Some("note")
		&& object_string(map, "status") == Some("active")
		&& Uuid::parse_str(id).is_ok()
}

fn memory_ref_id(value: &Value) -> Option<Uuid> {
	Uuid::parse_str(object_string(value.as_object()?, "id")?).ok()
}

fn is_valid_dreaming_review_ref(value: &Value) -> bool {
	let Some(map) = value.as_object() else {
		return false;
	};
	let Some(proposal_id) = object_string(map, "proposal_id") else {
		return false;
	};
	let review_state = object_string(map, "review_state");

	object_string(map, "schema") == Some("elf.dreaming_review_queue/v1")
		&& Uuid::parse_str(proposal_id).is_ok()
		&& matches!(review_state, Some("approved" | "applied"))
}

fn dreaming_ref_proposal_id(value: &Value) -> Option<Uuid> {
	Uuid::parse_str(object_string(value.as_object()?, "proposal_id")?).ok()
}

fn object_string<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
	map.get(key).and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

fn validate_identifier(text: &str, field: &str) -> Result<()> {
	if text.trim().is_empty() || !english_gate::is_english_identifier(text.trim()) {
		return Err(Error::NonEnglishInput { field: field.to_string() });
	}

	Ok(())
}

fn validate_natural_language(text: &str, field: &str) -> Result<()> {
	if !english_gate::is_english_natural_language(text) {
		return Err(Error::NonEnglishInput { field: field.to_string() });
	}

	Ok(())
}

fn scope_write_allowed(cfg: &Config, scope: &str) -> bool {
	match scope {
		"agent_private" => cfg.scopes.write_allowed.agent_private,
		"project_shared" => cfg.scopes.write_allowed.project_shared,
		"org_shared" => cfg.scopes.write_allowed.org_shared,
		_ => false,
	}
}

fn work_journal_read_allowed(
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

fn row_to_response(row: WorkJournalEntry) -> Result<WorkJournalEntryResponse> {
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

fn build_where_stopped(items: &[WorkJournalEntryResponse]) -> Option<WorkJournalWhereStopped> {
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

fn first_non_empty<'a>(mut lists: impl Iterator<Item = &'a Vec<String>>) -> Vec<String> {
	lists.find(|items| !items.is_empty()).cloned().unwrap_or_default()
}

fn empty_object() -> Value {
	Value::Object(Map::new())
}

async fn resolve_promotion_boundary_authority(
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
		&& object_string(map, "review_state") == Some(proposal.review_state.as_str()))
}

async fn load_work_journal_shared_grants(
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

#[cfg(test)]
mod tests {
	use std::collections::HashSet;

	use serde_json;
	use time::OffsetDateTime;
	use uuid::Uuid;

	use crate::{
		access::SharedSpaceGrantKey,
		work_journal::{self, WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1},
	};
	use elf_storage::models::WorkJournalEntry;

	#[test]
	fn promotion_boundary_flags_journal_only_without_accepted_ref() {
		let boundary = work_journal::normalize_promotion_boundary(&serde_json::json!({
			"authoritative_memory_allowed": true
		}))
		.expect("boundary should normalize");

		assert_eq!(boundary["schema"], WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1);
		assert_eq!(boundary["authoritative_memory_allowed"], false);
		assert_eq!(boundary["promotion_required_for_current_facts"], true);
		assert_eq!(boundary["requested_authoritative_memory_allowed"], true);
	}

	#[test]
	fn promotion_boundary_preserves_memory_ref_without_granting_shape_only_authority() {
		let boundary = work_journal::normalize_promotion_boundary(&serde_json::json!({
			"accepted_memory_authority_ref": {
				"schema": "elf.memory_record_ref/v1",
				"kind": "note",
				"id": "11111111-1111-1111-1111-111111111111",
				"status": "active"
			}
		}))
		.expect("boundary should normalize");

		assert_eq!(boundary["authoritative_memory_allowed"], false);
		assert_eq!(boundary["promotion_required_for_current_facts"], true);
		assert_eq!(
			boundary["accepted_memory_authority_ref"]["id"],
			serde_json::json!("11111111-1111-1111-1111-111111111111")
		);
	}

	#[test]
	fn promotion_boundary_preserves_dreaming_ref_without_granting_shape_only_authority() {
		let boundary = work_journal::normalize_promotion_boundary(&serde_json::json!({
			"accepted_dreaming_review_ref": {
				"schema": "elf.dreaming_review_queue/v1",
				"proposal_id": "22222222-2222-4222-8222-222222222222",
				"review_state": "applied"
			}
		}))
		.expect("boundary should normalize");

		assert_eq!(boundary["authoritative_memory_allowed"], false);
		assert_eq!(boundary["promotion_required_for_current_facts"], true);
		assert_eq!(
			boundary["accepted_dreaming_review_ref"]["proposal_id"],
			serde_json::json!("22222222-2222-4222-8222-222222222222")
		);
	}

	#[test]
	fn promotion_boundary_rejects_forged_accepted_refs() {
		let primitive_result = work_journal::normalize_promotion_boundary(&serde_json::json!({
			"accepted_memory_authority_ref": true
		}));
		let object_result = work_journal::normalize_promotion_boundary(&serde_json::json!({
			"accepted_memory_authority_ref": {
				"schema": "elf.memory_record_ref/v1",
				"id": "11111111-1111-1111-1111-111111111111"
			}
		}));

		assert!(primitive_result.is_err());
		assert!(object_result.is_err());
	}

	#[test]
	fn source_refs_reject_non_object_items() {
		let result = work_journal::validate_source_refs(&[serde_json::json!("XY-1117")]);

		assert!(result.is_err());
	}

	#[test]
	fn read_allowed_enforces_private_and_shared_grants() {
		let allowed = vec!["agent_private".to_string(), "project_shared".to_string()];
		let no_grants = HashSet::new();
		let private = journal_row("agent_private", "agent-a", "active");
		let shared = journal_row("project_shared", "agent-a", "active");
		let inactive = journal_row("agent_private", "agent-a", "deleted");

		assert!(work_journal::work_journal_read_allowed(&private, "agent-a", &allowed, &no_grants));
		assert!(!work_journal::work_journal_read_allowed(
			&private, "agent-b", &allowed, &no_grants
		));
		assert!(!work_journal::work_journal_read_allowed(
			&inactive, "agent-a", &allowed, &no_grants
		));
		assert!(work_journal::work_journal_read_allowed(&shared, "agent-a", &allowed, &no_grants));
		assert!(!work_journal::work_journal_read_allowed(&shared, "agent-b", &allowed, &no_grants));

		let mut grants = HashSet::new();

		grants.insert(SharedSpaceGrantKey {
			scope: "project_shared".to_string(),
			space_owner_agent_id: "agent-a".to_string(),
		});

		assert!(work_journal::work_journal_read_allowed(&shared, "agent-b", &allowed, &grants));

		let private_only = vec!["agent_private".to_string()];

		assert!(!work_journal::work_journal_read_allowed(
			&shared,
			"agent-b",
			&private_only,
			&grants
		));
	}

	fn journal_row(scope: &str, agent_id: &str, status: &str) -> WorkJournalEntry {
		let now = OffsetDateTime::now_utc();

		WorkJournalEntry {
			entry_id: Uuid::nil(),
			tenant_id: "tenant".to_string(),
			project_id: "project".to_string(),
			agent_id: agent_id.to_string(),
			scope: scope.to_string(),
			session_id: "session".to_string(),
			family: "session_log".to_string(),
			status: status.to_string(),
			title: None,
			body: "body".to_string(),
			source_refs: serde_json::json!([{ "schema": "source_ref/v1" }]),
			explicit_next_steps: serde_json::json!([]),
			inferred_next_steps: serde_json::json!([]),
			rejected_options: serde_json::json!([]),
			promotion_boundary: serde_json::json!({}),
			redaction_audit: serde_json::json!({}),
			created_at: now,
			updated_at: now,
		}
	}
}
