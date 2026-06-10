//! Provenance inspection APIs.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result};
use elf_storage::models::MemoryNote;

const NOTE_PROVENANCE_BUNDLE_SCHEMA_V1: &str = "elf.note_provenance_bundle/v1";
const NOTE_PROVENANCE_INGEST_DECISIONS_LIMIT: i64 = 100;
const NOTE_PROVENANCE_NOTE_VERSIONS_LIMIT: i64 = 100;
const NOTE_PROVENANCE_OUTBOX_LIMIT: i64 = 100;
const NOTE_PROVENANCE_RECENT_TRACES_LIMIT: i64 = 20;
const NOTE_PROVENANCE_HISTORY_LIMIT: i64 = 200;
const MEMORY_HISTORY_SCHEMA_V1: &str = "elf.memory_history/v1";

/// Request payload for note provenance lookup.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceGetRequest {
	/// Tenant that owns the note.
	pub tenant_id: String,
	/// Project that owns the note.
	pub project_id: String,
	/// Identifier of the note to inspect.
	pub note_id: Uuid,
}

/// Request payload for memory-history lookup.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryHistoryGetRequest {
	/// Tenant that owns the memory.
	pub tenant_id: String,
	/// Project that owns the memory.
	pub project_id: String,
	/// Identifier of the note to inspect.
	pub note_id: Uuid,
}

/// Timeline response for one memory.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryHistoryResponse {
	/// History schema identifier.
	pub schema: String,
	/// Inspected note identifier.
	pub note_id: Uuid,
	/// Chronological memory events.
	pub events: Vec<MemoryHistoryEvent>,
}

/// Full provenance bundle for one note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceBundleResponse {
	/// Provenance bundle schema identifier.
	pub schema: String,
	/// Current persisted note snapshot.
	pub note: NoteProvenanceNote,
	/// Recorded ingestion decisions for the note.
	pub ingest_decisions: Vec<NoteProvenanceIngestDecision>,
	/// Version-history rows for the note.
	pub note_versions: Vec<NoteProvenanceNoteVersion>,
	/// Indexing outbox history for the note.
	pub indexing_outbox: Vec<NoteProvenanceIndexingOutbox>,
	/// Recent search traces that referenced the note.
	pub recent_traces: Vec<NoteProvenanceRecentTrace>,
	/// Chronological memory event timeline for the note.
	pub history: Vec<MemoryHistoryEvent>,
}

/// Current note snapshot returned by provenance APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceNote {
	/// Note identifier.
	pub note_id: Uuid,
	/// Tenant that owns the note.
	pub tenant_id: String,
	/// Project that owns the note.
	pub project_id: String,
	/// Agent that wrote the note.
	pub agent_id: String,
	/// Scope key for the note.
	pub scope: String,
	/// Note type discriminator.
	pub r#type: String,
	/// Optional application-defined key.
	pub key: Option<String>,
	/// Note body text.
	pub text: String,
	/// Importance score.
	pub importance: f32,
	/// Confidence score.
	pub confidence: f32,
	/// Lifecycle status.
	pub status: String,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// Optional expiry timestamp.
	pub expires_at: Option<OffsetDateTime>,
	/// Structured source reference metadata.
	pub source_ref: Value,
	/// Embedding version associated with the note.
	pub embedding_version: String,
	/// Search hit counter.
	pub hit_count: i64,
	#[serde(with = "crate::time_serde::option")]
	/// Timestamp of the most recent hit.
	pub last_hit_at: Option<OffsetDateTime>,
}
impl From<MemoryNote> for NoteProvenanceNote {
	fn from(note: MemoryNote) -> Self {
		Self {
			note_id: note.note_id,
			tenant_id: note.tenant_id,
			project_id: note.project_id,
			agent_id: note.agent_id,
			scope: note.scope,
			r#type: note.r#type,
			key: note.key,
			text: note.text,
			importance: note.importance,
			confidence: note.confidence,
			status: note.status,
			created_at: note.created_at,
			updated_at: note.updated_at,
			expires_at: note.expires_at,
			source_ref: note.source_ref,
			embedding_version: note.embedding_version,
			hit_count: note.hit_count,
			last_hit_at: note.last_hit_at,
		}
	}
}

