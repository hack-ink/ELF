use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Clone, Debug)]
pub(super) enum FilterValue {
	String(String),
	Number(f64),
	DateTime(OffsetDateTime),
	Null,
}
impl FilterValue {
	pub(super) fn to_node_value(&self) -> FilterNodeValue {
		match self {
			Self::String(value) => FilterNodeValue::String(value.clone()),
			Self::Number(value) => FilterNodeValue::Number(*value),
			Self::DateTime(value) => FilterNodeValue::DateTime(*value),
			Self::Null => FilterNodeValue::Null,
		}
	}

	pub(super) fn to_value(&self) -> Value {
		match self {
			Self::String(value) => Value::String(value.clone()),
			Self::Number(value) => serde_json::json!(value),
			Self::DateTime(value) => Value::String(value.format(&Rfc3339).unwrap_or_default()),
			Self::Null => Value::Null,
		}
	}

	pub(super) fn to_numeric(&self) -> f64 {
		match self {
			Self::Number(value) => *value,
			_ => 0.0,
		}
	}
}

impl PartialEq for FilterValue {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::String(lhs), Self::String(rhs)) => lhs == rhs,
			(Self::Number(lhs), Self::Number(rhs)) => lhs == rhs,
			(Self::DateTime(lhs), Self::DateTime(rhs)) => lhs == rhs,
			(Self::Null, Self::Null) => true,
			_ => false,
		}
	}
}

#[derive(Clone, Debug)]
pub(super) enum FilterNodeValue {
	String(String),
	Number(f64),
	DateTime(OffsetDateTime),
	Null,
}
impl From<&FilterValue> for FilterNodeValue {
	fn from(value: &FilterValue) -> Self {
		match value {
			FilterValue::String(value) => Self::String(value.clone()),
			FilterValue::Number(value) => Self::Number(*value),
			FilterValue::DateTime(value) => Self::DateTime(*value),
			FilterValue::Null => Self::Null,
		}
	}
}

impl PartialEq for FilterNodeValue {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::String(lhs), Self::String(rhs)) => lhs == rhs,
			(Self::Number(lhs), Self::Number(rhs)) => lhs == rhs,
			(Self::DateTime(lhs), Self::DateTime(rhs)) => lhs == rhs,
			(Self::Null, Self::Null) => true,
			_ => false,
		}
	}
}
