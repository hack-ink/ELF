use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{NoteOp, Result};
use elf_domain::{memory_policy::MemoryPolicyDecision, writegate::WritePolicyAudit};

pub(crate) struct IngestAuditArgs<'a> {
	pub tenant_id: &'a str,
	pub project_id: &'a str,
	pub agent_id: &'a str,
	pub scope: &'a str,
	pub pipeline: &'a str,
	pub note_type: &'a str,
	pub note_key: Option<&'a str>,
	pub note_id: Option<Uuid>,
	pub base_decision: MemoryPolicyDecision,
	pub policy_decision: MemoryPolicyDecision,
	pub note_op: NoteOp,
	pub reason_code: Option<&'a str>,
	pub similarity_best: Option<f32>,
	pub key_match: bool,
	pub matched_dup: bool,
	pub dup_sim_threshold: f32,
	pub update_sim_threshold: f32,
	pub confidence: f32,
	pub importance: f32,
	pub structured_present: bool,
	pub graph_present: bool,
	pub policy_rule: Option<&'a str>,
	pub min_confidence: Option<f32>,
	pub min_importance: Option<f32>,
	pub write_policy_audits: Option<Vec<WritePolicyAudit>>,
	pub ingestion_profile_id: Option<&'a str>,
	pub ingestion_profile_version: Option<i32>,
	pub ts: OffsetDateTime,
}

pub(crate) async fn insert_ingest_decision(
	tx: &mut Transaction<'_, Postgres>,
	args: IngestAuditArgs<'_>,
) -> Result<()> {
	let IngestAuditArgs {
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
		similarity_best,
		key_match,
		matched_dup,
		dup_sim_threshold,
		update_sim_threshold,
		confidence,
		importance,
		structured_present,
		graph_present,
		policy_rule,
		min_confidence,
		min_importance,
		write_policy_audits,
		ingestion_profile_id,
		ingestion_profile_version,
		ts,
	} = args;

	sqlx::query(
		"\
INSERT INTO memory_ingest_decisions (
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
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
	)
	.bind(Uuid::new_v4())
	.bind(tenant_id)
	.bind(project_id)
	.bind(agent_id)
	.bind(scope)
	.bind(pipeline)
	.bind(note_type)
	.bind(note_key)
	.bind(note_id)
	.bind(memory_policy_decision_to_str(base_decision))
	.bind(memory_policy_decision_to_str(policy_decision))
	.bind(note_op_to_str(note_op))
	.bind(reason_code)
	.bind(serde_json::json!({
		"similarity_best": similarity_best,
		"key_match": key_match,
		"matched_dup": matched_dup,
		"dup_sim_threshold": dup_sim_threshold,
		"update_sim_threshold": update_sim_threshold,
		"confidence": confidence,
		"importance": importance,
		"structured_present": structured_present,
		"graph_present": graph_present,
		"policy_rule": policy_rule,
		"min_confidence": min_confidence,
		"min_importance": min_importance,
		"write_policy_audits": write_policy_audits,
		"ingestion_profile": ingestion_profile_id.zip(ingestion_profile_version).map(
			|(id, version)| serde_json::json!({ "id": id, "version": version }),
		),
	}))
	.bind(ts)
	.execute(&mut **tx)
	.await?;

	Ok(())
}

fn memory_policy_decision_to_str(decision: MemoryPolicyDecision) -> &'static str {
	match decision {
		MemoryPolicyDecision::Remember => "remember",
		MemoryPolicyDecision::Update => "update",
		MemoryPolicyDecision::Ignore => "ignore",
		MemoryPolicyDecision::Reject => "reject",
	}
}

fn note_op_to_str(op: NoteOp) -> &'static str {
	match op {
		NoteOp::Add => "ADD",
		NoteOp::Update => "UPDATE",
		NoteOp::None => "NONE",
		NoteOp::Delete => "DELETE",
		NoteOp::Rejected => "REJECTED",
	}
}