/// One recorded ingestion decision for a note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceIngestDecision {
	/// Decision identifier.
	pub decision_id: Uuid,
	/// Tenant that owns the decision record.
	pub tenant_id: String,
	/// Project that owns the decision record.
	pub project_id: String,
	/// Agent that triggered the ingestion decision.
	pub agent_id: String,
	/// Scope key evaluated by the decision.
	pub scope: String,
	/// Pipeline name that produced the decision.
	pub pipeline: String,
	/// Note type discriminator under evaluation.
	pub note_type: String,
	/// Optional application-defined key under evaluation.
	pub note_key: Option<String>,
	/// Note identifier, when a note was persisted or matched.
	pub note_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Note version produced by this decision, when applicable.
	pub note_version_id: Option<Uuid>,
	/// Pre-policy base decision.
	pub base_decision: String,
	/// Final policy decision.
	pub policy_decision: String,
	/// Persistence operation that followed the decision.
	pub note_op: String,
	/// Machine-readable reason code, if any.
	pub reason_code: Option<String>,
	/// Structured diagnostic details.
	pub details: Value,
	#[serde(with = "crate::time_serde")]
	/// Decision timestamp.
	pub ts: OffsetDateTime,
}
impl From<NoteIngestDecisionRow> for NoteProvenanceIngestDecision {
	fn from(row: NoteIngestDecisionRow) -> Self {
		Self {
			decision_id: row.decision_id,
			tenant_id: row.tenant_id,
			project_id: row.project_id,
			agent_id: row.agent_id,
			scope: row.scope,
			pipeline: row.pipeline,
			note_type: row.note_type,
			note_key: row.note_key,
			note_id: row.note_id,
			note_version_id: row.note_version_id,
			base_decision: row.base_decision,
			policy_decision: row.policy_decision,
			note_op: row.note_op,
			reason_code: row.reason_code,
			details: row.details,
			ts: row.ts,
		}
	}
}

/// One version-history row for a note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceNoteVersion {
	/// Version row identifier.
	pub version_id: Uuid,
	/// Note identifier.
	pub note_id: Uuid,
	/// Operation recorded in the version row.
	pub op: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Snapshot before the operation, when available.
	pub prev_snapshot: Option<Value>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Snapshot after the operation, when available.
	pub new_snapshot: Option<Value>,
	/// Human-readable reason for the change.
	pub reason: String,
	/// Actor that performed the change.
	pub actor: String,
	#[serde(with = "crate::time_serde")]
	/// Version timestamp.
	pub ts: OffsetDateTime,
}
impl From<NoteVersionRow> for NoteProvenanceNoteVersion {
	fn from(row: NoteVersionRow) -> Self {
		Self {
			version_id: row.version_id,
			note_id: row.note_id,
			op: row.op,
			prev_snapshot: row.prev_snapshot,
			new_snapshot: row.new_snapshot,
			reason: row.reason,
			actor: row.actor,
			ts: row.ts,
		}
	}
}

/// One indexing-outbox row for a note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceIndexingOutbox {
	/// Outbox identifier.
	pub outbox_id: Uuid,
	/// Note identifier.
	pub note_id: Uuid,
	/// Requested indexing operation.
	pub op: String,
	/// Embedding version targeted by the job.
	pub embedding_version: String,
	/// Current outbox status.
	pub status: String,
	/// Number of attempts already made.
	pub attempts: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Most recent failure text, if any.
	pub last_error: Option<String>,
	#[serde(with = "crate::time_serde")]
	/// Earliest time the job may be claimed again.
	pub available_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}
impl From<NoteIndexingOutboxRow> for NoteProvenanceIndexingOutbox {
	fn from(row: NoteIndexingOutboxRow) -> Self {
		Self {
			outbox_id: row.outbox_id,
			note_id: row.note_id,
			op: row.op,
			embedding_version: row.embedding_version,
			status: row.status,
			attempts: row.attempts,
			last_error: row.last_error,
			available_at: row.available_at,
			created_at: row.created_at,
			updated_at: row.updated_at,
		}
	}
}

