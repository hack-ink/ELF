mod embedding;
mod graph_queries;
mod notes;
mod policy;
mod service_setup;

pub(super) use self::{
	graph_queries::{
		graph_fact_count, graph_fact_evidence_count, graph_fact_evidence_count_for_note,
		graph_fact_id,
	},
	notes::{add_fact_note, duplicate_fact_attaches_multiple_evidence_request},
	policy::assert_graph_policy_from_op,
	service_setup::{
		build_hash_service, build_service_with_extractor_payload, build_stub_service,
		build_test_db, reset_service_db,
	},
};

pub(super) const TEST_TENANT: &str = "t";
pub(super) const TEST_PROJECT: &str = "p";
pub(super) const TEST_SCOPE: &str = "agent_private";
pub(super) const GRAPH_REL_SUBJECT: &str = "alice";
pub(super) const GRAPH_REL_PREDICATE: &str = "mentors";
pub(super) const GRAPH_REL_OBJECT: &str = "Bob";
