use uuid::Uuid;

use super::{types::NoteProvenanceGetRequest, validation};
use crate::Error;

#[test]
fn normalize_note_provenance_request_trims_ids() {
	let request = NoteProvenanceGetRequest {
		tenant_id: "  tenant-a  ".to_string(),
		project_id: " project-a\n".to_string(),
		note_id: Uuid::new_v4(),
	};
	let result =
		validation::validate_note_provenance_request(request).expect("expected valid request");

	assert_eq!(result.tenant_id, "tenant-a");
	assert_eq!(result.project_id, "project-a");
}

#[test]
fn note_provenance_request_requires_tenant_and_project() {
	let missing_tenant = NoteProvenanceGetRequest {
		tenant_id: "   ".to_string(),
		project_id: "project-a".to_string(),
		note_id: Uuid::new_v4(),
	};
	let empty_project = NoteProvenanceGetRequest {
		tenant_id: "tenant-a".to_string(),
		project_id: "   ".to_string(),
		note_id: Uuid::new_v4(),
	};
	let first = validation::validate_note_provenance_request(missing_tenant)
		.expect_err("expected tenant validation error");
	let second = validation::validate_note_provenance_request(empty_project)
		.expect_err("expected project validation error");

	match first {
		Error::InvalidRequest { message } => {
			assert!(message.contains("tenant_id"));
		},
		_ => panic!("tenant validation should produce InvalidRequest"),
	}
	match second {
		Error::InvalidRequest { message } => {
			assert!(message.contains("tenant_id") || message.contains("project_id"));
		},
		_ => panic!("project validation should produce InvalidRequest"),
	}
}