/// Recent search trace that referenced the note.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NoteProvenanceRecentTrace {
	/// Search trace identifier.
	pub trace_id: Uuid,
	/// Tenant that owns the trace.
	pub tenant_id: String,
	/// Project that owns the trace.
	pub project_id: String,
	/// Agent that ran the search.
	pub agent_id: String,
	/// Read profile used for the trace.
	pub read_profile: String,
	/// Search query text.
	pub query: String,
	#[serde(with = "crate::time_serde")]
	/// Trace creation timestamp.
	pub created_at: OffsetDateTime,
}

/// One normalized memory-history event.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryHistoryEvent {
	/// Stable event identifier within its source table.
	pub event_id: String,
	/// Normalized event type.
	pub event_type: String,
	/// Subject kind for the event.
	pub subject_type: String,
	/// Inspected note identifier.
	pub note_id: Uuid,
	/// Durable source table behind the event.
	pub source_table: String,
	/// Source row identifier when available.
	pub source_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Related note version, when an ingest decision produced a version row.
	pub related_note_version_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Related ingest decision, when a version or history event was caused by ingestion.
	pub related_decision_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Related consolidation proposal, when a derived memory proposal references the note.
	pub related_proposal_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Actor that caused the event, when available.
	pub actor: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Source operation string.
	pub op: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Machine-readable reason code, when available.
	pub reason_code: Option<String>,
	/// Human-readable one-line event summary.
	pub summary: String,
	/// Source-specific event details.
	pub details: Value,
	#[serde(with = "crate::time_serde")]
	/// Event timestamp.
	pub ts: OffsetDateTime,
}

#[derive(Clone, Debug)]
struct ValidatedNoteProvenanceRequest {
	tenant_id: String,
	project_id: String,
	note_id: Uuid,
}

#[derive(FromRow)]
struct NoteIngestDecisionRow {
	decision_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	scope: String,
	pipeline: String,
	note_type: String,
	note_key: Option<String>,
	note_id: Option<Uuid>,
	note_version_id: Option<Uuid>,
	base_decision: String,
	policy_decision: String,
	note_op: String,
	reason_code: Option<String>,
	details: Value,
	ts: OffsetDateTime,
}

#[derive(FromRow)]
struct NoteVersionRow {
	version_id: Uuid,
	note_id: Uuid,
	op: String,
	prev_snapshot: Option<Value>,
	new_snapshot: Option<Value>,
	reason: String,
	actor: String,
	ts: OffsetDateTime,
}

#[derive(FromRow)]
struct NoteIndexingOutboxRow {
	outbox_id: Uuid,
	note_id: Uuid,
	op: String,
	embedding_version: String,
	status: String,
	attempts: i32,
	last_error: Option<String>,
	available_at: OffsetDateTime,
	created_at: OffsetDateTime,
	updated_at: OffsetDateTime,
}

#[derive(FromRow)]
struct NoteRecentTraceRow {
	trace_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	query: String,
	created_at: OffsetDateTime,
}

#[derive(FromRow)]
struct NoteDerivedProposalRow {
	proposal_id: Uuid,
	run_id: Uuid,
	agent_id: String,
	proposal_kind: String,
	apply_intent: String,
	review_state: String,
	source_refs: Value,
	source_snapshot: Value,
	lineage: Value,
	diff: Value,
	confidence: f32,
	target_ref: Value,
	proposed_payload: Value,
	created_at: OffsetDateTime,
}

#[derive(FromRow)]
struct NoteProposalReviewRow {
	review_id: Uuid,
	proposal_id: Uuid,
	run_id: Uuid,
	reviewer_agent_id: String,
	action: String,
	from_review_state: String,
	to_review_state: String,
	review_comment: Option<String>,
	created_at: OffsetDateTime,
	proposal_kind: String,
	apply_intent: String,
	diff: Value,
}

impl ElfService {
	/// Loads the current note plus recent provenance tables as one bundle.
	pub async fn note_provenance_get(
		&self,
		req: NoteProvenanceGetRequest,
	) -> Result<NoteProvenanceBundleResponse> {
		let req = validate_note_provenance_request(req)?;
		let note = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1
  AND tenant_id = $2
  AND project_id = $3",
		)
		.bind(req.note_id)
		.bind(&req.tenant_id)
		.bind(&req.project_id)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(note_row) = note else {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		};
		let ingest_decisions = load_ingest_decisions(&self.db.pool, &req).await?;
		let note_versions =
			load_note_versions(&self.db.pool, &req.tenant_id, &req.project_id, req.note_id).await?;
		let indexing_outbox =
			load_indexing_outbox(&self.db.pool, &req.tenant_id, &req.project_id, req.note_id)
				.await?;
		let recent_traces = load_recent_traces_for_note(
			&self.db.pool,
			&req.tenant_id,
			&req.project_id,
			req.note_id,
		)
		.await?;
		let history = load_memory_history_events(&self.db.pool, &req, &note_row).await?;

