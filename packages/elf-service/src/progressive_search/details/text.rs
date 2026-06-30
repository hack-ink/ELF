use crate::{PayloadLevel, structured_fields::StructuredFields};

pub(crate) fn build_summary(raw: &str, max_chars: usize) -> String {
	let normalized = normalize_whitespace(raw);

	truncate_chars(&normalized, max_chars)
}

pub(super) fn apply_payload_level_to_search_details_text(
	raw_text: &str,
	structured: Option<&StructuredFields>,
	payload_level: PayloadLevel,
	max_note_chars: usize,
) -> String {
	match payload_level {
		PayloadLevel::L0 => build_summary(raw_text, max_note_chars),
		PayloadLevel::L1 => {
			let candidate_text = structured
				.and_then(|item| item.summary.as_deref())
				.filter(|summary| !summary.trim().is_empty())
				.unwrap_or(raw_text);

			build_summary(candidate_text, max_note_chars)
		},
		PayloadLevel::L2 => raw_text.to_string(),
	}
}

fn normalize_whitespace(raw: &str) -> String {
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

fn truncate_chars(raw: &str, max_chars: usize) -> String {
	if raw.chars().count() <= max_chars {
		return raw.to_string();
	}

	const TRUNCATION_MARKER: &str = "...";

	let marker_chars = TRUNCATION_MARKER.chars().count();

	if max_chars <= marker_chars {
		return TRUNCATION_MARKER.chars().take(max_chars).collect();
	}

	let truncated_chars = max_chars - marker_chars;
	let mut out = String::with_capacity(max_chars);

	for (idx, ch) in raw.chars().enumerate() {
		if idx >= truncated_chars {
			break;
		}

		out.push(ch);
	}

	out.push_str(TRUNCATION_MARKER);

	out
}
