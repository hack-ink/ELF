pub fn evidence_matches(messages: &[String], index: usize, quote: &str) -> bool {
	if quote.trim().is_empty() {
		return false;
	}

	messages.get(index).map(|msg| msg.contains(quote)).unwrap_or(false)
}
