use time::OffsetDateTime;

use super::{
	persistence::{
		detach_core_block_attachment, fetch_active_attachment_for_update,
		fetch_active_block_for_attachment, fetch_attached_block_rows, fetch_audit_history,
		insert_core_block, insert_core_block_event, update_core_block,
		upsert_core_block_attachment,
	},
	types::{
		CoreBlockAttachRequest, CoreBlockAttachResponse, CoreBlockDetachRequest,
		CoreBlockDetachResponse, CoreBlockEventInput, CoreBlockUpsertRequest,
		CoreBlockUpsertResponse, CoreBlocksGetRequest, CoreBlocksResponse,
		ELF_CORE_MEMORY_BLOCKS_SCHEMA_V1,
	},
	validation::{
		attachment_snapshot, block_read_allowed, block_snapshot, filter_visible_rows,
		prepare_attach_request, prepare_detach_request, prepare_get_request,
		prepare_upsert_request,
	},
};
use crate::{ElfService, Error, Result, access};

impl ElfService {
	/// Returns core memory blocks explicitly attached for one agent/read-profile pair.
	pub async fn core_blocks_get(&self, req: CoreBlocksGetRequest) -> Result<CoreBlocksResponse> {
		let prepared = prepare_get_request(&self.cfg, req)?;
		let rows = fetch_attached_block_rows(
			&self.db.pool,
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.agent_id.as_str(),
			prepared.read_profile.as_str(),
		)
		.await?;
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.agent_id.as_str(),
			prepared.allowed_scopes.iter().any(|scope| scope == "org_shared"),
		)
		.await?;
		let visible_rows = filter_visible_rows(rows, &prepared.allowed_scopes, &shared_grants);
		let block_ids = visible_rows.iter().map(|row| row.block_id).collect::<Vec<_>>();
		let audit_by_block = fetch_audit_history(&self.db.pool, &block_ids).await?;
		let items =
			visible_rows.into_iter().map(|row| row.into_item(&audit_by_block)).collect::<Vec<_>>();

		Ok(CoreBlocksResponse {
			schema: ELF_CORE_MEMORY_BLOCKS_SCHEMA_V1.to_string(),
			tenant_id: prepared.tenant_id,
			project_id: prepared.project_id,
			agent_id: prepared.agent_id,
			read_profile: prepared.read_profile,
			items,
		})
	}

	/// Creates or updates a core memory block and records append-only audit history.
	pub async fn core_block_upsert(
		&self,
		req: CoreBlockUpsertRequest,
	) -> Result<CoreBlockUpsertResponse> {
		let prepared = prepare_upsert_request(&self.cfg, req)?;
		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;
		let (row, prev_snapshot) = match prepared.block_id {
			Some(block_id) => update_core_block(&mut tx, &prepared, block_id, now).await?,
			None => (insert_core_block(&mut tx, &prepared, now).await?, None),
		};

		insert_core_block_event(
			&mut tx,
			CoreBlockEventInput {
				block_id: row.block_id,
				attachment_id: None,
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				actor_agent_id: prepared.agent_id.as_str(),
				event_type: if prepared.block_id.is_some() {
					"block_updated"
				} else {
					"block_created"
				},
				target_agent_id: None,
				read_profile: None,
				prev_snapshot,
				new_snapshot: Some(block_snapshot(&row)),
				reason: prepared.reason.as_str(),
				ts: now,
			},
		)
		.await?;

		tx.commit().await?;

		Ok(CoreBlockUpsertResponse { block: row.into_record() })
	}

	/// Attaches an active core block to one exact agent/read-profile pair.
	pub async fn core_block_attach(
		&self,
		req: CoreBlockAttachRequest,
	) -> Result<CoreBlockAttachResponse> {
		let prepared = prepare_attach_request(&self.cfg, req)?;
		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;
		let block = fetch_active_block_for_attachment(&mut tx, &prepared).await?;
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&mut *tx,
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.target_agent_id.as_str(),
			prepared.allowed_scopes.iter().any(|scope| scope == "org_shared"),
		)
		.await?;

		if !block_read_allowed(
			&block,
			prepared.target_agent_id.as_str(),
			&prepared.allowed_scopes,
			&shared_grants,
		) {
			return Err(Error::ScopeDenied {
				message: "Block scope is not allowed for this attachment.".to_string(),
			});
		}

		let attachment = upsert_core_block_attachment(&mut tx, &prepared, now).await?;

		insert_core_block_event(
			&mut tx,
			CoreBlockEventInput {
				block_id: attachment.block_id,
				attachment_id: Some(attachment.attachment_id),
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				actor_agent_id: prepared.agent_id.as_str(),
				event_type: "attachment_added",
				target_agent_id: Some(prepared.target_agent_id.as_str()),
				read_profile: Some(prepared.read_profile.as_str()),
				prev_snapshot: None,
				new_snapshot: Some(attachment_snapshot(&attachment)),
				reason: prepared.reason.as_str(),
				ts: now,
			},
		)
		.await?;

		tx.commit().await?;

		Ok(CoreBlockAttachResponse {
			attachment_id: attachment.attachment_id,
			block_id: attachment.block_id,
			target_agent_id: attachment.agent_id,
			read_profile: attachment.read_profile,
			attached_by_agent_id: attachment.attached_by_agent_id,
			attached_at: attachment.attached_at,
		})
	}

	/// Detaches an active core block attachment and records an audit event.
	pub async fn core_block_detach(
		&self,
		req: CoreBlockDetachRequest,
	) -> Result<CoreBlockDetachResponse> {
		let prepared = prepare_detach_request(req)?;
		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;
		let Some(prev) = fetch_active_attachment_for_update(&mut tx, &prepared).await? else {
			tx.commit().await?;

			return Ok(CoreBlockDetachResponse {
				attachment_id: prepared.attachment_id,
				detached: false,
			});
		};
		let updated = detach_core_block_attachment(&mut tx, &prepared, now).await?;

		insert_core_block_event(
			&mut tx,
			CoreBlockEventInput {
				block_id: updated.block_id,
				attachment_id: Some(updated.attachment_id),
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				actor_agent_id: prepared.agent_id.as_str(),
				event_type: "attachment_removed",
				target_agent_id: Some(updated.agent_id.as_str()),
				read_profile: Some(updated.read_profile.as_str()),
				prev_snapshot: Some(attachment_snapshot(&prev)),
				new_snapshot: Some(attachment_snapshot(&updated)),
				reason: prepared.reason.as_str(),
				ts: now,
			},
		)
		.await?;

		tx.commit().await?;

		Ok(CoreBlockDetachResponse { attachment_id: updated.attachment_id, detached: true })
	}
}
