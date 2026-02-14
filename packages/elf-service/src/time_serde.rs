pub mod option;

use serde::{Deserialize, Deserializer, Serializer};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub fn serialize<S>(value: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let formatted = value.format(&Rfc3339).map_err(serde::ser::Error::custom)?;

	serializer.serialize_str(&formatted)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
	D: Deserializer<'de>,
{
	let raw = String::deserialize(deserializer)?;

	OffsetDateTime::parse(&raw, &Rfc3339).map_err(serde::de::Error::custom)
}
