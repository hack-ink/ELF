//! Consolidation proposal contract validation.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

mod error;
mod lifecycle;
mod markers;
mod proposal;
mod sources;

pub use self::{error::*, lifecycle::*, markers::*, proposal::*, sources::*};

/// Current consolidation contract schema identifier.
pub const CONSOLIDATION_CONTRACT_SCHEMA_V1: &str = "elf.consolidation/v1";

const FORBIDDEN_DIFF_KEYS: [&str; 7] = [
	"delete_source",
	"delete_sources",
	"source_delete",
	"source_mutation",
	"source_mutations",
	"source_note_updates",
	"overwrite_source",
];

fn validate_json_object(
	field: &'static str,
	value: &Value,
) -> Result<(), ConsolidationValidationError> {
	if matches!(value, Value::Object(_)) {
		Ok(())
	} else {
		Err(ConsolidationValidationError::InvalidJsonObject { field })
	}
}

fn non_empty_object(value: &Value) -> bool {
	match value {
		Value::Object(map) => !map.is_empty(),
		_ => false,
	}
}

fn contains_forbidden_diff_key(value: &Value) -> bool {
	match value {
		Value::Object(map) => map.iter().any(|(key, nested)| {
			FORBIDDEN_DIFF_KEYS.contains(&key.as_str()) || contains_forbidden_diff_key(nested)
		}),
		Value::Array(items) => items.iter().any(contains_forbidden_diff_key),
		_ => false,
	}
}
