pub(crate) mod assertions;
pub(crate) mod inserts;
pub(crate) mod seed;
pub(crate) mod setup;

use uuid::Uuid;

use elf_service::ElfService;
use elf_testkit::TestDatabase;

pub(crate) const TENANT_ID: &str = "tenant_admin_scope";
pub(crate) const PROJECT_ID: &str = "project_admin_scope";
pub(crate) const TRACE_VERSION: i32 = 3;

pub(crate) struct TraceAdminObservabilityFixture {
	pub(crate) service: ElfService,
	pub(crate) test_db: TestDatabase,
}

pub(crate) struct VisibilityTraceFixtureIds {
	pub(crate) trace_one: Uuid,
	pub(crate) trace_two: Uuid,
	pub(crate) trace_three: Uuid,
	pub(crate) item_two: Uuid,
}
