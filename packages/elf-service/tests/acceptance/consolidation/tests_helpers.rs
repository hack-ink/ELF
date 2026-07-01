mod corrections;
mod notes;
mod proposals;
mod refs;
mod service_setup;
mod worker_processing;

pub(super) use self::{
	corrections::{
		active_list_contains, apply_memory_correction, memory_history_event_types,
		promote_reviewed_memory,
	},
	notes::insert_source_note,
	proposals::{
		create_run_with_proposals, materialized_proposals, proposal_id_by_kind, proposal_input,
		proposal_input_with_payload,
	},
	refs::source_ref,
	service_setup::setup_service,
	worker_processing::process_consolidation_worker,
};

use elf_service::ElfService;
use elf_testkit::TestDatabase;

pub(super) const TENANT_ID: &str = "tenant_consolidation";
pub(super) const PROJECT_ID: &str = "project_consolidation";
pub(super) const AGENT_ID: &str = "agent_consolidation";
pub(super) const REVIEWER_ID: &str = "reviewer_consolidation";

pub(super) struct ConsolidationFixture {
	pub(super) service: ElfService,
	_test_db: TestDatabase,
}
