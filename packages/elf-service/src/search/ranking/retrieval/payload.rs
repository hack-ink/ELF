use std::collections::HashMap;

use qdrant_client::qdrant::{PointId, Value, point_id::PointIdOptions, value::Kind};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub(super) fn point_id_to_uuid(point_id: &PointId) -> Option<Uuid> {
	match &point_id.point_id_options {
		Some(PointIdOptions::Uuid(id)) => Uuid::parse_str(id).ok(),
		_ => None,
	}
}

pub(super) fn payload_uuid(payload: &HashMap<String, Value>, key: &str) -> Option<Uuid> {
	let value = payload.get(key)?;

	match &value.kind {
		Some(Kind::StringValue(text)) => Uuid::parse_str(text).ok(),
		_ => None,
	}
}

pub(super) fn payload_string(payload: &HashMap<String, Value>, key: &str) -> Option<String> {
	let value = payload.get(key)?;

	match &value.kind {
		Some(Kind::StringValue(text)) => Some(text.to_string()),
		_ => None,
	}
}

pub(super) fn payload_rfc3339(
	payload: &HashMap<String, Value>,
	key: &str,
) -> Option<OffsetDateTime> {
	let text = payload_string(payload, key)?;

	OffsetDateTime::parse(text.as_str(), &Rfc3339).ok()
}

pub(super) fn payload_i32(payload: &HashMap<String, Value>, key: &str) -> Option<i32> {
	let value = payload.get(key)?;

	match &value.kind {
		Some(Kind::IntegerValue(value)) => i32::try_from(*value).ok(),
		Some(Kind::DoubleValue(value)) =>
			if value.fract() == 0.0 {
				i32::try_from(*value as i64).ok()
			} else {
				None
			},
		_ => None,
	}
}
