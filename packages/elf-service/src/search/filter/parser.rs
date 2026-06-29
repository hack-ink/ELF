use std::{
	collections::HashMap,
	fmt::{Display, Formatter},
};

use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::search::{
	ChunkCandidate, NoteMeta,
	filter::{
		expr::{FilterExpr, FilterField},
		impact::SearchFilterImpact,
		value::FilterValue,
	},
};

pub(super) const SEARCH_FILTER_EXPR_SCHEMA_V1: &str = "search_filter_expr/v1";
pub(super) const MAX_FILTER_DEPTH: usize = 8;
pub(super) const MAX_FILTER_NODES: usize = 128;
pub(super) const MAX_IN_LIST_ITEMS: usize = 128;
pub(super) const MAX_STRING_BYTES: usize = 512;

#[derive(Clone, Debug)]
pub(crate) struct FilterParseError {
	pub(super) path: String,
	pub(super) message: String,
}
impl Display for FilterParseError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}: {}", self.path, self.message)
	}
}

#[derive(Clone, Debug)]
pub(crate) struct SearchFilter {
	expr: FilterExpr,
	json: Value,
}
impl SearchFilter {
	pub(super) fn as_value(&self) -> Value {
		self.json.clone()
	}

	pub(super) fn evaluate(&self, note: &NoteMeta) -> (bool, Option<String>) {
		self.expr.evaluate(note)
	}

	pub(crate) fn parse(raw: &Value) -> Result<Self, FilterParseError> {
		let path = "$.filter";
		let obj = raw.as_object().ok_or_else(|| FilterParseError {
			path: path.to_string(),
			message: "filter must be an object.".to_string(),
		})?;
		let schema = obj.get("schema").and_then(Value::as_str).ok_or_else(|| FilterParseError {
			path: format!("{path}.schema"),
			message: "filter.schema is required.".to_string(),
		})?;

		if schema != SEARCH_FILTER_EXPR_SCHEMA_V1 {
			return Err(FilterParseError {
				path: format!("{path}.schema"),
				message: format!(
					"unsupported filter schema '{schema}', expected '{SEARCH_FILTER_EXPR_SCHEMA_V1}'."
				),
			});
		}

		let expr = obj.get("expr").ok_or_else(|| FilterParseError {
			path: format!("{path}.expr"),
			message: "filter.expr is required.".to_string(),
		})?;
		let mut state = FilterParseState::default();
		let parsed = parse_expr(expr, "$.filter.expr", 1, &mut state)?;

		Ok(Self {
			expr: parsed.clone(),
			json: serde_json::json!({"schema": SEARCH_FILTER_EXPR_SCHEMA_V1, "expr": parsed.to_value()}),
		})
	}

	pub(crate) fn eval(
		&self,
		candidates: Vec<ChunkCandidate>,
		note_meta: &HashMap<Uuid, NoteMeta>,
		requested_candidate_k: u32,
		effective_candidate_k: u32,
	) -> (Vec<ChunkCandidate>, SearchFilterImpact) {
		let impact = SearchFilterImpact::from_eval(
			self,
			candidates.as_slice(),
			note_meta,
			requested_candidate_k,
			effective_candidate_k,
		);
		let pre = candidates.len();
		let mut kept = Vec::with_capacity(impact.candidate_count_post);

		for candidate in candidates {
			let Some(note) = note_meta.get(&candidate.note_id) else {
				continue;
			};

			if self.expr.evaluate(note).0 {
				kept.push(candidate);
			}
		}

		let post = kept.len();

		(
			kept,
			SearchFilterImpact {
				candidate_count_post: post,
				dropped_total: pre.saturating_sub(post),
				..impact
			},
		)
	}
}

#[derive(Default)]
pub(super) struct FilterParseState {
	pub(super) nodes: usize,
	pub(super) max_depth: usize,
}

pub(super) fn parse_expr(
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

pub(super) fn parse_value(
	field: &FilterField,
	raw: &Value,
	path: &str,
) -> Result<FilterValue, FilterParseError> {
	match field {
		FilterField::Type | FilterField::Key | FilterField::Scope | FilterField::AgentId =>
			match raw {
				Value::String(_) | Value::Null if matches!(field, FilterField::Key) => {
					if raw.is_null() {
						Ok(FilterValue::Null)
					} else {
						parse_string(path, raw).map(FilterValue::String)
					}
				},
				_ => parse_string(path, raw).map(FilterValue::String),
			},
		FilterField::Importance | FilterField::Confidence | FilterField::HitCount => {
			let value = raw.as_f64().ok_or_else(|| FilterParseError {
				path: path.to_string(),
				message: "numeric value expected.".to_string(),
			})?;

			Ok(FilterValue::Number(value))
		},
		FilterField::UpdatedAt =>
			OffsetDateTime::parse(parse_string(path, raw)?.as_str(), &Rfc3339)
				.map(FilterValue::DateTime)
				.map_err(|_| FilterParseError {
					path: path.to_string(),
					message: "datetime value must be RFC3339.".to_string(),
				}),
		FilterField::ExpiresAt | FilterField::LastHitAt =>
			if raw.is_null() {
				Ok(FilterValue::Null)
			} else {
				OffsetDateTime::parse(parse_string(path, raw)?.as_str(), &Rfc3339)
					.map(FilterValue::DateTime)
					.map_err(|_| FilterParseError {
						path: path.to_string(),
						message: "datetime value must be RFC3339.".to_string(),
					})
			},
	}
}

fn parse_string(path: &str, raw: &Value) -> Result<String, FilterParseError> {
	let value = raw.as_str().ok_or_else(|| FilterParseError {
		path: path.to_string(),
		message: "string value expected.".to_string(),
	})?;

	if value.len() > MAX_STRING_BYTES {
		return Err(FilterParseError {
			path: path.to_string(),
			message: format!("string value exceeds maximum bytes ({}).", MAX_STRING_BYTES),
		});
	}

	Ok(value.to_string())
}
