mod evidence;
mod request;
mod structured;
mod write_policy;
mod writegate;

pub(super) use self::{
	evidence::reject_extracted_note_if_evidence_invalid, request::validate_add_event_request,
	structured::reject_extracted_note_if_structured_invalid,
	write_policy::apply_write_policies_to_messages,
	writegate::reject_extracted_note_if_writegate_rejects,
};

pub(super) const REJECT_STRUCTURED_INVALID: &str = "REJECT_STRUCTURED_INVALID";
