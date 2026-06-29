use crate::knowledge::support::{KnowledgeNoteSource, KnowledgePageKind};

pub(in crate::knowledge) fn snippet_for_query(
	content: &str,
	query: &str,
	max_chars: usize,
) -> String {
	let normalized = normalize_whitespace(content);
	let query = query.trim();

	if query.is_empty() {
		return truncate_chars(normalized.as_str(), max_chars);
	}

	let lower = normalized.to_ascii_lowercase();
	let lower_query = query.to_ascii_lowercase();
	let Some(byte_idx) = lower.find(lower_query.as_str()) else {
		return truncate_chars(normalized.as_str(), max_chars);
	};
	let before_chars = normalized[..byte_idx].chars().count();
	let start = before_chars.saturating_sub(40);
	let mut snippet: String = normalized.chars().skip(start).take(max_chars).collect();

	if start > 0 {
		snippet = format!("...{snippet}");
	}
	if normalized.chars().count() > start + snippet.chars().count() {
		snippet.push_str("...");
	}

	snippet
}

pub(in crate::knowledge) fn normalize_whitespace(raw: &str) -> String {
	let mut out = String::with_capacity(raw.len());
	let mut prev_space = false;

	for ch in raw.chars() {
		if ch.is_whitespace() {
			if !prev_space {
				out.push(' ');

				prev_space = true;
			}

			continue;
		}

		out.push(ch);

		prev_space = false;
	}

	out.trim().to_string()
}

pub(in crate::knowledge) fn truncate_chars(raw: &str, max_chars: usize) -> String {
	if raw.chars().count() <= max_chars {
		return raw.to_string();
	}

	const TRUNCATION_MARKER: &str = "...";

	let marker_chars = TRUNCATION_MARKER.chars().count();

	if max_chars <= marker_chars {
		return TRUNCATION_MARKER.chars().take(max_chars).collect();
	}

	let truncated_chars = max_chars - marker_chars;
	let mut out = raw.chars().take(truncated_chars).collect::<String>();

	out.push_str(TRUNCATION_MARKER);

	out
}

pub(in crate::knowledge) fn note_prefix(row: &KnowledgeNoteSource) -> String {
	row.key
		.as_ref()
		.map(|key| format!("[{}:{key}] ", row.note_type))
		.unwrap_or_else(|| format!("[{}] ", row.note_type))
}

pub(in crate::knowledge) fn generated_title(
	page_kind: KnowledgePageKind,
	page_key: &str,
) -> String {
	format!("{} Knowledge Page: {page_key}", title_kind(page_kind))
}

pub(in crate::knowledge) fn title_kind(page_kind: KnowledgePageKind) -> &'static str {
	match page_kind {
		KnowledgePageKind::Project => "Project",
		KnowledgePageKind::Entity => "Entity",
		KnowledgePageKind::Concept => "Concept",
		KnowledgePageKind::Issue => "Issue",
		KnowledgePageKind::Decision => "Decision",
		KnowledgePageKind::Author => "Author",
		KnowledgePageKind::Timeline => "Timeline",
	}
}
