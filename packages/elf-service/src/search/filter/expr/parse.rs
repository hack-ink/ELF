use serde_json::{Map, Value};

use crate::search::filter::{
	expr::{FilterExpr, FilterField},
	parser::{
		self, FilterParseError, FilterParseState, MAX_FILTER_DEPTH, MAX_FILTER_NODES,
		MAX_IN_LIST_ITEMS,
	},
	value::FilterValue,
};

impl FilterExpr {
	pub(in crate::search::filter) fn parse_args(
		value: &Value,
		path: &str,
		depth: usize,
		state: &mut FilterParseState,
	) -> Result<Vec<Self>, FilterParseError> {
		let nodes = value.as_array().ok_or_else(|| FilterParseError {
			path: path.to_string(),
			message: "op args must be an array.".to_string(),
		})?;

		if nodes.is_empty() {
			return Err(FilterParseError {
				path: path.to_string(),
				message: "op args must contain at least one node.".to_string(),
			});
		}

		nodes
			.iter()
			.enumerate()
			.map(|(index, node)| {
				let child_path = format!("{path}[{index}]");

				parser::parse_expr(node, &child_path, depth.saturating_add(1), state)
			})
			.collect()
	}

	fn parse_in_values(
		field: &FilterField,
		value: &Value,
		path: &str,
	) -> Result<Vec<FilterValue>, FilterParseError> {
		let values = value.as_array().ok_or_else(|| FilterParseError {
			path: path.to_string(),
			message: "in value must be an array.".to_string(),
		})?;

		if values.len() > MAX_IN_LIST_ITEMS {
			return Err(FilterParseError {
				path: path.to_string(),
				message: format!(
					"in list exceeds maximum size ({}/{})",
					values.len(),
					MAX_IN_LIST_ITEMS
				),
			});
		}

		values
			.iter()
			.enumerate()
			.map(|(index, raw)| {
				let item_path = format!("{path}[{index}]");

				parser::parse_value(field, raw, &item_path)
			})
			.collect()
	}

	pub(in crate::search::filter) fn validate_metrics(
		path: &str,
		depth: usize,
		state: &mut FilterParseState,
	) -> Result<(), FilterParseError> {
		state.nodes = state.nodes.saturating_add(1);
		state.max_depth = state.max_depth.max(depth);

		if state.nodes > MAX_FILTER_NODES {
			return Err(FilterParseError {
				path: path.to_string(),
				message: format!(
					"filter exceeds node limit ({}/{})",
					state.nodes, MAX_FILTER_NODES
				),
			});
		}
		if state.max_depth > MAX_FILTER_DEPTH {
			return Err(FilterParseError {
				path: path.to_string(),
				message: format!(
					"filter exceeds depth limit ({}/{})",
					state.max_depth, MAX_FILTER_DEPTH
				),
			});
		}

		Ok(())
	}

	pub(in crate::search::filter) fn parse_leaf(
		raw: &Map<String, Value>,
		op: &str,
		path: &str,
	) -> Result<Self, FilterParseError> {
		let field = FilterField::parse(
			&format!("{path}.field"),
			raw.get("field").ok_or_else(|| FilterParseError {
				path: format!("{path}.field"),
				message: "op node is missing required field 'field'.".to_string(),
			})?,
		)?;
		let path_value = format!("{path}.value");
		let value_raw = raw.get("value").ok_or_else(|| FilterParseError {
			path: format!("{path}.value"),
			message: "op node is missing required field 'value'.".to_string(),
		})?;
		let value = parser::parse_value(&field, value_raw, &path_value)?;

		match op {
			"eq" => Ok(Self::Eq { field, value }),
			"neq" => Ok(Self::Neq { field, value }),
			"contains" => match value {
				FilterValue::String(value) => Ok(Self::Contains { field, value }),
				_ => Err(FilterParseError {
					path: path_value,
					message: "contains requires a string value.".to_string(),
				}),
			},
			"gt" => Ok(Self::Gt { field, value }),
			"gte" => Ok(Self::Gte { field, value }),
			"lt" => Ok(Self::Lt { field, value }),
			"lte" => Ok(Self::Lte { field, value }),
			"in" => {
				let values = Self::parse_in_values(&field, value_raw, &path_value)?;

				Ok(Self::In { field, values })
			},
			_ => Err(FilterParseError {
				path: path.to_string(),
				message: format!("unsupported leaf op '{op}'."),
			}),
		}
	}
}
