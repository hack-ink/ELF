use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
	NoteOp, Result,
	add_note::types::{AddNoteContext, AddNoteInput},
	ingest_audit::{self, IngestAuditArgs},
	structured_fields::StructuredFields,
};
use elf_config::Config;
use elf_domain::{memory_policy::MemoryPolicyDecision, writegate::WritePolicyAudit};

#[allow(clippy::too_many_arguments)]
pub(super) async fn record_ingest_decision(
	tx: &mut Transaction<'_, Postgres>,
	cfg: &Config,
	ctx: &AddNoteContext<'_>,
	note: &AddNoteInput,
	note_id: Option<Uuid>,
	note_version_id: Option<Uuid>,
	base_decision: MemoryPolicyDecision,
	policy_decision: MemoryPolicyDecision,
	note_op: NoteOp,
	reason_code: Option<&str>,
	policy_rule: Option<&str>,
	similarity_best: Option<f32>,
	key_match: bool,
	matched_dup: bool,
	min_confidence: Option<f32>,
	min_importance: Option<f32>,
	write_policy_audit: Option<WritePolicyAudit>,
) -> Result<()> {
	let decision = IngestAuditArgs {
		tenant_id: ctx.tenant_id,
		project_id: ctx.project_id,
		agent_id: ctx.agent_id,
		scope: ctx.scope,
		pipeline: "add_note",
		note_type: note.r#type.as_str(),
		note_key: note.key.as_deref(),
		note_id,
		note_version_id,
		base_decision,
		policy_decision,
		note_op,
		reason_code,
		similarity_best,
		key_match,
		matched_dup,
		dup_sim_threshold: cfg.memory.dup_sim_threshold,
		update_sim_threshold: cfg.memory.update_sim_threshold,
		confidence: note.confidence,
		importance: note.importance,
		structured_present: note.structured.as_ref().is_some_and(|s| !s.is_effectively_empty()),
		graph_present: note.structured.as_ref().is_some_and(StructuredFields::has_graph_fields),
		policy_rule,
		min_confidence,
		min_importance,
		write_policy_audits: write_policy_audit.map(|audit| vec![audit]),
		ingestion_profile_id: None,
		ingestion_profile_version: None,
		ts: ctx.now,
	};

	ingest_audit::insert_ingest_decision(tx, decision).await
}
