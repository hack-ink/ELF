use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{NoteOp, structured_fields::StructuredFields};
use elf_domain::{
	memory_policy::MemoryPolicyDecision,
	writegate::{WritePolicy, WritePolicyAudit},
};

use crate::ingestion_profiles::{IngestionProfileRef, IngestionProfileSelector};

pub(super) type ProcessedEventOutput =
	(Vec<EventMessage>, Vec<bool>, Option<Vec<WritePolicyAudit>>);
pub(super) type AddEventPersistOutput = (AddEventResult, Option<Uuid>);

/// One chat or event message passed to the event extractor.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventMessage {
	/// Speaker or message role.
	pub role: String,
	/// Message body content.
	pub content: String,
	/// Optional source timestamp string.
	pub ts: Option<String>,
	/// Optional message identifier from the upstream source.
	pub msg_id: Option<String>,
	/// Optional write policy applied before extraction.
	pub write_policy: Option<WritePolicy>,
}

/// Request payload for event-driven note extraction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddEventRequest {
	/// Tenant that owns the request.
	pub tenant_id: String,
	/// Project that owns the request.
	pub project_id: String,
	/// Agent that emitted the event batch.
	pub agent_id: String,
	/// Optional explicit scope override for extracted notes.
	pub scope: Option<String>,
	/// When true, performs validation and extraction without persisting notes.
	pub dry_run: Option<bool>,
	/// Optional ingestion profile selector.
	pub ingestion_profile: Option<IngestionProfileSelector>,
	/// Source messages to extract notes from.
	pub messages: Vec<EventMessage>,
}

/// Per-note outcome for an `add_event` request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddEventResult {
	/// Note identifier when one was created or updated.
	pub note_id: Option<Uuid>,
	/// Persistence operation chosen for the extracted note.
	pub op: NoteOp,
	/// Memory-policy decision applied to the extracted note.
	pub policy_decision: MemoryPolicyDecision,
	/// Machine-readable rejection or ignore code, if any.
	pub reason_code: Option<String>,
	/// Human-readable rejection or ignore message, if any.
	pub reason: Option<String>,
	/// Field path associated with a validation failure, if any.
	pub field_path: Option<String>,
	/// Per-message write-policy audits when write policies were applied.
	pub write_policy_audits: Option<Vec<WritePolicyAudit>>,
}

/// Response payload for event-driven note extraction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddEventResponse {
	/// Raw structured extractor output after normalization.
	pub extracted: Value,
	/// One result per extracted note.
	pub results: Vec<AddEventResult>,
	/// Resolved ingestion profile used for the request.
	pub ingestion_profile: Option<IngestionProfileRef>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ExtractorOutput {
	pub notes: Vec<ExtractedNote>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ExtractedNote {
	pub r#type: Option<String>,
	pub key: Option<String>,
	pub text: Option<String>,
	pub structured: Option<StructuredFields>,
	pub importance: Option<f32>,
	pub confidence: Option<f32>,
	pub ttl_days: Option<i64>,
	pub scope_suggestion: Option<String>,
	pub evidence: Option<Vec<EvidenceQuote>>,
	pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct EvidenceQuote {
	pub message_index: usize,
	pub quote: String,
}

pub(super) struct NoteProcessingData {
	pub(super) note_type: String,
	pub(super) text: String,
	pub(super) structured: Option<StructuredFields>,
	pub(super) importance: f32,
	pub(super) confidence: f32,
	pub(super) reason: Option<String>,
	pub(super) ttl_days: Option<i64>,
	pub(super) scope: String,
	pub(super) evidence: Vec<EvidenceQuote>,
	pub(super) structured_present: bool,
	pub(super) graph_present: bool,
}
impl NoteProcessingData {
	pub(super) fn from_request_and_note(req: &AddEventRequest, note: &ExtractedNote) -> Self {
		let note_type = note.r#type.clone().unwrap_or_default();
		let text = note.text.clone().unwrap_or_default();
		let structured = note.structured.clone();
		let structured_present =
			structured.as_ref().is_some_and(|value| !value.is_effectively_empty());
		let graph_present = structured.as_ref().is_some_and(StructuredFields::has_graph_fields);

		Self {
			note_type,
			text,
			structured,
			importance: note.importance.unwrap_or(0.0),
			confidence: note.confidence.unwrap_or(0.0),
			reason: note.reason.clone(),
			ttl_days: note.ttl_days,
			scope: req.scope.clone().or(note.scope_suggestion.clone()).unwrap_or_default(),
			evidence: note.evidence.clone().unwrap_or_default(),
			structured_present,
			graph_present,
		}
	}
}

pub(super) struct PersistExtractedNoteArgs<'a> {
	pub(super) req: &'a AddEventRequest,
	pub(super) project_id: &'a str,
	pub(super) structured: Option<&'a StructuredFields>,
	pub(super) key: Option<&'a str>,
	pub(super) reason: Option<&'a String>,
	pub(super) note_type: &'a str,
	pub(super) text: &'a str,
	pub(super) scope: &'a str,
	pub(super) importance: f32,
	pub(super) confidence: f32,
	pub(super) expires_at: Option<OffsetDateTime>,
	pub(super) source_ref: Value,
	pub(super) now: OffsetDateTime,
	pub(super) embed_version: &'a str,
}

pub(super) struct AddEventContext<'a> {
	pub(super) tenant_id: &'a str,
	pub(super) project_id: &'a str,
	pub(super) agent_id: &'a str,
	pub(super) scope: &'a str,
	pub(super) now: OffsetDateTime,
}
