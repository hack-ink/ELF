use crate::search::{
	NoteMeta,
	filter::{
		expr::{FilterExpr, FilterField},
		value::{FilterNodeValue, FilterValue},
	},
};

impl FilterExpr {
	pub(in crate::search::filter) fn evaluate(&self, note: &NoteMeta) -> (bool, Option<String>) {
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
}
