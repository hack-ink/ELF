use serde_json::Value;
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(FromRow)]
pub(in crate::provenance) struct NoteIngestDecisionRow {
	pub(in crate::provenance) decision_id: Uuid,
	pub(in crate::provenance) tenant_id: String,
	pub(in crate::provenance) project_id: String,
	pub(in crate::provenance) agent_id: String,
	pub(in crate::provenance) scope: String,
	pub(in crate::provenance) pipeline: String,
	pub(in crate::provenance) note_type: String,
	pub(in crate::provenance) note_key: Option<String>,
	pub(in crate::provenance) note_id: Option<Uuid>,
	pub(in crate::provenance) note_version_id: Option<Uuid>,
	pub(in crate::provenance) base_decision: String,
	pub(in crate::provenance) policy_decision: String,
	pub(in crate::provenance) note_op: String,
	pub(in crate::provenance) reason_code: Option<String>,
	pub(in crate::provenance) details: Value,
	pub(in crate::provenance) ts: OffsetDateTime,
}

#[derive(FromRow)]
pub(in crate::provenance) struct NoteVersionRow {
	pub(in crate::provenance) version_id: Uuid,
	pub(in crate::provenance) note_id: Uuid,
	pub(in crate::provenance) op: String,
	pub(in crate::provenance) prev_snapshot: Option<Value>,
	pub(in crate::provenance) new_snapshot: Option<Value>,
	pub(in crate::provenance) reason: String,
	pub(in crate::provenance) actor: String,
	pub(in crate::provenance) ts: OffsetDateTime,
}

#[derive(FromRow)]
pub(in crate::provenance) struct NoteIndexingOutboxRow {
	pub(in crate::provenance) outbox_id: Uuid,
	pub(in crate::provenance) note_id: Uuid,
	pub(in crate::provenance) op: String,
	pub(in crate::provenance) embedding_version: String,
	pub(in crate::provenance) status: String,
	pub(in crate::provenance) attempts: i32,
	pub(in crate::provenance) last_error: Option<String>,
	pub(in crate::provenance) available_at: OffsetDateTime,
	pub(in crate::provenance) created_at: OffsetDateTime,
	pub(in crate::provenance) updated_at: OffsetDateTime,
}

#[derive(FromRow)]
pub(in crate::provenance) struct NoteRecentTraceRow {
	pub(in crate::provenance) trace_id: Uuid,
	pub(in crate::provenance) tenant_id: String,
	pub(in crate::provenance) project_id: String,
	pub(in crate::provenance) agent_id: String,
	pub(in crate::provenance) read_profile: String,
	pub(in crate::provenance) query: String,
	pub(in crate::provenance) created_at: OffsetDateTime,
}

#[derive(FromRow)]
pub(in crate::provenance) struct NoteDerivedProposalRow {
	pub(in crate::provenance) proposal_id: Uuid,
	pub(in crate::provenance) run_id: Uuid,
	pub(in crate::provenance) agent_id: String,
	pub(in crate::provenance) proposal_kind: String,
	pub(in crate::provenance) apply_intent: String,
	pub(in crate::provenance) review_state: String,
	pub(in crate::provenance) source_refs: Value,
	pub(in crate::provenance) source_snapshot: Value,
	pub(in crate::provenance) lineage: Value,
	pub(in crate::provenance) diff: Value,
	pub(in crate::provenance) confidence: f32,
	pub(in crate::provenance) target_ref: Value,
	pub(in crate::provenance) proposed_payload: Value,
	pub(in crate::provenance) created_at: OffsetDateTime,
}

#[derive(FromRow)]
pub(in crate::provenance) struct NoteProposalReviewRow {
	pub(in crate::provenance) review_id: Uuid,
	pub(in crate::provenance) proposal_id: Uuid,
	pub(in crate::provenance) run_id: Uuid,
	pub(in crate::provenance) reviewer_agent_id: String,
	pub(in crate::provenance) action: String,
	pub(in crate::provenance) from_review_state: String,
	pub(in crate::provenance) to_review_state: String,
	pub(in crate::provenance) review_comment: Option<String>,
	pub(in crate::provenance) created_at: OffsetDateTime,
	pub(in crate::provenance) proposal_kind: String,
	pub(in crate::provenance) apply_intent: String,
	pub(in crate::provenance) diff: Value,
}
