use time::OffsetDateTime;

use crate::{ElfService, NoteOp, Result};

use super::{
	storage::{
		RestoreNoteArgs, delete_note, load_note_for_correction, restore_note, supersede_note,
	},
	types::{MemoryCorrectionAction, MemoryCorrectionRequest, MemoryCorrectionResponse},
	validation::{validate_correction_request, validate_write_scope},
};

impl ElfService {
	/// Applies a review-backed memory correction and writes an audit version row.
	pub async fn memory_correction_apply(
		&self,
		req: MemoryCorrectionRequest,
	) -> Result<MemoryCorrectionResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let actor_agent_id = req.actor_agent_id.trim();
		let reason = req.reason.trim();

		validate_correction_request(
			tenant_id,
			project_id,
			actor_agent_id,
			reason,
			&req.source_ref,
		)?;

		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;
		let mut note =
			load_note_for_correction(&mut tx, req.note_id, tenant_id, project_id).await?;

		validate_write_scope(&note, &self.cfg.scopes)?;

		let version_id = match req.action {
			MemoryCorrectionAction::Supersede =>
				supersede_note(&mut tx, &mut note, actor_agent_id, reason, &req.source_ref, now)
					.await?,
			MemoryCorrectionAction::Delete =>
				delete_note(&mut tx, &mut note, actor_agent_id, reason, &req.source_ref, now)
					.await?,
			MemoryCorrectionAction::Restore => {
				let embed_version = crate::embedding_version(&self.cfg);

				restore_note(
					&mut tx,
					&mut note,
					RestoreNoteArgs {
						actor_agent_id,
						reason,
						correction_source_ref: &req.source_ref,
						restore_version_id: req.restore_version_id,
						embedding_version: embed_version.as_str(),
						now,
					},
				)
				.await?
			},
		};
		let op = match (req.action, version_id) {
			(_, None) => NoteOp::None,
			(MemoryCorrectionAction::Delete, Some(_)) => NoteOp::Delete,
			(MemoryCorrectionAction::Supersede | MemoryCorrectionAction::Restore, Some(_)) =>
				NoteOp::Update,
		};

		tx.commit().await?;

		Ok(MemoryCorrectionResponse {
			note_id: note.note_id,
			action: req.action,
			op,
			status: note.status,
			version_id,
		})
	}
}
