use serde::{Deserialize, Serialize};
use serde_json::Value;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteProvenanceGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub note_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteProvenanceBundleResponse {
	pub schema: String,
	pub note: NoteProvenanceNote,
	pub ingest_decisions: Vec<NoteProvenanceIngestDecision>,
	pub note_versions: Vec<NoteProvenanceNoteVersion>,
	pub indexing_outbox: Vec<NoteProvenanceIndexingOutbox>,
	pub recent_traces: Vec<NoteProvenanceRecentTrace>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteProvenanceNote {
	pub note_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub r#type: String,
	pub key: Option<String>,
	pub text: String,
	pub importance: f32,
	pub confidence: f32,
	pub status: String,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub expires_at: Option<OffsetDateTime>,
	pub source_ref: Value,
	pub embedding_version: String,
	pub hit_count: i64,
	#[serde(with = "crate::time_serde::option")]
	pub last_hit_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteProvenanceIngestDecision {
	pub decision_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub pipeline: String,
	pub note_type: String,
	pub note_key: Option<String>,
	pub note_id: Option<Uuid>,
	pub base_decision: String,
	pub policy_decision: String,
	pub note_op: String,
	pub reason_code: Option<String>,
	pub details: Value,
	#[serde(with = "crate::time_serde")]
	pub ts: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteProvenanceNoteVersion {
	pub version_id: Uuid,
	pub note_id: Uuid,
	pub op: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub prev_snapshot: Option<Value>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_snapshot: Option<Value>,
	pub reason: String,
	pub actor: String,
	#[serde(with = "crate::time_serde")]
	pub ts: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteProvenanceIndexingOutbox {
	pub outbox_id: Uuid,
	pub note_id: Uuid,
	pub op: String,
	pub embedding_version: String,
	pub status: String,
	pub attempts: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_error: Option<String>,
	#[serde(with = "crate::time_serde")]
	pub available_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteProvenanceRecentTrace {
	pub trace_id: Uuid,
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub read_profile: String,
	pub query: String,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
}

impl ElfService {
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

		Ok(NoteProvenanceBundleResponse {
			schema: NOTE_PROVENANCE_BUNDLE_SCHEMA_V1.to_string(),
			note: NoteProvenanceNote::from(note_row),
			ingest_decisions,
			note_versions,
			indexing_outbox,
			recent_traces,
		})
	}
}

#[derive(Clone, Debug)]
struct ValidatedNoteProvenanceRequest {
	tenant_id: String,
	project_id: String,
	note_id: Uuid,
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
			base_decision: row.base_decision,
			policy_decision: row.policy_decision,
			note_op: row.note_op,
			reason_code: row.reason_code,
			details: row.details,
			ts: row.ts,
		}
	}
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
	version_id,
	note_id,
	op,
	prev_snapshot,
	new_snapshot,
	reason,
	actor,
	ts
FROM memory_note_versions
JOIN memory_notes n ON n.note_id = memory_note_versions.note_id
WHERE memory_note_versions.note_id = $1
	AND n.tenant_id = $2
	AND n.project_id = $3
ORDER BY ts DESC
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
	outbox_id,
	note_id,
	op,
	embedding_version,
	status,
	attempts,
	last_error,
	available_at,
	created_at,
	updated_at
FROM indexing_outbox
JOIN memory_notes n ON n.note_id = indexing_outbox.note_id
WHERE indexing_outbox.note_id = $1
	AND n.tenant_id = $2
	AND n.project_id = $3
ORDER BY updated_at DESC
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

#[cfg(test)]
mod tests {
	use super::{Error, NoteProvenanceGetRequest, validate_note_provenance_request};
	use uuid::Uuid;

	#[test]
	fn normalize_note_provenance_request_trims_ids() {
		let request = NoteProvenanceGetRequest {
			tenant_id: "  tenant-a  ".to_string(),
			project_id: " project-a\n".to_string(),
			note_id: Uuid::new_v4(),
		};
		let result = validate_note_provenance_request(request).expect("expected valid request");

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

		let first = validate_note_provenance_request(missing_tenant)
			.expect_err("expected tenant validation error");
		let second = validate_note_provenance_request(empty_project)
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
