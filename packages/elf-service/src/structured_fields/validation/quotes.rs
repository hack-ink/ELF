use serde::Deserialize;
use serde_json::Value;

use crate::{Error, Result};
use elf_domain::evidence;

#[derive(Clone, Debug, Deserialize)]
struct SourceRefEvidenceQuote {
	quote: String,
}

pub(super) fn event_evidence_quotes(
	messages: &[String],
	evidence: &[(usize, String)],
) -> Result<()> {
	for (idx, (message_index, quote)) in evidence.iter().enumerate() {
		if quote.trim().is_empty() {
			return Err(Error::InvalidRequest {
				message: format!("evidence[{idx}].quote must not be empty."),
			});
		}
		if !evidence::evidence_matches(messages, *message_index, quote) {
			return Err(Error::InvalidRequest {
				message: format!("evidence[{idx}] does not match its source message."),
			});
		}
	}

	Ok(())
}

pub(super) fn extract_source_ref_quotes(source_ref: &Value) -> Vec<String> {
	let Some(evidence) = source_ref.get("evidence") else { return Vec::new() };
	let Ok(quotes) = serde_json::from_value::<Vec<SourceRefEvidenceQuote>>(evidence.clone()) else {
		return Vec::new();
	};

	quotes.into_iter().map(|q| q.quote).collect()
}

pub(super) fn fact_is_evidence_bound(
	fact: &str,
	note_text: &str,
	evidence_quotes: &[String],
) -> bool {
	let trimmed = fact.trim();

	if trimmed.is_empty() {
		return false;
	}
	if note_text.contains(trimmed) {
		return true;
	}

	for quote in evidence_quotes {
		if quote.contains(trimmed) {
			return true;
		}
	}

	false
}
