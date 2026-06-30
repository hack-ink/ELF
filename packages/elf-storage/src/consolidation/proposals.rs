mod insert;
mod read;
mod update;

pub use self::{
	insert::insert_consolidation_proposal,
	read::{get_consolidation_proposal, list_consolidation_proposals, lock_consolidation_proposal},
	update::{update_consolidation_proposal_review, update_consolidation_proposal_target_ref},
};
