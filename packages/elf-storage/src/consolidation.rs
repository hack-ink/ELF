//! Consolidation run and proposal persistence queries.

mod jobs;
mod proposal_reviews;
mod proposals;
mod runs;
mod sql;
mod types;

pub use self::{
	jobs::{
		claim_next_consolidation_run_job, insert_consolidation_run_job,
		mark_consolidation_run_job_done, mark_consolidation_run_job_failed,
	},
	proposal_reviews::{
		insert_consolidation_proposal_review_event, list_consolidation_proposal_review_events,
	},
	proposals::{
		get_consolidation_proposal, insert_consolidation_proposal, list_consolidation_proposals,
		lock_consolidation_proposal, update_consolidation_proposal_review,
		update_consolidation_proposal_target_ref,
	},
	runs::{
		get_consolidation_run, insert_consolidation_run, list_consolidation_runs,
		update_consolidation_run_state,
	},
	types::{
		ConsolidationProposalReviewEventInsert, ConsolidationProposalReviewUpdate,
		ConsolidationProposalTargetRefUpdate, ConsolidationRunJobInsert,
		ConsolidationRunStateUpdate,
	},
};
