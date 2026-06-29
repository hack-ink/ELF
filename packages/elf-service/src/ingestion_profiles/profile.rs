use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Error, Result};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct IngestionProfileV1 {
	#[serde(default = "default_schema_version")]
	pub(super) schema_version: i32,

	pub(super) prompt_schema: Option<Value>,

	pub(super) prompt_system_template: Option<String>,

	pub(super) prompt_user_template: Option<String>,

	pub(super) model: Option<String>,

	pub(super) temperature: Option<f32>,

	pub(super) timeout_ms: Option<u64>,
}
impl IngestionProfileV1 {
	pub(super) fn with_defaults(self) -> Self {
		let defaults = builtin_profile_v1();
		let mut merged = defaults;

		if self.schema_version != 0 {
			merged.schema_version = self.schema_version;
		}

		merged.prompt_schema = self.prompt_schema.or(merged.prompt_schema);
		merged.prompt_system_template =
			self.prompt_system_template.or(merged.prompt_system_template);
		merged.prompt_user_template = self.prompt_user_template.or(merged.prompt_user_template);
		merged.model = self.model.or(merged.model);
		merged.temperature = self.temperature.or(merged.temperature);
		merged.timeout_ms = self.timeout_ms.or(merged.timeout_ms);

		merged
	}
}

pub(super) fn parse_profile(profile: Value) -> Result<IngestionProfileV1> {
	let parsed = serde_json::from_value::<IngestionProfileV1>(profile.clone()).or_else(|_| {
		if profile.is_object() {
			Ok(IngestionProfileV1 {
				schema_version: 1,
				prompt_schema: Some(profile),
				prompt_system_template: None,
				prompt_user_template: None,
				model: None,
				temperature: None,
				timeout_ms: None,
			})
		} else {
			Err(Error::InvalidRequest {
				message: "Ingestion profile JSON has unsupported format.".to_string(),
			})
		}
	})?;

	Ok(parsed)
}

pub(super) fn builtin_profile_v1() -> IngestionProfileV1 {
	IngestionProfileV1 {
		schema_version: 1,
		prompt_schema: Some(builtin_profile_schema()),
		prompt_system_template: Some(
			"You are a memory extraction engine for an agent memory system. Output must be valid JSON only and must match the provided schema exactly. \
Extract at most MAX_NOTES high-signal, cross-session reusable memory notes from the given messages. \
Each note must be one English sentence and must not contain any non-English text. \
The structured field is optional. If present, summary must be short, facts must be short sentences supported by the evidence quotes, and concepts must be short phrases. \
structured.entities and structured.relations should mirror the structured schema with optional entity and relation metadata and relation timestamps. \
Preserve numbers, dates, percentages, currency amounts, tickers, URLs, and code snippets exactly. \
Never store secrets or PII: API keys, tokens, private keys, seed phrases, passwords, bank IDs, personal addresses. \
For every note, provide 1 to 2 evidence quotes copied verbatim from the input messages and include the message_index. \
If you cannot provide verbatim evidence, omit the note. \
If content is ephemeral or not useful long-term, return an empty notes array."
				.to_string(),
		),
		prompt_user_template: Some(
			"Return JSON matching this exact schema:\n{SCHEMA}\nConstraints:\n- MAX_NOTES = {MAX_NOTES}\n- MAX_NOTE_CHARS = {MAX_NOTE_CHARS}\nHere are the messages as JSON:\n{MESSAGES_JSON}"
				.to_string(),
		),
		model: None,
		temperature: None,
		timeout_ms: None,
	}
}

fn default_schema_version() -> i32 {
	1
}

fn builtin_profile_schema() -> Value {
	serde_json::json!({
		"notes": [
			{
				"type": "preference|constraint|decision|profile|fact|plan",
				"key": "string|null",
				"text": "English-only sentence <= MAX_NOTE_CHARS",
				"structured": {
					"summary": "string|null",
					"facts": "string[]|null",
					"concepts": "string[]|null",
					"entities": [
						{
							"canonical": "string|null",
							"kind": "string|null",
							"aliases": "string[]|null"
						}
					],
					"relations": [
						{
							"subject": {
								"canonical": "string|null",
								"kind": "string|null",
								"aliases": "string[]|null"
							},
							"predicate": "string",
							"object": {
								"entity": {
									"canonical": "string|null",
									"kind": "string|null",
									"aliases": "string[]|null"
								},
								"value": "string|null"
							},
							"valid_from": "string|null",
							"valid_to": "string|null"
						}
					]
				},
				"importance": 0.0,
				"confidence": 0.0,
				"ttl_days": "number|null",
				"scope_suggestion": "agent_private|project_shared|org_shared|null",
				"evidence": [
					{ "message_index": "number", "quote": "string" }
				],
				"reason": "string"
			}
		]
	})
}
