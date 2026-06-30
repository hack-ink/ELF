use std::collections::HashSet;

pub(crate) fn tokenize_query(query: &str, max_terms: usize) -> Vec<String> {
	let mut normalized = String::with_capacity(query.len());

	for ch in query.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	let mut out = Vec::new();
	let mut seen = HashSet::new();

	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}
		if seen.insert(token) {
			out.push(token.to_string());
		}
		if out.len() >= max_terms {
			break;
		}
	}

	out
}

pub(crate) fn lexical_overlap_ratio(
	query_tokens: &[String],
	text: &str,
	max_text_terms: usize,
) -> f32 {
	if query_tokens.is_empty() {
		return 0.0;
	}

	let text_terms = tokenize_text_terms(text, max_text_terms);

	if text_terms.is_empty() {
		return 0.0;
	}

	let mut matched = 0_usize;

	for token in query_tokens {
		if text_terms.contains(token.as_str()) {
			matched += 1;
		}
	}

	matched as f32 / query_tokens.len() as f32
}

fn tokenize_text_terms(text: &str, max_terms: usize) -> HashSet<String> {
	if max_terms == 0 {
		return HashSet::new();
	}

	let mut normalized = String::with_capacity(text.len());

	for ch in text.chars() {
		if ch.is_ascii_alphanumeric() {
			normalized.push(ch.to_ascii_lowercase());
		} else {
			normalized.push(' ');
		}
	}

	let mut out = HashSet::new();

	for token in normalized.split_whitespace() {
		if token.len() < 2 {
			continue;
		}

		out.insert(token.to_string());

		if out.len() >= max_terms {
			break;
		}
	}

	out
}