		Ok(NoteProvenanceBundleResponse {
			schema: NOTE_PROVENANCE_BUNDLE_SCHEMA_V1.to_string(),
			note: NoteProvenanceNote::from(note_row),
			ingest_decisions,
			note_versions,
			indexing_outbox,
			recent_traces,
			history,
		})
	}

	/// Loads the normalized memory-history timeline for one note.
	pub async fn memory_history_get(
		&self,
		req: MemoryHistoryGetRequest,
	) -> Result<MemoryHistoryResponse> {
		let req = validate_note_provenance_request(NoteProvenanceGetRequest {
			tenant_id: req.tenant_id,
			project_id: req.project_id,
			note_id: req.note_id,
		})?;
		let note_row = sqlx::query_as::<_, MemoryNote>(
			"\
SELECT *
FROM memory_notes
WHERE note_id = $1
  AND tenant_id = $2
  AND project_id = $3",
		)
		.bind(req.note_id)
		.bind(&req.tenant_id)
		.bind(&req.project_id)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(note_row) = note_row else {
			return Err(Error::InvalidRequest { message: "Note not found.".to_string() });
		};
		let events = load_memory_history_events(&self.db.pool, &req, &note_row).await?;

		Ok(MemoryHistoryResponse {
			schema: MEMORY_HISTORY_SCHEMA_V1.to_string(),
			note_id: req.note_id,
			events,
		})
	}
}

fn validate_note_provenance_request(
	req: NoteProvenanceGetRequest,
) -> Result<ValidatedNoteProvenanceRequest> {
	let tenant_id = req.tenant_id.trim();
	let project_id = req.project_id.trim();

	if tenant_id.is_empty() || project_id.is_empty() {
		return Err(Error::InvalidRequest {
			message: "tenant_id and project_id are required.".to_string(),
		});
	}

	Ok(ValidatedNoteProvenanceRequest {
		tenant_id: tenant_id.to_string(),
		project_id: project_id.to_string(),
		note_id: req.note_id,
	})
}

fn to_recent_trace(item: NoteRecentTraceRow) -> NoteProvenanceRecentTrace {
	NoteProvenanceRecentTrace {
		trace_id: item.trace_id,
		tenant_id: item.tenant_id,
		project_id: item.project_id,
		agent_id: item.agent_id,
		read_profile: item.read_profile,
		query: item.query,
		created_at: item.created_at,
	}
}

fn version_history_event(
	version: &NoteProvenanceNoteVersion,
	decision: Option<&&NoteProvenanceIngestDecision>,
) -> MemoryHistoryEvent {
	let event_type = version_event_type(version.op.as_str(), version.reason.as_str());
	let related_decision_id = decision.map(|decision| decision.decision_id);
	let details = serde_json::json!({
		"reason": version.reason,
		"prev_snapshot": version.prev_snapshot,
		"new_snapshot": version.new_snapshot,
		"ingest_decision": decision.map(|decision| serde_json::json!({
			"decision_id": decision.decision_id,
			"pipeline": decision.pipeline,
			"base_decision": decision.base_decision,
			"policy_decision": decision.policy_decision,
			"note_op": decision.note_op,
			"reason_code": decision.reason_code,
		})),
	});

	MemoryHistoryEvent {
		event_id: format!("memory_note_versions:{}", version.version_id),
		event_type: event_type.to_string(),
		subject_type: "note".to_string(),
		note_id: version.note_id,
		source_table: "memory_note_versions".to_string(),
		source_id: Some(version.version_id),
		related_note_version_id: Some(version.version_id),
		related_decision_id,
		related_proposal_id: None,
		actor: Some(version.actor.clone()),
		op: Some(version.op.clone()),
		reason_code: None,
		summary: version_summary(event_type, version.reason.as_str()),
		details,
		ts: version.ts,
	}
}

