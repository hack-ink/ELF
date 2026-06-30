mod add;
mod none;
mod update;

use sqlx::{Postgres, Transaction};

use crate::{
	Result, UpdateDecision,
	add_event::types::{AddEventPersistOutput, PersistExtractedNoteArgs},
};
use elf_domain::memory_policy::MemoryPolicyDecision;

pub(super) async fn persist_extracted_note_decision(
	tx: &mut Transaction<'_, Postgres>,
	args: PersistExtractedNoteArgs<'_>,
	decision: UpdateDecision,
	policy_decision: MemoryPolicyDecision,
) -> Result<AddEventPersistOutput> {
	match (decision, args) {
		(UpdateDecision::Add { note_id, .. }, args) =>
			add::persist_extracted_note_add(tx, args, note_id, policy_decision).await,
		(UpdateDecision::Update { note_id, .. }, args) =>
			update::persist_extracted_note_update(tx, args, note_id, policy_decision).await,
		(UpdateDecision::None { note_id, .. }, args) =>
			none::persist_extracted_note_none(tx, args, note_id, policy_decision).await,
	}
}
