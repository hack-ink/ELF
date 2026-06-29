use std::collections::HashSet;

use serde_json::{self, Map, Value};
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::types::{
	ELF_WORK_JOURNAL_SCHEMA_V1, MAX_BODY_CHARS, MAX_SIDE_LIST_ITEMS, ValidatedWorkJournalCreate,
	WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1, WorkJournalEntryCreateRequest,
	WorkJournalEntryFamily, WorkJournalEntryResponse, WorkJournalWhereStopped,
};
use crate::{
	ElfService, Error, Result,
	access::{self, ORG_PROJECT_ID, SharedSpaceGrantKey},
};
use elf_config::Config;
use elf_domain::{
	english_gate,
	writegate::{self, WritePolicyAudit},
};
use elf_storage::{
	consolidation,
	models::{MemoryNote, WorkJournalEntry},
};

mod common;
mod context;
mod create;
mod promotion;
mod read;
mod refs;

use self::{
	common::{object_string, validate_json_strings, validate_natural_language},
	context::validate_write_context,
};

pub(super) use self::{
	common::validate_identifier,
	context::validate_read_context,
	create::validate_work_journal_create,
	promotion::{normalize_promotion_boundary, resolve_promotion_boundary_authority},
	read::{
		build_where_stopped, load_work_journal_shared_grants, row_to_response,
		work_journal_read_allowed,
	},
	refs::validate_source_refs,
};
