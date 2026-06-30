use std::collections::HashSet;

pub(crate) fn match_terms_in_text(
	tokens: &[String],
	text: &str,
	key: Option<&str>,
	max_terms: usize,
) -> (Vec<String>, Vec<String>) {
	if tokens.is_empty() {
		return (Vec::new(), Vec::new());
	}

	let text = text.to_lowercase();
	let key = key.map(|value| value.to_lowercase());
	let mut matched_terms = Vec::new();
	let mut matched_fields = HashSet::new();

	for token in tokens {
		let mut matched = false;

		if text.contains(token) {
			matched_fields.insert("text");

			matched = true;
		}

		if let Some(key) = key.as_ref()
			&& key.contains(token)
		{
			matched_fields.insert("key");

			matched = true;
		}

		if matched {
			matched_terms.push(token.clone());
		}
		if matched_terms.len() >= max_terms {
			break;
		}
	}

	let mut fields: Vec<String> =
		matched_fields.into_iter().map(|field| field.to_string()).collect();

	fields.sort();

	(matched_terms, fields)
}

pub(crate) fn merge_matched_fields(
	mut base: Vec<String>,
	extra: Option<&Vec<String>>,
) -> Vec<String> {
	if let Some(extra) = extra {
		for field in extra {
			base.push(field.clone());
		}

		base.sort();
		base.dedup();
	}

	base
}
