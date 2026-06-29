use serde::{Deserialize, Serialize};

/// Quote-based selector for excerpt extraction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextQuoteSelector {
	/// Exact quote text to resolve.
	pub exact: String,
	/// Optional leading context used to disambiguate repeated quotes.
	pub prefix: Option<String>,
	/// Optional trailing context used to disambiguate repeated quotes.
	pub suffix: Option<String>,
}

/// Byte-position selector for excerpt extraction.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextPositionSelector {
	/// Inclusive start byte offset.
	pub start: usize,
	/// Exclusive end byte offset.
	pub end: usize,
}
