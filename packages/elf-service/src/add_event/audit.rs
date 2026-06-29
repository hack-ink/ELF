use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
	NoteOp, Result,
	add_event::types::{AddEventContext, ExtractedNote},
	ingest_audit::{self, IngestAuditArgs},
};
use elf_config::Config;
use elf_domain::{memory_policy::MemoryPolicyDecision, writegate::WritePolicyAudit};

#[allow(clippy::too_many_arguments)]
pub(super) async fn record_ingest_decision(
	tx: &mut Transaction<'_, Postgres>,
	cfg: &Config,
	ctx: &AddEventContext<'_>,
	note: &ExtractedNote,
	note_type: &str,
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
	ingestion_profile_id: Option<&str>,
	ingestion_profile_version: Option<i32>,
	structured_present: bool,
	graph_present: bool,
	write_policy_audits: Option<Vec<WritePolicyAudit>>,
) -> Result<()> {
	let args = IngestAuditArgs {
		tenant_id: ctx.tenant_id,
		project_id: ctx.project_id,
		agent_id: ctx.agent_id,
		scope: ctx.scope,
		pipeline: "add_event",
		note_type,
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
		confidence: note.confidence.unwrap_or(0.0),
		importance: note.importance.unwrap_or(0.0),
		structured_present,
		graph_present,
		policy_rule,
		min_confidence,
		min_importance,
		ingestion_profile_id,
		ingestion_profile_version,
		write_policy_audits,
		ts: ctx.now,
	};

	ingest_audit::insert_ingest_decision(tx, args).await
}
