mod proposals;
mod runs;

pub(super) use self::{
	proposals::{
		__path_consolidation_proposal_get, __path_consolidation_proposal_review,
		__path_consolidation_proposals_list, consolidation_proposal_get,
		consolidation_proposal_review, consolidation_proposals_list,
	},
	runs::{
		__path_consolidation_run_create, __path_consolidation_run_get,
		__path_consolidation_runs_list, consolidation_run_create, consolidation_run_get,
		consolidation_runs_list,
	},
};
