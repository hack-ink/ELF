use crate::{
	Error, Result,
	provenance::types::{NoteProvenanceGetRequest, requests::ValidatedNoteProvenanceRequest},
};

pub(super) fn validate_note_provenance_request(
	req: NoteProvenanceGetRequest,
) -> Result<ValidatedNoteProvenanceRequest> {
	let tenant_id = req.tenant_id.trim();
	let project_id = req.project_id.trim();

	if tenant_id.is_empty() || project_id.is_empty() {
		return Err(Error::InvalidRequest {
			message: "tenant_id and project_id are required.".to_string(),
		});
	}

	Ok(ValidatedNoteProvenanceRequest {
		tenant_id: tenant_id.to_string(),
		project_id: project_id.to_string(),
		note_id: req.note_id,
	})
}
