use serde::{Deserialize as _, Deserializer, Serializer};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub fn serialize<S>(value: &Option<OffsetDateTime>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match value {
		Some(value) => crate::time_serde::serialize(value, serializer),
		None => serializer.serialize_none(),
	}
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
where
	D: Deserializer<'de>,
{
	let raw = Option::<String>::deserialize(deserializer)?;

	match raw {
		Some(value) =>
			OffsetDateTime::parse(&value, &Rfc3339).map(Some).map_err(serde::de::Error::custom),
		None => Ok(None),
	}
}