fn decision_history_event(
	note_id: Uuid,
	decision: &NoteProvenanceIngestDecision,
) -> MemoryHistoryEvent {
	let event_type = decision_event_type(decision);
	let details = serde_json::json!({
		"pipeline": decision.pipeline,
		"note_type": decision.note_type,
		"note_key": decision.note_key,
		"base_decision": decision.base_decision,
		"policy_decision": decision.policy_decision,
		"note_op": decision.note_op,
		"details": decision.details,
	});

	MemoryHistoryEvent {
		event_id: format!("memory_ingest_decisions:{}", decision.decision_id),
		event_type: event_type.to_string(),
		subject_type: "note".to_string(),
		note_id,
		source_table: "memory_ingest_decisions".to_string(),
		source_id: Some(decision.decision_id),
		related_note_version_id: decision.note_version_id,
		related_decision_id: Some(decision.decision_id),
		related_proposal_id: None,
		actor: Some(decision.agent_id.clone()),
		op: Some(decision.note_op.clone()),
		reason_code: decision.reason_code.clone(),
		summary: decision_summary(event_type, decision),
		details,
		ts: decision.ts,
	}
}

fn expire_history_event(note: &MemoryNote, expires_at: OffsetDateTime) -> MemoryHistoryEvent {
	MemoryHistoryEvent {
		event_id: format!("memory_notes:{}:expire:{expires_at}", note.note_id),
		event_type: "expire".to_string(),
		subject_type: "note".to_string(),
		note_id: note.note_id,
		source_table: "memory_notes".to_string(),
		source_id: Some(note.note_id),
		related_note_version_id: None,
		related_decision_id: None,
		related_proposal_id: None,
		actor: Some(note.agent_id.clone()),
		op: Some("EXPIRE".to_string()),
		reason_code: None,
		summary: "Note reached its persisted expires_at timestamp.".to_string(),
		details: serde_json::json!({
			"status": note.status,
			"expires_at": expires_at,
		}),
		ts: expires_at,
	}
}

fn derived_proposal_history_event(
	note_id: Uuid,
	proposal: NoteDerivedProposalRow,
) -> MemoryHistoryEvent {
	MemoryHistoryEvent {
		event_id: format!("consolidation_proposals:{}", proposal.proposal_id),
		event_type: "derived".to_string(),
		subject_type: "note".to_string(),
		note_id,
		source_table: "consolidation_proposals".to_string(),
		source_id: Some(proposal.proposal_id),
		related_note_version_id: None,
		related_decision_id: None,
		related_proposal_id: Some(proposal.proposal_id),
		actor: Some(proposal.agent_id),
		op: Some(proposal.apply_intent.clone()),
		reason_code: None,
		summary: format!(
			"Derived proposal '{}' was created with review_state '{}'.",
			proposal.proposal_kind, proposal.review_state
		),
		details: serde_json::json!({
			"run_id": proposal.run_id,
			"proposal_kind": proposal.proposal_kind,
			"apply_intent": proposal.apply_intent,
			"review_state": proposal.review_state,
			"source_refs": proposal.source_refs,
			"source_snapshot": proposal.source_snapshot,
			"lineage": proposal.lineage,
			"diff": proposal.diff,
			"confidence": proposal.confidence,
			"target_ref": proposal.target_ref,
			"proposed_payload": proposal.proposed_payload,
		}),
		ts: proposal.created_at,
	}
}

fn proposal_review_history_event(
	note_id: Uuid,
	review: NoteProposalReviewRow,
) -> MemoryHistoryEvent {
	let event_type = proposal_review_event_type(review.action.as_str());

	MemoryHistoryEvent {
		event_id: format!("consolidation_proposal_reviews:{}", review.review_id),
		event_type: event_type.to_string(),
		subject_type: "note".to_string(),
		note_id,
		source_table: "consolidation_proposal_reviews".to_string(),
		source_id: Some(review.review_id),
		related_note_version_id: None,
		related_decision_id: None,
		related_proposal_id: Some(review.proposal_id),
		actor: Some(review.reviewer_agent_id),
		op: Some(review.action.clone()),
		reason_code: None,
		summary: format!(
			"Proposal review action '{}' moved '{}' from '{}' to '{}'.",
			review.action, review.proposal_kind, review.from_review_state, review.to_review_state
		),
		details: serde_json::json!({
			"proposal_id": review.proposal_id,
			"run_id": review.run_id,
			"proposal_kind": review.proposal_kind,
			"apply_intent": review.apply_intent,
			"from_review_state": review.from_review_state,
			"to_review_state": review.to_review_state,
			"review_comment": review.review_comment,
			"diff": review.diff,
		}),
		ts: review.created_at,
	}
}

