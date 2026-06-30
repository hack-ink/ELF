mod build;
mod proposal;
mod refs;
mod run;

pub(in crate::knowledge) use self::{
	build::memory_candidates_for_page,
	proposal::candidate_proposal_input,
	run::{candidate_run_input_refs, knowledge_delta_source_snapshot, proposal_run_summary},
};
