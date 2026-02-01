pub fn evidence_matches(messages: &[String], index: usize, quote: &str) -> bool {
    messages
        .get(index)
        .map(|msg| msg.contains(quote))
        .unwrap_or(false)
}
