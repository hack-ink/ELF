use crate::{
	Error,
	add_event::{
		types::{AddEventRequest, EventMessage},
		validation,
	},
};

#[test]
fn rejects_long_non_english_message_content() {
	let req = AddEventRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "a".to_string(),
			scope: None,
			dry_run: None,
			ingestion_profile: None,
			messages: vec![EventMessage {
				role: "user".to_string(),
					content: "Bonjour, je veux m'assurer que ce texte est suffisamment long et riche en lettres pour declencher la detection de langue. Merci beaucoup."
						.to_string(),
					ts: None,
					msg_id: None,
					write_policy: None,
				}],
			};
	let err =
		validation::validate_add_event_request(&req).expect_err("Expected English gate rejection.");

	assert!(matches!(
		err,
		Error::NonEnglishInput { field } if field == "$.messages[0].content"
	));
}