fn should_emit_decision_event(decision: &NoteProvenanceIngestDecision) -> bool {
	if matches!(decision.note_op.as_str(), "NONE" | "REJECTED") {
		return true;
	}

	decision.note_version_id.is_none()
}

fn version_event_type(op: &str, reason: &str) -> &'static str {
	let reason = reason.to_ascii_lowercase();

	match op {
		"ADD" => "add",
		"UPDATE" => "update",
		"DELETE" if reason.contains("expire") => "expire",
		"DELETE" => "delete",
		"PUBLISH" | "UNPUBLISH" => "related",
		"DEPRECATE" | "INVALIDATE" => "invalidated",
		_ => "related",
	}
}

fn decision_event_type(decision: &NoteProvenanceIngestDecision) -> &'static str {
	if decision.policy_decision == "reject" || decision.note_op == "REJECTED" {
		return "reject";
	}
	if decision.policy_decision == "ignore" || decision.note_op == "NONE" {
		return "ignore";
	}

	match decision.note_op.as_str() {
		"ADD" => "add",
		"UPDATE" => "update",
		"DELETE" => "delete",
		_ => "related",
	}
}

fn proposal_review_event_type(action: &str) -> &'static str {
	match action {
		"apply" => "applied",
		"discard" | "defer" => "invalidated",
		"approve" => "related",
		_ => "related",
	}
}

fn version_summary(event_type: &str, reason: &str) -> String {
	match event_type {
		"add" => format!("Note was added by {reason}."),
		"update" => format!("Note was updated by {reason}."),
		"delete" => format!("Note was deleted by {reason}."),
		"expire" => format!("Note expired through {reason}."),
		"invalidated" => format!("Note was invalidated by {reason}."),
		_ => format!("Note recorded related transition {reason}."),
	}
}

fn decision_summary(event_type: &str, decision: &NoteProvenanceIngestDecision) -> String {
	let reason = decision.reason_code.as_deref().unwrap_or("no_reason_code");

	match event_type {
		"ignore" => format!("Ingestion ignored candidate memory with {reason}."),
		"reject" => format!("Ingestion rejected candidate memory with {reason}."),
		_ => format!(
			"Ingestion recorded {} decision for operation {}.",
			decision.policy_decision, decision.note_op
		),
	}
}

async fn load_ingest_decisions(
	pool: &PgPool,
	req: &ValidatedNoteProvenanceRequest,
) -> Result<Vec<NoteProvenanceIngestDecision>> {
	let rows: Vec<NoteIngestDecisionRow> = sqlx::query_as::<_, NoteIngestDecisionRow>(
		"\
SELECT
	decision_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	pipeline,
	note_type,
	note_key,
	note_id,
	note_version_id,
	base_decision,
	policy_decision,
	note_op,
	reason_code,
	details,
	ts
FROM memory_ingest_decisions
WHERE note_id = $1 AND tenant_id = $2 AND project_id = $3
ORDER BY ts DESC
LIMIT $4",
	)
	.bind(req.note_id)
	.bind(&req.tenant_id)
	.bind(&req.project_id)
	.bind(NOTE_PROVENANCE_INGEST_DECISIONS_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows.into_iter().map(NoteProvenanceIngestDecision::from).collect())
}

async fn load_note_versions(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	note_id: Uuid,
) -> Result<Vec<NoteProvenanceNoteVersion>> {
	let rows: Vec<NoteVersionRow> = sqlx::query_as::<_, NoteVersionRow>(
		"\
SELECT
	memory_note_versions.version_id,
	memory_note_versions.note_id,
	memory_note_versions.op,
	memory_note_versions.prev_snapshot,
	memory_note_versions.new_snapshot,
	memory_note_versions.reason,
	memory_note_versions.actor,
	memory_note_versions.ts
FROM memory_note_versions
JOIN memory_notes n ON n.note_id = memory_note_versions.note_id
WHERE memory_note_versions.note_id = $1
	AND n.tenant_id = $2
	AND n.project_id = $3
ORDER BY memory_note_versions.ts DESC
LIMIT $4",
	)
	.bind(note_id)
	.bind(tenant_id)
	.bind(project_id)
	.bind(NOTE_PROVENANCE_NOTE_VERSIONS_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows.into_iter().map(NoteProvenanceNoteVersion::from).collect())
}

