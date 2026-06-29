//! Source-adjacent Work Journal capture and readback APIs.

mod service;
#[cfg(test)] mod tests;
mod types;
mod validation;

pub use types::{
	ELF_WORK_JOURNAL_SCHEMA_V1, WorkJournalEntryCreateRequest, WorkJournalEntryCreateResponse,
	WorkJournalEntryFamily, WorkJournalEntryGetRequest, WorkJournalEntryResponse,
	WorkJournalSessionReadbackRequest, WorkJournalSessionReadbackResponse, WorkJournalWhereStopped,
};
