use std::{
	cmp::Ordering,
	collections::HashMap,
	fmt::{Display, Formatter},
};

use serde::Serialize;
use serde_json::{Map, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::search::{ChunkCandidate, NoteMeta, SEARCH_FILTER_IMPACT_SCHEMA_V1};

const SEARCH_FILTER_EXPR_SCHEMA_V1: &str = "search_filter_expr/v1";
const MAX_FILTER_DEPTH: usize = 8;
const MAX_FILTER_NODES: usize = 128;
const MAX_IN_LIST_ITEMS: usize = 128;
const MAX_STRING_BYTES: usize = 512;

#[derive(Debug, Clone)]
pub(crate) struct FilterParseError {
	path: String,
	message: String,
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
	fn as_value(&self) -> Value {
		self.json.clone()
	}

	fn evaluate(&self, note: &NoteMeta) -> (bool, Option<String>) {
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

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SearchFilterImpact {
	requested_candidate_k: u32,
	effective_candidate_k: u32,
	candidate_count_pre: usize,
	candidate_count_post: usize,
	dropped_total: usize,
	top_drop_reasons: Vec<SearchFilterDropReason>,
	filter: Value,
}
impl SearchFilterImpact {
	pub(crate) fn from_eval(
		filter: &SearchFilter,
		note_candidates: &[ChunkCandidate],
		note_meta: &HashMap<Uuid, NoteMeta>,
		requested_candidate_k: u32,
		effective_candidate_k: u32,
	) -> Self {
		let pre = note_candidates.len();
		let mut kept: Vec<ChunkCandidate> = Vec::new();
		let mut dropped_reason_counts: HashMap<String, usize> = HashMap::new();

		for candidate in note_candidates {
			let Some(note) = note_meta.get(&candidate.note_id) else {
				dropped_reason_counts
					.entry("note_meta_missing".to_string())
					.and_modify(|count| *count += 1)
					.or_insert(1);

				continue;
			};
			let (keep, reason) = filter.evaluate(note);

			if keep {
				kept.push(candidate.clone());
			} else {
				dropped_reason_counts
					.entry(reason.unwrap_or_else(|| "filter.no_match".to_string()))
					.and_modify(|count| *count += 1)
					.or_insert(1);
			}
		}

		let mut top_drop_reasons: Vec<_> = dropped_reason_counts
			.into_iter()
			.map(|(reason, count)| SearchFilterDropReason { reason, count })
			.collect();

		top_drop_reasons.sort_by(|a, b| match b.count.cmp(&a.count) {
			Ordering::Equal => a.reason.cmp(&b.reason),
			other => other,
		});
		top_drop_reasons.truncate(5);

		let post = kept.len();

		Self {
			requested_candidate_k,
			effective_candidate_k,
			candidate_count_pre: pre,
			candidate_count_post: post,
			dropped_total: pre.saturating_sub(post),
			top_drop_reasons,
			filter: filter.as_value(),
		}
	}

	pub(crate) fn to_stage_payload(&self) -> Value {
		serde_json::json!({
			"schema": SEARCH_FILTER_IMPACT_SCHEMA_V1,
			"requested_candidate_k": self.requested_candidate_k,
			"effective_candidate_k": self.effective_candidate_k,
			"candidate_count_pre": self.candidate_count_pre,
			"candidate_count_post": self.candidate_count_post,
			"dropped_total": self.dropped_total,
			"top_drop_reasons": self.top_drop_reasons,
			"filter": self.filter,
		})
	}
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SearchFilterDropReason {
	reason: String,
	count: usize,
}

#[derive(Default)]
struct FilterParseState {
	nodes: usize,
	max_depth: usize,
}

#[derive(Clone, Debug)]
enum FilterField {
	Type,
	Key,
	Scope,
	AgentId,
	Importance,
	Confidence,
	UpdatedAt,
	ExpiresAt,
	HitCount,
	LastHitAt,
}
impl FilterField {
	fn as_str(&self) -> &'static str {
		match self {
			Self::Type => "type",
			Self::Key => "key",
			Self::Scope => "scope",
			Self::AgentId => "agent_id",
			Self::Importance => "importance",
			Self::Confidence => "confidence",
			Self::UpdatedAt => "updated_at",
			Self::ExpiresAt => "expires_at",
			Self::HitCount => "hit_count",
			Self::LastHitAt => "last_hit_at",
		}
	}

	fn parse(path: &str, raw: &Value) -> Result<Self, FilterParseError> {
		let field = raw
			.as_str()
			.ok_or_else(|| FilterParseError {
				path: path.to_string(),
				message: "filter field must be a string.".to_string(),
			})?
			.to_ascii_lowercase();

		match field.as_str() {
			"type" => Ok(Self::Type),
			"key" => Ok(Self::Key),
			"scope" => Ok(Self::Scope),
			"agent_id" => Ok(Self::AgentId),
			"importance" => Ok(Self::Importance),
			"confidence" => Ok(Self::Confidence),
			"updated_at" => Ok(Self::UpdatedAt),
			"expires_at" => Ok(Self::ExpiresAt),
			"hit_count" => Ok(Self::HitCount),
			"last_hit_at" => Ok(Self::LastHitAt),
			_ => Err(FilterParseError {
				path: path.to_string(),
				message: format!(
					"field '{}' is not in allowlist: type, key, scope, agent_id, importance, confidence, updated_at, expires_at, hit_count, last_hit_at",
					field,
				),
			}),
		}
	}

	fn lookup_note_value(&self, note: &NoteMeta) -> FilterNodeValue {
		FilterExpr::lookup_note_value(self, note)
	}
}

#[derive(Clone, Debug)]
enum FilterExpr {
	And(Vec<FilterExpr>),
	Or(Vec<FilterExpr>),
	Not(Box<FilterExpr>),
	Eq { field: FilterField, value: FilterValue },
	Neq { field: FilterField, value: FilterValue },
	In { field: FilterField, values: Vec<FilterValue> },
	Contains { field: FilterField, value: String },
	Gt { field: FilterField, value: FilterValue },
	Gte { field: FilterField, value: FilterValue },
	Lt { field: FilterField, value: FilterValue },
	Lte { field: FilterField, value: FilterValue },
}
impl FilterExpr {
	fn to_value(&self) -> Value {
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

	fn evaluate(&self, note: &NoteMeta) -> (bool, Option<String>) {
		match self {
			Self::And(nodes) => Self::evaluate_and(nodes, note),
			Self::Or(nodes) => Self::evaluate_or(nodes, note),
			Self::Not(node) => Self::evaluate_not(node, note),
			Self::Eq { field, value } => Self::evaluate_eq(field, value, note),
			Self::Neq { field, value } => Self::evaluate_neq(field, value, note),
			Self::In { field, values } => Self::evaluate_in(field, values, note),
			Self::Contains { field, value } => Self::evaluate_contains(field, value, note),
			Self::Gt { field, value } => Self::evaluate_gt(field, value, note),
			Self::Gte { field, value } => Self::evaluate_gte(field, value, note),
			Self::Lt { field, value } => Self::evaluate_lt(field, value, note),
			Self::Lte { field, value } => Self::evaluate_lte(field, value, note),
		}
	}

	fn evaluate_and(nodes: &[Self], note: &NoteMeta) -> (bool, Option<String>) {
		for node in nodes {
			let (passed, reason) = node.evaluate(note);

			if !passed {
				return (false, reason);
			}
		}

		(true, None)
	}

	fn evaluate_or(nodes: &[Self], note: &NoteMeta) -> (bool, Option<String>) {
		let mut first_reason = None;

		for node in nodes {
			let (passed, reason) = node.evaluate(note);

			if passed {
				return (true, None);
			}
			if first_reason.is_none() {
				first_reason = reason;
			}
		}

		(false, first_reason.or_else(|| Some("or.no_match".to_string())))
	}

	fn evaluate_not(node: &Self, note: &NoteMeta) -> (bool, Option<String>) {
		let (passed, reason) = node.evaluate(note);

		if passed { (false, Some("not.true".to_string())) } else { (true, reason) }
	}

	fn evaluate_eq(
		field: &FilterField,
		value: &FilterValue,
		note: &NoteMeta,
	) -> (bool, Option<String>) {
		let note_value = field.lookup_note_value(note);
		let filter_value = value.to_node_value();
		let matches = note_value == filter_value;

		(matches, Some(format!("eq:{}", field.as_str())).filter(|_| !matches))
	}

	fn evaluate_neq(
		field: &FilterField,
		value: &FilterValue,
		note: &NoteMeta,
	) -> (bool, Option<String>) {
		let note_value = field.lookup_note_value(note);
		let filter_value = value.to_node_value();
		let matches = note_value != filter_value;

		(matches, Some(format!("neq:{}", field.as_str())).filter(|_| !matches))
	}

	fn evaluate_in(
		field: &FilterField,
		values: &[FilterValue],
		note: &NoteMeta,
	) -> (bool, Option<String>) {
		let note_value = field.lookup_note_value(note);
		let matches = values.iter().any(|value| note_value == FilterNodeValue::from(value));

		(matches, Some(format!("in:{}", field.as_str())).filter(|_| !matches))
	}

	fn evaluate_contains(
		field: &FilterField,
		value: &str,
		note: &NoteMeta,
	) -> (bool, Option<String>) {
		let note_value = field.lookup_note_value(note);
		let note_text = match note_value {
			FilterNodeValue::String(s) => s,
			_ => {
				return (false, Some(format!("contains:{}", field.as_str())));
			},
		};
		let matches = note_text.contains(value);

		(matches, Some(format!("contains:{}", field.as_str())).filter(|_| !matches))
	}

	fn evaluate_gt(
		field: &FilterField,
		value: &FilterValue,
		note: &NoteMeta,
	) -> (bool, Option<String>) {
		match field.lookup_note_value(note) {
			FilterNodeValue::Number(note_value) => {
				let matches = note_value > value.to_numeric();

				(matches, Some(format!("gt:{}", field.as_str())).filter(|_| !matches))
			},
			FilterNodeValue::DateTime(note_value) => {
				let matches = match value {
					FilterValue::DateTime(filter_value) => note_value > *filter_value,
					_ => false,
				};

				(matches, Some(format!("gt:{}", field.as_str())).filter(|_| !matches))
			},
			_ => (false, Some(format!("gt:{}", field.as_str()))),
		}
	}

	fn evaluate_gte(
		field: &FilterField,
		value: &FilterValue,
		note: &NoteMeta,
	) -> (bool, Option<String>) {
		match field.lookup_note_value(note) {
			FilterNodeValue::Number(note_value) => {
				let matches = note_value >= value.to_numeric();

				(matches, Some(format!("gte:{}", field.as_str())).filter(|_| !matches))
			},
			FilterNodeValue::DateTime(note_value) => {
				let matches = match value {
					FilterValue::DateTime(filter_value) => note_value >= *filter_value,
					_ => false,
				};

				(matches, Some(format!("gte:{}", field.as_str())).filter(|_| !matches))
			},
			_ => (false, Some(format!("gte:{}", field.as_str()))),
		}
	}

	fn evaluate_lt(
		field: &FilterField,
		value: &FilterValue,
		note: &NoteMeta,
	) -> (bool, Option<String>) {
		match field.lookup_note_value(note) {
			FilterNodeValue::Number(note_value) => {
				let matches = note_value < value.to_numeric();

				(matches, Some(format!("lt:{}", field.as_str())).filter(|_| !matches))
			},
			FilterNodeValue::DateTime(note_value) => {
				let matches = match value {
					FilterValue::DateTime(filter_value) => note_value < *filter_value,
					_ => false,
				};

				(matches, Some(format!("lt:{}", field.as_str())).filter(|_| !matches))
			},
			_ => (false, Some(format!("lt:{}", field.as_str()))),
		}
	}

	fn evaluate_lte(
		field: &FilterField,
		value: &FilterValue,
		note: &NoteMeta,
	) -> (bool, Option<String>) {
		match field.lookup_note_value(note) {
			FilterNodeValue::Number(note_value) => {
				let matches = note_value <= value.to_numeric();

				(matches, Some(format!("lte:{}", field.as_str())).filter(|_| !matches))
			},
			FilterNodeValue::DateTime(note_value) => {
				let matches = match value {
					FilterValue::DateTime(filter_value) => note_value <= *filter_value,
					_ => false,
				};

				(matches, Some(format!("lte:{}", field.as_str())).filter(|_| !matches))
			},
			_ => (false, Some(format!("lte:{}", field.as_str()))),
		}
	}

	fn lookup_note_value(field: &FilterField, note: &NoteMeta) -> FilterNodeValue {
		match field {
			FilterField::Type => FilterNodeValue::String(note.note_type.clone()),
			FilterField::Key => FilterNodeValue::String(note.key.clone().unwrap_or_default()),
			FilterField::Scope => FilterNodeValue::String(note.scope.clone()),
			FilterField::AgentId => FilterNodeValue::String(note.agent_id.clone()),
			FilterField::Importance => FilterNodeValue::Number(note.importance as f64),
			FilterField::Confidence => FilterNodeValue::Number(note.confidence as f64),
			FilterField::HitCount => FilterNodeValue::Number(note.hit_count as f64),
			FilterField::UpdatedAt => FilterNodeValue::DateTime(note.updated_at),
			FilterField::ExpiresAt =>
				note.expires_at.map_or(FilterNodeValue::Null, FilterNodeValue::DateTime),
			FilterField::LastHitAt =>
				note.last_hit_at.map_or(FilterNodeValue::Null, FilterNodeValue::DateTime),
		}
	}

	fn parse_args(
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

				parse_expr(node, &child_path, depth.saturating_add(1), state)
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

				parse_value(field, raw, &item_path)
			})
			.collect()
	}

	fn validate_metrics(
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

	fn parse_leaf(
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
		let value = parse_value(&field, value_raw, &path_value)?;

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

impl Default for FilterExpr {
	fn default() -> Self {
		Self::Eq { field: FilterField::Type, value: FilterValue::Null }
	}
}

#[derive(Clone, Debug)]
enum FilterValue {
	String(String),
	Number(f64),
	DateTime(OffsetDateTime),
	Null,
}
impl FilterValue {
	fn to_node_value(&self) -> FilterNodeValue {
		match self {
			Self::String(value) => FilterNodeValue::String(value.clone()),
			Self::Number(value) => FilterNodeValue::Number(*value),
			Self::DateTime(value) => FilterNodeValue::DateTime(*value),
			Self::Null => FilterNodeValue::Null,
		}
	}

	fn to_value(&self) -> Value {
		match self {
			Self::String(value) => Value::String(value.clone()),
			Self::Number(value) => serde_json::json!(value),
			Self::DateTime(value) => Value::String(value.format(&Rfc3339).unwrap_or_default()),
			Self::Null => Value::Null,
		}
	}

	fn to_numeric(&self) -> f64 {
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
enum FilterNodeValue {
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

fn parse_expr(
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

fn parse_value(
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

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use serde_json::{Map, Value};
	use time::OffsetDateTime;

	use uuid::Uuid;

	use crate::search::filter::{
		ChunkCandidate, MAX_FILTER_NODES, MAX_IN_LIST_ITEMS, MAX_STRING_BYTES, NoteMeta,
		SEARCH_FILTER_EXPR_SCHEMA_V1, SearchFilter,
	};

	fn note_meta() -> NoteMeta {
		NoteMeta {
			note_id: Uuid::new_v4(),
			note_type: "fact".to_string(),
			key: Some("foo".to_string()),
			scope: "project_shared".to_string(),
			agent_id: "agent-a".to_string(),
			importance: 0.9,
			confidence: 0.8,
			updated_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("timestamp"),
			expires_at: None,
			source_ref: Value::Object(Map::new()),
			embedding_version: "provider:model:1".to_string(),
			hit_count: 4,
			last_hit_at: None,
		}
	}

	#[test]
	fn parse_requires_known_schema() {
		let raw = serde_json::json!({ "schema": "bad", "expr": { "op": "eq", "field": "scope", "value": "project_shared" } });

		assert!(SearchFilter::parse(&raw).is_err());
	}

	#[test]
	fn parse_and_validate_depth_limit() {
		let mut expr =
			serde_json::json!({ "op": "eq", "field": "scope", "value": "project_shared" });

		for _ in 0..9 {
			expr = serde_json::json!({ "op": "not", "expr": expr });
		}

		let raw = serde_json::json!({ "schema": SEARCH_FILTER_EXPR_SCHEMA_V1, "expr": expr });

		assert!(SearchFilter::parse(&raw).is_err());
	}

	#[test]
	fn parse_and_validate_node_limit() {
		let leaf = serde_json::json!({ "op": "eq", "field": "scope", "value": "project_shared" });
		let mut args = Vec::with_capacity(MAX_FILTER_NODES);

		for _ in 0..(MAX_FILTER_NODES - 1) {
			args.push(leaf.clone());
		}

		let expr = serde_json::json!({ "op": "and", "args": args });
		let raw = serde_json::json!({ "schema": SEARCH_FILTER_EXPR_SCHEMA_V1, "expr": expr });

		assert!(SearchFilter::parse(&raw).is_ok());

		let expr = serde_json::json!({ "op": "and", "args": [expr, leaf] });
		let raw = serde_json::json!({ "schema": SEARCH_FILTER_EXPR_SCHEMA_V1, "expr": expr });

		assert!(
			SearchFilter::parse(&raw).is_err(),
			"expected parse failure when node count is greater than limit"
		);
	}

	#[test]
	fn parse_in_list_limit() {
		let values = (0_i32..=MAX_IN_LIST_ITEMS as i32)
			.map(|value| serde_json::json!(value))
			.collect::<Vec<_>>();
		let raw = serde_json::json!({
			"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
			"expr": {
				"op": "in",
				"field": "importance",
				"value": values,
			},
		});

		assert!(SearchFilter::parse(&raw).is_err());
	}

	#[test]
	fn parse_rejects_unknown_field_with_json_path() {
		let raw = serde_json::json!({
			"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
			"expr": { "op": "eq", "field": "bad_field", "value": "project_shared" },
		});
		let err = SearchFilter::parse(&raw).expect_err("expected unknown field error");

		assert!(err.to_string().contains("$.filter.expr"));
		assert!(err.to_string().contains("not in allowlist"));
	}

	#[test]
	fn parse_rejects_invalid_value_type_with_json_path() {
		let raw = serde_json::json!({
			"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
			"expr": { "op": "eq", "field": "importance", "value": "wrong" },
		});
		let err = SearchFilter::parse(&raw).expect_err("expected invalid value type error");

		assert!(err.to_string().contains("$.filter.expr.value"));
	}

	#[test]
	fn parse_rejects_oversize_string_with_json_path() {
		let value = "x".repeat(MAX_STRING_BYTES + 1);
		let raw = serde_json::json!({
			"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
			"expr": { "op": "eq", "field": "scope", "value": value },
		});
		let err = SearchFilter::parse(&raw).expect_err("expected string too long error");

		assert!(err.to_string().contains("$.filter.expr.value"));
	}

	#[test]
	fn eval_filters_note_metadata() {
		let raw = serde_json::json!({
			"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
			"expr": {
				"op": "and",
				"args": [
					{ "op": "eq", "field": "scope", "value": "project_shared" },
					{ "op": "gte", "field": "importance", "value": 0.5 },
				],
			},
		});
		let filter = SearchFilter::parse(&raw).expect("valid filter");
		let meta = note_meta();
		let note_meta = HashMap::from([(meta.note_id, meta)]);
		let candidate = ChunkCandidate {
			note_id: Uuid::new_v4(),
			chunk_id: Uuid::new_v4(),
			chunk_index: 0,
			retrieval_rank: 1,
			scope: Some("project_shared".to_string()),
			updated_at: None,
			embedding_version: None,
		};
		let (result, impact) = filter.eval(vec![candidate], &note_meta, 10, 12);

		assert_eq!(result.len(), 0);
		assert_eq!(impact.requested_candidate_k, 10);
		assert_eq!(impact.effective_candidate_k, 12);
	}

	#[test]
	fn filter_impact_lists_top_drop_reasons_deterministically() {
		let filter = SearchFilter::parse(&serde_json::json!({
			"schema": SEARCH_FILTER_EXPR_SCHEMA_V1,
			"expr": { "op": "eq", "field": "scope", "value": "project_shared" },
		}))
		.expect("valid filter");
		let first = Uuid::new_v4();
		let second = Uuid::new_v4();
		let third = Uuid::new_v4();
		let mut note_meta = HashMap::new();

		note_meta.insert(
			first,
			NoteMeta {
				note_id: first,
				note_type: "fact".to_string(),
				key: Some("k1".to_string()),
				scope: "agent_private".to_string(),
				agent_id: "a".to_string(),
				importance: 0.9,
				confidence: 0.9,
				updated_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("timestamp"),
				expires_at: None,
				source_ref: Value::Object(Map::new()),
				embedding_version: "provider:model:1".to_string(),
				hit_count: 0,
				last_hit_at: None,
			},
		);
		note_meta.insert(
			second,
			NoteMeta {
				note_id: second,
				note_type: "fact".to_string(),
				key: Some("k2".to_string()),
				scope: "agent_private".to_string(),
				agent_id: "a".to_string(),
				importance: 0.9,
				confidence: 0.9,
				updated_at: OffsetDateTime::from_unix_timestamp(1_700_000_001).expect("timestamp"),
				expires_at: None,
				source_ref: Value::Object(Map::new()),
				embedding_version: "provider:model:1".to_string(),
				hit_count: 0,
				last_hit_at: None,
			},
		);

		let candidates = vec![
			ChunkCandidate {
				note_id: first,
				chunk_id: Uuid::new_v4(),
				chunk_index: 0,
				retrieval_rank: 1,
				scope: None,
				updated_at: None,
				embedding_version: None,
			},
			ChunkCandidate {
				note_id: second,
				chunk_id: Uuid::new_v4(),
				chunk_index: 1,
				retrieval_rank: 2,
				scope: None,
				updated_at: None,
				embedding_version: None,
			},
			ChunkCandidate {
				note_id: third,
				chunk_id: Uuid::new_v4(),
				chunk_index: 2,
				retrieval_rank: 3,
				scope: None,
				updated_at: None,
				embedding_version: None,
			},
		];
		let (_, impact) = filter.eval(candidates, &note_meta, 10, 20);

		assert_eq!(impact.candidate_count_pre, 3);
		assert_eq!(impact.candidate_count_post, 0);
		assert_eq!(impact.dropped_total, 3);
		assert_eq!(impact.top_drop_reasons.len(), 2);
		assert_eq!(impact.top_drop_reasons[0].reason, "eq:scope");
		assert_eq!(impact.top_drop_reasons[0].count, 2);
		assert_eq!(impact.top_drop_reasons[1].reason, "note_meta_missing");
		assert_eq!(impact.top_drop_reasons[1].count, 1);
	}
}
