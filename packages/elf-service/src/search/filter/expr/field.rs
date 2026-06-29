use serde_json::Value;

use crate::search::NoteMeta;

use super::super::{parser::FilterParseError, value::FilterNodeValue};

#[derive(Clone, Debug)]
pub(in crate::search::filter) enum FilterField {
	Type,
	Key,
	Scope,
	AgentId,
	Importance,
	Confidence,
	UpdatedAt,
	ExpiresAt,
	HitCount,
	LastHitAt,
}

impl FilterField {
	pub(in crate::search::filter) fn as_str(&self) -> &'static str {
		match self {
			Self::Type => "type",
			Self::Key => "key",
			Self::Scope => "scope",
			Self::AgentId => "agent_id",
			Self::Importance => "importance",
			Self::Confidence => "confidence",
			Self::UpdatedAt => "updated_at",
			Self::ExpiresAt => "expires_at",
			Self::HitCount => "hit_count",
			Self::LastHitAt => "last_hit_at",
		}
	}

	pub(in crate::search::filter) fn parse(
		path: &str,
		raw: &Value,
	) -> Result<Self, FilterParseError> {
		let field = raw
			.as_str()
			.ok_or_else(|| FilterParseError {
				path: path.to_string(),
				message: "filter field must be a string.".to_string(),
			})?
			.to_ascii_lowercase();

		match field.as_str() {
			"type" => Ok(Self::Type),
			"key" => Ok(Self::Key),
			"scope" => Ok(Self::Scope),
			"agent_id" => Ok(Self::AgentId),
			"importance" => Ok(Self::Importance),
			"confidence" => Ok(Self::Confidence),
			"updated_at" => Ok(Self::UpdatedAt),
			"expires_at" => Ok(Self::ExpiresAt),
			"hit_count" => Ok(Self::HitCount),
			"last_hit_at" => Ok(Self::LastHitAt),
			_ => Err(FilterParseError {
				path: path.to_string(),
				message: format!(
					"field '{}' is not in allowlist: type, key, scope, agent_id, importance, confidence, updated_at, expires_at, hit_count, last_hit_at",
					field,
				),
			}),
		}
	}

	pub(in crate::search::filter) fn lookup_note_value(&self, note: &NoteMeta) -> FilterNodeValue {
		match self {
			Self::Type => FilterNodeValue::String(note.note_type.clone()),
			Self::Key => FilterNodeValue::String(note.key.clone().unwrap_or_default()),
			Self::Scope => FilterNodeValue::String(note.scope.clone()),
			Self::AgentId => FilterNodeValue::String(note.agent_id.clone()),
			Self::Importance => FilterNodeValue::Number(note.importance as f64),
			Self::Confidence => FilterNodeValue::Number(note.confidence as f64),
			Self::HitCount => FilterNodeValue::Number(note.hit_count as f64),
			Self::UpdatedAt => FilterNodeValue::DateTime(note.updated_at),
			Self::ExpiresAt =>
				note.expires_at.map_or(FilterNodeValue::Null, FilterNodeValue::DateTime),
			Self::LastHitAt =>
				note.last_hit_at.map_or(FilterNodeValue::Null, FilterNodeValue::DateTime),
		}
	}
}
