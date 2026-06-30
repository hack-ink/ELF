use std::collections::HashMap;

use serde_json::Value;
use uuid::Uuid;

use crate::search::{
	ChunkCandidate, NoteMeta,
	filter::{
		expr::FilterExpr,
		impact::SearchFilterImpact,
		parser::{self, FilterParseError, FilterParseState, SEARCH_FILTER_EXPR_SCHEMA_V1},
	},
};

#[derive(Clone, Debug)]
pub(crate) struct SearchFilter {
	expr: FilterExpr,
	json: Value,
}
impl SearchFilter {
	pub(in crate::search::filter) fn as_value(&self) -> Value {
		self.json.clone()
	}

	pub(in crate::search::filter) fn evaluate(&self, note: &NoteMeta) -> (bool, Option<String>) {
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
		let parsed = parser::parse_expr(expr, "$.filter.expr", 1, &mut state)?;

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
