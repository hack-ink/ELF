mod evidence;
mod fixture;
mod refs;
mod review;
mod run;

use crate::{
	ConsolidationInputRef, ConsolidationMaterializationEvidence, ConsolidationProposalResponse,
	ConsolidationReviewAction, CorpusText, IngestedCorpus, LiveConsolidationFixture, LoadedJob,
	PreparedConsolidationRun, Result, Uuid, serde_json::Value,
};

pub(super) fn live_consolidation_fixture(loaded: &LoadedJob) -> Result<LiveConsolidationFixture> {
	fixture::live_consolidation_fixture(loaded)
}

pub(super) fn prepare_consolidation_run(
	loaded: &LoadedJob,
	adapter_id: &str,
	ingested: &IngestedCorpus,
	fixture: &LiveConsolidationFixture,
	corpus: &[CorpusText],
) -> Result<PreparedConsolidationRun> {
	run::prepare_consolidation_run(loaded, adapter_id, ingested, fixture, corpus)
}

pub(super) fn validate_reviewed_consolidation_count(
	loaded: &LoadedJob,
	fixture: &LiveConsolidationFixture,
	reviewed: &[ConsolidationProposalResponse],
) -> Result<()> {
	review::validate_reviewed_consolidation_count(loaded, fixture, reviewed)
}

pub(super) fn consolidation_materialization_evidence(
	run_id: Uuid,
	fixture: &LiveConsolidationFixture,
	input_refs: &[ConsolidationInputRef],
	reviewed: &[ConsolidationProposalResponse],
) -> ConsolidationMaterializationEvidence {
	evidence::consolidation_materialization_evidence(run_id, fixture, input_refs, reviewed)
}

pub(super) fn consolidation_review_action(raw: &str) -> Result<ConsolidationReviewAction> {
	review::consolidation_review_action(raw)
}

pub(super) fn live_consolidation_response(
	fixture: &LiveConsolidationFixture,
	reviewed: &[ConsolidationProposalResponse],
) -> Result<Value> {
	review::live_consolidation_response(fixture, reviewed)
}

pub(super) fn live_note_ids(ingested: &IngestedCorpus) -> Vec<Uuid> {
	refs::live_note_ids(ingested)
}
