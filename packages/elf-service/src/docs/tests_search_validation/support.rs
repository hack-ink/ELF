use qdrant_client::qdrant::{
	DatetimeRange, Filter, condition::ConditionOneOf, r#match::MatchValue,
};

pub(crate) fn first_datetime_range(filter: &Filter, key: &str) -> Option<DatetimeRange> {
	for condition in &filter.must {
		if let Some(ConditionOneOf::Field(field)) = condition.condition_one_of.as_ref() {
			if field.key != key {
				continue;
			}

			if let Some(range) = field.datetime_range.as_ref() {
				return Some(*range);
			}
		}
	}

	None
}

pub(crate) fn first_match_value(filter: &Filter, key: &str) -> Option<String> {
	for condition in &filter.must {
		if let Some(ConditionOneOf::Field(field)) = condition.condition_one_of.as_ref() {
			if field.key != key {
				continue;
			}

			if let Some(r#match) = field.r#match.as_ref() {
				let Some(match_value) = r#match.match_value.as_ref() else {
					continue;
				};

				return match match_value {
					MatchValue::Keyword(value) => Some(value.clone()),
					_ => None,
				};
			}
		}
	}

	None
}
