use crate::search::api::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

/// Payload-detail level used by search and trace APIs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PayloadLevel {
	#[default]
	/// Level 0 payloads.
	L0,
	/// Level 1 payloads.
	L1,
	/// Level 2 payloads.
	L2,
}
impl PayloadLevel {
	fn as_str(self) -> &'static str {
		match self {
			Self::L0 => "l0",
			Self::L1 => "l1",
			Self::L2 => "l2",
		}
	}

	fn parse(raw: &str) -> Option<Self> {
		match raw.to_ascii_lowercase().as_str() {
			"l0" => Some(Self::L0),
			"l1" => Some(Self::L1),
			"l2" => Some(Self::L2),
			_ => None,
		}
	}
}

impl Serialize for PayloadLevel {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_str().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for PayloadLevel {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let raw = String::deserialize(deserializer)?;

		Self::parse(&raw).ok_or_else(|| Error::custom("payload_level must be l0, l1, or l2"))
	}
}
