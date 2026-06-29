use super::super::*;

pub(in crate::docs) fn truncate_bytes(text: &str, max: usize) -> String {
	if text.len() <= max {
		return text.to_string();
	}

	let mut cut = max;

	while cut > 0 && !text.is_char_boundary(cut) {
		cut -= 1;
	}

	text.get(0..cut).unwrap_or("").to_string()
}

pub(in crate::docs) fn locate_quote(
	text: &str,
	quote: &TextQuoteSelector,
) -> Option<(usize, usize)> {
	let prefix = quote.prefix.as_deref().unwrap_or("");
	let suffix = quote.suffix.as_deref().unwrap_or("");

	for (start, _) in text.match_indices(quote.exact.as_str()) {
		let end = start + quote.exact.len();

		if !text[..start].ends_with(prefix) {
			continue;
		}
		if !text[end..].starts_with(suffix) {
			continue;
		}

		return Some((start, end));
	}

	None
}

pub(in crate::docs) fn bounded_window(
	match_start: usize,
	match_end: usize,
	text: &str,
	max_bytes: usize,
) -> (usize, usize) {
	let len = text.len();
	let match_center = match_start.saturating_add(match_end.saturating_sub(match_start) / 2);
	let half = max_bytes / 2;
	let mut start = match_center.saturating_sub(half);
	let mut end = (start + max_bytes).min(len);

	if end - start < max_bytes && start > 0 {
		start = start.saturating_sub(max_bytes - (end - start));
	}

	while start < len && !text.is_char_boundary(start) {
		start += 1;
	}
	while end > start && !text.is_char_boundary(end) {
		end -= 1;
	}

	(start, end)
}

pub(in crate::docs) fn docs_search_sparse_enabled(mode: DocsSparseMode, query: &str) -> bool {
	match mode {
		DocsSparseMode::Auto => should_enable_sparse_auto(query),
		DocsSparseMode::On => true,
		DocsSparseMode::Off => false,
	}
}

pub(in crate::docs) fn should_enable_sparse_auto(query: &str) -> bool {
	let trimmed = query.trim();

	if trimmed.is_empty() {
		return false;
	}
	if trimmed.contains("://")
		|| trimmed.contains('/')
		|| trimmed.contains('\\')
		|| trimmed.contains('?')
	{
		return true;
	}

	let has_mixed_alpha_num = trimmed.split_whitespace().any(|token| {
		token.chars().any(|ch| ch.is_ascii_alphabetic())
			&& token.chars().any(|ch| ch.is_ascii_digit())
	});
	let special_count = trimmed
		.chars()
		.filter(|ch| !(ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || *ch == '_'))
		.count();
	let compact_hex_like = {
		let compact = trimmed.chars().filter(|ch| !ch.is_ascii_whitespace()).collect::<String>();

		compact.len() >= 12 && compact.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-')
	};

	special_count >= 2 || compact_hex_like || (has_mixed_alpha_num && trimmed.len() > 12)
}
