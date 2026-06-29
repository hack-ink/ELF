use crate::{
	Error,
	add_note::{
		types::{AddNoteInput, AddNoteRequest},
		validation,
	},
};

#[test]
fn accepts_identifier_like_source_ref_ref_field() {
	validation::validate_add_note_request(&AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: Some("test_key".to_string()),
			text: "English text".to_string(),
			structured: None,
			importance: 0.5,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!({"ref": "packages/elf-service/src/docs.rs:661"}),
			write_policy: None,
		}],
	})
	.expect("Expected identifier-like source_ref to be accepted.");
}

#[test]
fn rejects_non_english_source_ref_hints_quote() {
	let req = AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: Some("test_key".to_string()),
			text: "English text".to_string(),
			structured: None,
			importance: 0.5,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!({"hints": {"quote": "\u{4f60}\u{597d}\u{4e16}\u{754c}"}}),
			write_policy: None,
		}],
	};
	let err = validation::validate_add_note_request(&req)
		.expect_err("Expected non-English free-text under source_ref.hints.quote to be rejected.");

	match err {
		Error::NonEnglishInput { field } => {
			assert_eq!(field, "$.notes[0].source_ref[\"hints\"][\"quote\"]")
		},
		other => panic!("Unexpected error: {other:?}"),
	}
}

#[test]
fn rejects_long_non_english_note_text() {
	let req = AddNoteRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: "agent_private".to_string(),
			notes: vec![AddNoteInput {
				r#type: "fact".to_string(),
				key: Some("test_key".to_string()),
				text: "Bonjour, je veux m'assurer que ce texte est suffisamment long et riche en lettres pour declencher la detection de langue. Merci beaucoup."
					.to_string(),
					structured: None,
					importance: 0.5,
					confidence: 0.9,
					ttl_days: None,
					source_ref: serde_json::json!({}),
					write_policy: None,
				}],
			};
	let err =
		validation::validate_add_note_request(&req).expect_err("Expected English gate rejection.");

	assert!(matches!(
		err,
		Error::NonEnglishInput { field } if field == "$.notes[0].text"
	));
}

#[test]
fn accepts_missing_source_ref_and_defaults_to_empty_object() {
	let req: AddNoteRequest = serde_json::from_value(serde_json::json!({
		"tenant_id": "t",
		"project_id": "p",
		"agent_id": "a",
		"scope": "agent_private",
		"notes": [
			{
				"type": "fact",
				"text": "English text.",
				"importance": 0.5,
				"confidence": 0.9
			}
		]
	}))
	.expect("Expected request to deserialize with default source_ref.");

	assert_eq!(req.notes[0].source_ref, serde_json::json!({}));

	validation::validate_add_note_request(&req)
		.expect("Expected missing source_ref to be accepted.");
}

#[test]
fn accepts_null_source_ref_and_normalizes_to_empty_object() {
	let req = AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: Some("test_key".to_string()),
			text: "English text.".to_string(),
			structured: None,
			importance: 0.5,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!(null),
			write_policy: None,
		}],
	};
	let req = validation::normalize_add_note_request(req);

	assert_eq!(req.notes[0].source_ref, serde_json::json!({}));

	validation::validate_add_note_request(&req).expect("Expected null source_ref to be accepted.");
}

#[test]
fn rejects_non_object_source_ref() {
	let req = AddNoteRequest {
		tenant_id: "t".to_string(),
		project_id: "p".to_string(),
		agent_id: "a".to_string(),
		scope: "agent_private".to_string(),
		notes: vec![AddNoteInput {
			r#type: "fact".to_string(),
			key: Some("test_key".to_string()),
			text: "English text.".to_string(),
			structured: None,
			importance: 0.5,
			confidence: 0.9,
			ttl_days: None,
			source_ref: serde_json::json!("legacy-shape"),
			write_policy: None,
		}],
	};
	let err = validation::validate_add_note_request(&req)
		.expect_err("Expected non-object source_ref rejection.");

	match err {
		Error::InvalidRequest { message } => {
			assert_eq!(message, "source_ref must be a JSON object.");
		},
		other => panic!("Expected InvalidRequest for non-object source_ref, got {other:?}"),
	}
}