async fn load_indexing_outbox(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	note_id: Uuid,
) -> Result<Vec<NoteProvenanceIndexingOutbox>> {
	let rows: Vec<NoteIndexingOutboxRow> = sqlx::query_as::<_, NoteIndexingOutboxRow>(
		"\
SELECT
	indexing_outbox.outbox_id,
	indexing_outbox.note_id,
	indexing_outbox.op,
	indexing_outbox.embedding_version,
	indexing_outbox.status,
	indexing_outbox.attempts,
	indexing_outbox.last_error,
	indexing_outbox.available_at,
	indexing_outbox.created_at,
	indexing_outbox.updated_at
FROM indexing_outbox
JOIN memory_notes n ON n.note_id = indexing_outbox.note_id
WHERE indexing_outbox.note_id = $1
	AND n.tenant_id = $2
	AND n.project_id = $3
ORDER BY indexing_outbox.updated_at DESC
LIMIT $4",
	)
	.bind(note_id)
	.bind(tenant_id)
	.bind(project_id)
	.bind(NOTE_PROVENANCE_OUTBOX_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows.into_iter().map(NoteProvenanceIndexingOutbox::from).collect())
}

async fn load_recent_traces_for_note(
	pool: &PgPool,
	tenant_id: &str,
	project_id: &str,
	note_id: Uuid,
) -> Result<Vec<NoteProvenanceRecentTrace>> {
	let rows: Vec<NoteRecentTraceRow> = sqlx::query_as::<_, NoteRecentTraceRow>(
		"\
SELECT
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	created_at
FROM search_traces
WHERE tenant_id = $1
	AND project_id = $2
	AND trace_id IN (SELECT DISTINCT trace_id FROM search_trace_items WHERE note_id = $3)
ORDER BY created_at DESC, trace_id DESC
LIMIT $4",
	)
	.bind(tenant_id)
	.bind(project_id)
	.bind(note_id)
	.bind(NOTE_PROVENANCE_RECENT_TRACES_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows.into_iter().map(to_recent_trace).collect())
}

async fn load_memory_history_events(
	pool: &PgPool,
	req: &ValidatedNoteProvenanceRequest,
	note: &MemoryNote,
) -> Result<Vec<MemoryHistoryEvent>> {
	let decisions = load_ingest_decisions(pool, req).await?;
	let versions = load_note_versions(pool, &req.tenant_id, &req.project_id, req.note_id).await?;
	let proposal_ref = serde_json::json!([{ "kind": "note", "id": req.note_id }]);
	let proposals = load_derived_proposals_for_note(pool, req, &proposal_ref).await?;
	let reviews = load_proposal_reviews_for_note(pool, req, &proposal_ref).await?;
	let mut decision_by_version = HashMap::new();

	for decision in &decisions {
		if let Some(version_id) = decision.note_version_id {
			decision_by_version.insert(version_id, decision);
		}
	}

	let mut events = Vec::new();

	for version in &versions {
		events.push(version_history_event(version, decision_by_version.get(&version.version_id)));
	}
	for decision in &decisions {
		if should_emit_decision_event(decision) {
			events.push(decision_history_event(req.note_id, decision));
		}
	}

	if let Some(expires_at) = note.expires_at
		&& expires_at <= OffsetDateTime::now_utc()
		&& !events.iter().any(|event| event.event_type == "expire")
	{
		events.push(expire_history_event(note, expires_at));
	}

	for proposal in proposals {
		events.push(derived_proposal_history_event(req.note_id, proposal));
	}
	for review in reviews {
		events.push(proposal_review_history_event(req.note_id, review));
	}

	events.sort_by(|left, right| {
		left.ts.cmp(&right.ts).then_with(|| left.event_id.cmp(&right.event_id))
	});

	let history_limit = NOTE_PROVENANCE_HISTORY_LIMIT as usize;

	if events.len() > history_limit {
		let drop_count = events.len() - history_limit;

		events.drain(0..drop_count);
	}

	Ok(events)
}

