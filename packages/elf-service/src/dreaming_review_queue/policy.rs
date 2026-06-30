mod actions;
mod refs;
mod summary;
mod variants;

pub(in crate::dreaming_review_queue) use self::{
	actions::{available_review_actions, bounded_queue_limit, policy_reason},
	refs::{affected_refs, contains_forbidden_source_mutation_key, non_empty_json_array},
	summary::summarize_items,
	variants::{high_impact_variant, low_risk_derived_organization, queue_variant_for},
};

/// Schema identifier for Dreaming review queue responses.
pub const ELF_DREAMING_REVIEW_QUEUE_SCHEMA_V1: &str = "elf.dreaming_review_queue/v1";

pub(super) const HIGH_CONFIDENCE_AUTO_APPLY_FLOOR: f32 = 0.9;
