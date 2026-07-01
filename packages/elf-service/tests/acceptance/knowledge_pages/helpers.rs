mod assertions;
mod request;
mod setup;
mod source_inserts;

pub(crate) use self::{
	assertions::assert_first_rebuild, request::knowledge_foundation_request, setup::setup_service,
	source_inserts::insert_rebuild_sources,
};

use uuid::Uuid;

use elf_service::ElfService;
use elf_testkit::TestDatabase;

pub(crate) const TENANT_ID: &str = "tenant_knowledge";
pub(crate) const PROJECT_ID: &str = "project_knowledge";
pub(crate) const AGENT_ID: &str = "agent_knowledge";

pub(crate) struct KnowledgeFixture {
	pub(crate) service: ElfService,
	pub(crate) _test_db: TestDatabase,
}

#[derive(Clone, Copy)]
pub(crate) struct KnowledgeSourceIds {
	pub(crate) note_id: Uuid,
	pub(crate) event_id: Uuid,
	pub(crate) doc_id: Uuid,
	pub(crate) chunk_id: Uuid,
	pub(crate) fact_id: Uuid,
	pub(crate) proposal_id: Uuid,
}
