use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
	ElfService, NoteOp, Result, UpdateDecision,
	add_note::{
		materialize::{self},
		types::{AddNoteContext, AddNoteInput, AddNoteResult},
	},
	structured_fields::StructuredFields,
};
use elf_config::Config;
use elf_domain::memory_policy::{self, MemoryPolicyDecision};

type AddNoteApplyOutput = (AddNoteResult, NoteOp, Option<Uuid>);

const IGNORE_DUPLICATE: &str = "IGNORE_DUPLICATE";
const IGNORE_POLICY_THRESHOLD: &str = "IGNORE_POLICY_THRESHOLD";

pub(super) fn structured_and_graph_present(structured: Option<&StructuredFields>) -> (bool, bool) {
	let structured_present = structured.is_some_and(|s| !s.is_effectively_empty());
	let graph_present = structured.is_some_and(StructuredFields::has_graph_fields);

	(structured_present, graph_present)
}

pub(super) fn resolve_policy_for_update(
	cfg: &Config,
	scope: &str,
	note: &AddNoteInput,
	base_decision: MemoryPolicyDecision,
) -> (MemoryPolicyDecision, Option<String>, Option<f32>, Option<f32>) {
	if matches!(base_decision, MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update) {
		let policy_eval = memory_policy::evaluate_memory_policy(
			cfg,
			note.r#type.as_str(),
			scope,
			f64::from(note.confidence),
			f64::from(note.importance),
			base_decision,
		);
		let decision_policy_rule = policy_eval
			.matched_rule
			.and_then(|rule| policy_rule_id(rule.note_type.as_deref(), rule.scope.as_deref()));
		let min_confidence = policy_eval.matched_rule.and_then(|rule| rule.min_confidence);
		let min_importance = policy_eval.matched_rule.and_then(|rule| rule.min_importance);

		(policy_eval.decision, decision_policy_rule, min_confidence, min_importance)
	} else {
		(MemoryPolicyDecision::Ignore, None, None, None)
	}
}

pub(super) fn ignore_reason_code_for_policy(
	base_decision: MemoryPolicyDecision,
	policy_decision: MemoryPolicyDecision,
	matched_dup: bool,
) -> Option<&'static str> {
	if !matches!(policy_decision, MemoryPolicyDecision::Ignore) {
		return None;
	}

	match base_decision {
		MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update =>
			Some(IGNORE_POLICY_THRESHOLD),
		MemoryPolicyDecision::Ignore if matched_dup => Some(IGNORE_DUPLICATE),
		_ => None,
	}
}

pub(super) fn base_decision_for_update(
	decision: &UpdateDecision,
	structured_present: bool,
	graph_present: bool,
) -> MemoryPolicyDecision {
	match decision {
		UpdateDecision::Update { .. } => MemoryPolicyDecision::Update,
		UpdateDecision::Add { .. } => MemoryPolicyDecision::Remember,
		UpdateDecision::None { .. } =>
			if structured_present || graph_present {
				MemoryPolicyDecision::Update
			} else {
				MemoryPolicyDecision::Ignore
			},
	}
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_policy_result(
	service: &ElfService,
	tx: &mut Transaction<'_, Postgres>,
	decision: &UpdateDecision,
	ctx: &AddNoteContext<'_>,
	note: &AddNoteInput,
	note_id: Uuid,
	policy_decision: MemoryPolicyDecision,
	ignore_reason_code: Option<&'static str>,
) -> Result<AddNoteApplyOutput> {
	let should_apply =
		matches!(policy_decision, MemoryPolicyDecision::Remember | MemoryPolicyDecision::Update);

	if should_apply {
		let (result, note_version_id) = match decision {
			UpdateDecision::Add { .. } => {
				let note_version_id =
					materialize::handle_add_note_add(service, tx, ctx, note, note_id).await?;

				(
					AddNoteResult {
						note_id: Some(note_id),
						op: NoteOp::Add,
						policy_decision,
						reason_code: None,
						field_path: None,
						write_policy_audit: None,
					},
					Some(note_version_id),
				)
			},
			UpdateDecision::Update { .. } =>
				materialize::handle_add_note_update(
					service,
					tx,
					note,
					note_id,
					ctx.agent_id,
					ctx.now,
					policy_decision,
				)
				.await?,
			UpdateDecision::None { .. } => {
				let (mut none_result, note_version_id) = materialize::handle_add_note_none(
					tx,
					ctx,
					note,
					note_id,
					ctx.now,
					ctx.embed_version,
					policy_decision,
				)
				.await?;

				none_result.policy_decision = policy_decision;

				(none_result, note_version_id)
			},
		};
		let note_op = result.op;

		Ok((result, note_op, note_version_id))
	} else {
		let mut result = AddNoteResult {
			note_id: Some(note_id),
			op: NoteOp::None,
			policy_decision,
			reason_code: ignore_reason_code.map(str::to_string),
			field_path: None,
			write_policy_audit: None,
		};

		match decision {
			UpdateDecision::Add { .. } => {
				result.note_id = None;
			},
			UpdateDecision::Update { .. } | UpdateDecision::None { .. } => {},
		}

		Ok((result, NoteOp::None, None))
	}
}

fn policy_rule_id(note_type: Option<&str>, scope: Option<&str>) -> Option<String> {
	match (note_type, scope) {
		(Some(note_type), Some(scope)) => Some(format!("note_type={note_type},scope={scope}")),
		(Some(note_type), None) => Some(format!("note_type={note_type}")),
		(None, Some(scope)) => Some(format!("scope={scope}")),
		(None, None) => None,
	}
}
