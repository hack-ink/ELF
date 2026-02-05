// crates.io
use serde::{Deserialize, Deserializer, Serializer, de::Error as DeError, ser::Error as SerError};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub fn serialize<S>(value: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let formatted = value.format(&Rfc3339).map_err(SerError::custom)?;
	serializer.serialize_str(&formatted)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
	D: Deserializer<'de>,
{
	let raw = String::deserialize(deserializer)?;
	OffsetDateTime::parse(&raw, &Rfc3339).map_err(DeError::custom)
}

pub mod option {
	use super::*;

	pub fn serialize<S>(value: &Option<OffsetDateTime>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match value {
			Some(value) => super::serialize(value, serializer),
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
				OffsetDateTime::parse(&value, &Rfc3339).map(Some).map_err(DeError::custom),
			None => Ok(None),
		}
	}
}
