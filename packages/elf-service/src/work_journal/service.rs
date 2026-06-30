use std::collections::HashSet;

use serde_json;
use time::OffsetDateTime;

use crate::{
	ElfService, Error, Result,
	access::{self, ORG_PROJECT_ID},
	search,
	work_journal::{
		types::{
			WorkJournalEntryCreateRequest, WorkJournalEntryCreateResponse, WorkJournalEntryFamily,
			WorkJournalEntryGetRequest, WorkJournalEntryResponse,
			WorkJournalSessionReadbackRequest, WorkJournalSessionReadbackResponse,
			constants::{
				DEFAULT_SESSION_READBACK_LIMIT, ELF_WORK_JOURNAL_SCHEMA_V1,
				MAX_SESSION_READBACK_LIMIT, MAX_STORAGE_SCAN_ROWS,
			},
		},
		validation::{self},
	},
};
use elf_storage::{models::WorkJournalEntry, work_journal};

impl ElfService {
	/// Captures one source-adjacent Work Journal entry.
	pub async fn work_journal_entry_create(
		&self,
		req: WorkJournalEntryCreateRequest,
	) -> Result<WorkJournalEntryCreateResponse> {
		let mut validated = validation::validate_work_journal_create(&self.cfg, &req)?;
		let now = OffsetDateTime::now_utc();
		let effective_project_id = if validated.scope == "org_shared" {
			ORG_PROJECT_ID.to_string()
		} else {
			req.project_id.trim().to_string()
		};
		let mut tx = self.db.pool.begin().await?;

		validated.promotion_boundary = validation::resolve_promotion_boundary_authority(
			&mut tx,
			&self.cfg,
			validated.promotion_boundary,
			req.tenant_id.trim(),
			req.project_id.trim(),
			req.agent_id.trim(),
			now,
		)
		.await?;

		let entry = WorkJournalEntry {
			entry_id: validated.entry_id,
			tenant_id: req.tenant_id.trim().to_string(),
			project_id: effective_project_id.clone(),
			agent_id: req.agent_id.trim().to_string(),
			scope: validated.scope,
			session_id: validated.session_id,
			family: req.family.as_str().to_string(),
			status: "active".to_string(),
			title: validated.title,
			body: validated.body,
			source_refs: validated.source_refs,
			explicit_next_steps: validated.explicit_next_steps,
			inferred_next_steps: validated.inferred_next_steps,
			rejected_options: validated.rejected_options,
			promotion_boundary: validated.promotion_boundary,
			redaction_audit: serde_json::to_value(validated.redaction_audit).map_err(|err| {
				Error::InvalidRequest { message: format!("redaction audit is invalid: {err}") }
			})?,
			created_at: now,
			updated_at: now,
		};

		work_journal::insert_work_journal_entry(&mut *tx, &entry).await?;

		if entry.scope != "agent_private" {
			access::ensure_active_project_scope_grant(
				&mut *tx,
				entry.tenant_id.as_str(),
				effective_project_id.as_str(),
				entry.scope.as_str(),
				entry.agent_id.as_str(),
			)
			.await?;
		}

		tx.commit().await?;

		Ok(WorkJournalEntryCreateResponse { entry: validation::row_to_response(entry)? })
	}

	/// Reads one source-adjacent Work Journal entry.
	pub async fn work_journal_entry_get(
		&self,
		req: WorkJournalEntryGetRequest,
	) -> Result<WorkJournalEntryResponse> {
		validation::validate_read_context(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			req.read_profile.as_str(),
		)?;

		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.trim())?;
		let shared_grants = validation::load_work_journal_shared_grants(
			self,
			req.tenant_id.trim(),
			req.project_id.trim(),
			req.agent_id.trim(),
			&allowed_scopes,
		)
		.await?;
		let row =
			work_journal::get_work_journal_entry(&self.db.pool, req.tenant_id.trim(), req.entry_id)
				.await?
				.ok_or_else(|| Error::NotFound {
					message: "Work Journal entry not found.".to_string(),
				})?;

		if row.project_id != req.project_id.trim() && row.project_id != ORG_PROJECT_ID {
			return Err(Error::NotFound { message: "Work Journal entry not found.".to_string() });
		}
		if !validation::work_journal_read_allowed(
			&row,
			req.agent_id.trim(),
			&allowed_scopes,
			&shared_grants,
		) {
			return Err(Error::ScopeDenied {
				message: "Work Journal entry is not readable by this agent.".to_string(),
			});
		}

		validation::row_to_response(row)
	}

	/// Reads newest-first Work Journal entries for one session.
	pub async fn work_journal_session_readback(
		&self,
		req: WorkJournalSessionReadbackRequest,
	) -> Result<WorkJournalSessionReadbackResponse> {
		validation::validate_read_context(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			req.read_profile.as_str(),
		)?;
		validation::validate_identifier(req.session_id.as_str(), "$.session_id")?;

		let limit = req
			.limit
			.unwrap_or(DEFAULT_SESSION_READBACK_LIMIT)
			.clamp(1, MAX_SESSION_READBACK_LIMIT);
		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.trim())?;
		let shared_grants = validation::load_work_journal_shared_grants(
			self,
			req.tenant_id.trim(),
			req.project_id.trim(),
			req.agent_id.trim(),
			&allowed_scopes,
		)
		.await?;
		let family_filter =
			req.families.iter().copied().collect::<HashSet<WorkJournalEntryFamily>>();
		let rows = work_journal::list_work_journal_entries_for_session(
			&self.db.pool,
			req.tenant_id.trim(),
			req.project_id.trim(),
			ORG_PROJECT_ID,
			req.session_id.trim(),
			MAX_STORAGE_SCAN_ROWS,
		)
		.await?;
		let mut items = Vec::new();

		for row in rows {
			if !family_filter.is_empty()
				&& !family_filter.contains(&WorkJournalEntryFamily::parse(row.family.as_str())?)
			{
				continue;
			}
			if !validation::work_journal_read_allowed(
				&row,
				req.agent_id.trim(),
				&allowed_scopes,
				&shared_grants,
			) {
				continue;
			}

			items.push(validation::row_to_response(row)?);

			if items.len() >= limit as usize {
				break;
			}
		}

		let where_stopped = validation::build_where_stopped(&items);

		Ok(WorkJournalSessionReadbackResponse {
			schema: ELF_WORK_JOURNAL_SCHEMA_V1.to_string(),
			session_id: req.session_id.trim().to_string(),
			items,
			where_stopped,
		})
	}
}
