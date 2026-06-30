use serde_json::Value;

use crate::search::filter::{
	expr::FilterExpr,
	parser::{FilterParseError, FilterParseState},
};

pub(in crate::search::filter) fn parse_expr(
	value: &Value,
	path: &str,
	depth: usize,
	state: &mut FilterParseState,
) -> Result<FilterExpr, FilterParseError> {
	FilterExpr::validate_metrics(path, depth, state)?;

	let Some(map) = value.as_object() else {
		return Err(FilterParseError {
			path: path.to_string(),
			message: "filter node must be an object.".to_string(),
		});
	};
	let op = map.get("op").and_then(Value::as_str).ok_or_else(|| FilterParseError {
		path: path.to_string(),
		message: "filter node is missing required string op.".to_string(),
	})?;

	match op {
		"and" => {
			let args = map.get("args").ok_or_else(|| FilterParseError {
				path: format!("{path}.args"),
				message: "and node requires args.".to_string(),
			})?;
			let args = FilterExpr::parse_args(args, &format!("{path}.args"), depth, state)?;

			Ok(FilterExpr::And(args))
		},
		"or" => {
			let args = map.get("args").ok_or_else(|| FilterParseError {
				path: format!("{path}.args"),
				message: "or node requires args.".to_string(),
			})?;
			let args = FilterExpr::parse_args(args, &format!("{path}.args"), depth, state)?;

			Ok(FilterExpr::Or(args))
		},
		"not" => {
			let expr = map.get("expr").ok_or_else(|| FilterParseError {
				path: format!("{path}.expr"),
				message: "not node requires expr.".to_string(),
			})?;
			let child = parse_expr(expr, &format!("{path}.expr"), depth.saturating_add(1), state)?;

			Ok(FilterExpr::Not(Box::new(child)))
		},
		"in" => FilterExpr::parse_leaf(map, op, path),
		"eq" | "neq" | "gt" | "gte" | "lt" | "lte" | "contains" =>
			FilterExpr::parse_leaf(map, op, path),
		_ => Err(FilterParseError {
			path: path.to_string(),
			message: format!("unsupported filter op '{op}'."),
		}),
	}
}