async fn load_derived_proposals_for_note(
	pool: &PgPool,
	req: &ValidatedNoteProvenanceRequest,
	proposal_ref: &Value,
) -> Result<Vec<NoteDerivedProposalRow>> {
	let rows = sqlx::query_as::<_, NoteDerivedProposalRow>(
		"\
SELECT
	proposal_id,
	run_id,
	agent_id,
	proposal_kind,
	apply_intent,
	review_state,
	source_refs,
	source_snapshot,
	lineage,
	diff,
	confidence,
	COALESCE(target_ref, '{}'::jsonb) AS target_ref,
	COALESCE(proposed_payload, '{}'::jsonb) AS proposed_payload,
	created_at
FROM consolidation_proposals
WHERE tenant_id = $1
	AND project_id = $2
	AND source_refs @> $3
ORDER BY created_at DESC, proposal_id DESC
LIMIT $4",
	)
	.bind(&req.tenant_id)
	.bind(&req.project_id)
	.bind(proposal_ref)
	.bind(NOTE_PROVENANCE_HISTORY_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows)
}

async fn load_proposal_reviews_for_note(
	pool: &PgPool,
	req: &ValidatedNoteProvenanceRequest,
	proposal_ref: &Value,
) -> Result<Vec<NoteProposalReviewRow>> {
	let rows = sqlx::query_as::<_, NoteProposalReviewRow>(
		"\
SELECT
	reviews.review_id,
	reviews.proposal_id,
	reviews.run_id,
	reviews.reviewer_agent_id,
	reviews.action,
	reviews.from_review_state,
	reviews.to_review_state,
	reviews.review_comment,
	reviews.created_at,
	proposals.proposal_kind,
	proposals.apply_intent,
	proposals.diff
FROM consolidation_proposal_reviews reviews
JOIN consolidation_proposals proposals
	ON proposals.proposal_id = reviews.proposal_id
WHERE reviews.tenant_id = $1
	AND reviews.project_id = $2
	AND proposals.source_refs @> $3
ORDER BY reviews.created_at DESC, reviews.review_id DESC
LIMIT $4",
	)
	.bind(&req.tenant_id)
	.bind(&req.project_id)
	.bind(proposal_ref)
	.bind(NOTE_PROVENANCE_HISTORY_LIMIT)
	.fetch_all(pool)
	.await?;

	Ok(rows)
}

#[cfg(test)]
mod tests {
	use uuid::Uuid;

	use crate::provenance::{self, Error, NoteProvenanceGetRequest};

	#[test]
	fn normalize_note_provenance_request_trims_ids() {
		let request = NoteProvenanceGetRequest {
			tenant_id: "  tenant-a  ".to_string(),
			project_id: " project-a\n".to_string(),
			note_id: Uuid::new_v4(),
		};
		let result =
			provenance::validate_note_provenance_request(request).expect("expected valid request");

		assert_eq!(result.tenant_id, "tenant-a");
		assert_eq!(result.project_id, "project-a");
	}

	#[test]
	fn note_provenance_request_requires_tenant_and_project() {
		let missing_tenant = NoteProvenanceGetRequest {
			tenant_id: "   ".to_string(),
			project_id: "project-a".to_string(),
			note_id: Uuid::new_v4(),
		};
		let empty_project = NoteProvenanceGetRequest {
			tenant_id: "tenant-a".to_string(),
			project_id: "   ".to_string(),
			note_id: Uuid::new_v4(),
		};
		let first = provenance::validate_note_provenance_request(missing_tenant)
			.expect_err("expected tenant validation error");
		let second = provenance::validate_note_provenance_request(empty_project)
			.expect_err("expected project validation error");

		match first {
			Error::InvalidRequest { message } => {
				assert!(message.contains("tenant_id"));
			},
			_ => panic!("tenant validation should produce InvalidRequest"),
		}
		match second {
			Error::InvalidRequest { message } => {
				assert!(message.contains("tenant_id") || message.contains("project_id"));
			},
			_ => panic!("project validation should produce InvalidRequest"),
		}
	}
}
