use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use super::{
	OUTPUT_SCHEMA,
	types::{AgentmemoryObservation, AgentmemorySession, FixtureContext},
};

pub(super) fn observation_timestamp(
	session: &AgentmemorySession,
	observation: &AgentmemoryObservation,
	ctx: &FixtureContext,
) -> Option<String> {
	[observation.ts.as_deref(), session.started_at.as_deref(), ctx.exported_at.as_deref()]
		.into_iter()
		.flatten()
		.find_map(normalize_rfc3339)
}

fn normalize_rfc3339(value: &str) -> Option<String> {
	OffsetDateTime::parse(value, &Rfc3339)
		.ok()
		.and_then(|timestamp| timestamp.format(&Rfc3339).ok())
}

pub(super) fn map_note_type(kind: &str) -> Option<&'static str> {
	match kind.trim().to_ascii_lowercase().as_str() {
		"preference" => Some("preference"),
		"constraint" => Some("constraint"),
		"decision" => Some("decision"),
		"profile" => Some("profile"),
		"fact" => Some("fact"),
		"plan" => Some("plan"),
		_ => None,
	}
}

pub(super) fn score_or_default(score: Option<f32>, default: f32) -> Option<f32> {
	let score = score.unwrap_or(default);

	if score.is_finite() && (0.0..=1.0).contains(&score) { Some(score) } else { None }
}

pub(super) fn clean_string(value: Option<&str>) -> Option<String> {
	value.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string)
}

pub(super) fn stable_uuid(kind: &str, parts: &[&str]) -> Uuid {
	let mut key = format!("https://hack.ink/elf/{OUTPUT_SCHEMA}/{kind}");

	for part in parts {
		key.push('/');
		key.push_str(part);
	}

	Uuid::new_v5(&Uuid::NAMESPACE_URL, key.as_bytes())
}
