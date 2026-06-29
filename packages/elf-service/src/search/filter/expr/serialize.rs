use serde_json::Value;

use crate::search::filter::{expr::FilterExpr, value::FilterValue};

impl FilterExpr {
	pub(in crate::search::filter) fn to_value(&self) -> Value {
		match self {
			Self::And(exprs) => {
				serde_json::json!({ "op": "and", "args": Value::Array(exprs.iter().map(Self::to_value).collect()) })
			},
			Self::Or(exprs) => {
				serde_json::json!({ "op": "or", "args": Value::Array(exprs.iter().map(Self::to_value).collect()) })
			},
			Self::Not(expr) => {
				serde_json::json!({ "op": "not", "expr": expr.to_value() })
			},
			Self::Eq { field, value } => {
				serde_json::json!({ "op": "eq", "field": field.as_str(), "value": value.to_value() })
			},
			Self::Neq { field, value } => {
				serde_json::json!({ "op": "neq", "field": field.as_str(), "value": value.to_value() })
			},
			Self::In { field, values } => {
				serde_json::json!({
					"op": "in",
					"field": field.as_str(),
					"value": Value::Array(values.iter().map(FilterValue::to_value).collect())
				})
			},
			Self::Contains { field, value } => {
				serde_json::json!({ "op": "contains", "field": field.as_str(), "value": value })
			},
			Self::Gt { field, value } => {
				serde_json::json!({ "op": "gt", "field": field.as_str(), "value": value.to_value() })
			},
			Self::Gte { field, value } => {
				serde_json::json!({ "op": "gte", "field": field.as_str(), "value": value.to_value() })
			},
			Self::Lt { field, value } => {
				serde_json::json!({ "op": "lt", "field": field.as_str(), "value": value.to_value() })
			},
			Self::Lte { field, value } => {
				serde_json::json!({ "op": "lte", "field": field.as_str(), "value": value.to_value() })
			},
		}
	}
}
