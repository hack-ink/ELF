use unicode_normalization::UnicodeNormalization;
use unicode_script::{Script, UnicodeScript};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnglishGateKind {
	/// Natural-language text that is expected to be English prose.
	NaturalLanguage,
	/// Structured identifiers (keys, URLs, ids). No language identification is applied.
	Identifier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnglishGateRejectReason {
	DisallowedControlChar,
	DisallowedZeroWidthChar,
	DisallowedScript,
	LanguageIdNonEnglish,
}

pub fn english_gate(input: &str, kind: EnglishGateKind) -> Result<(), EnglishGateRejectReason> {
	let normalized: String = input.nfkc().collect();

	if contains_disallowed_controls(normalized.as_str()) {
		return Err(EnglishGateRejectReason::DisallowedControlChar);
	}
	if contains_disallowed_zero_width(normalized.as_str()) {
		return Err(EnglishGateRejectReason::DisallowedZeroWidthChar);
	}
	if contains_disallowed_scripts(normalized.as_str()) {
		return Err(EnglishGateRejectReason::DisallowedScript);
	}
	if kind == EnglishGateKind::NaturalLanguage
		&& should_apply_lid(normalized.as_str())
		&& is_confidently_non_english(normalized.as_str())
	{
		return Err(EnglishGateRejectReason::LanguageIdNonEnglish);
	}

	Ok(())
}

pub fn is_english_natural_language(input: &str) -> bool {
	english_gate(input, EnglishGateKind::NaturalLanguage).is_ok()
}

pub fn is_english_identifier(input: &str) -> bool {
	english_gate(input, EnglishGateKind::Identifier).is_ok()
}

fn contains_disallowed_controls(input: &str) -> bool {
	for ch in input.chars() {
		if !ch.is_control() {
			continue;
		}

		// Allow common whitespace controls used in code/docs.
		if matches!(ch, '\n' | '\r' | '\t') {
			continue;
		}

		return true;
	}

	false
}

fn contains_disallowed_zero_width(input: &str) -> bool {
	for ch in input.chars() {
		if matches!(
			ch,
			'\u{00AD}' // soft hyphen
				| '\u{034F}' // combining grapheme joiner
				| '\u{061C}' // arabic letter mark
				| '\u{180E}' // mongolian vowel separator (deprecated)
				| '\u{200B}' // zero width space
				| '\u{200C}' // zero width non-joiner
				| '\u{200D}' // zero width joiner
				| '\u{2060}' // word joiner
				| '\u{FEFF}' // zero width no-break space
		) {
			return true;
		}
	}

	false
}

fn contains_disallowed_scripts(input: &str) -> bool {
	for ch in input.chars() {
		if ch.is_ascii() {
			continue;
		}
		if ch.is_whitespace() {
			continue;
		}

		// Allow only Latin + neutral scripts for punctuation/symbols/emoji.
		match ch.script() {
			Script::Latin | Script::Common | Script::Inherited => {},
			_ => return true,
		}
	}

	false
}

fn should_apply_lid(input: &str) -> bool {
	let mut letters = 0usize;
	let mut non_space = 0usize;
	let mut whitespace = 0usize;

	for ch in input.chars() {
		if ch.is_whitespace() {
			whitespace += 1;
			continue;
		}
		non_space += 1;
		if ch.is_alphabetic() {
			letters += 1;
		}
	}

	// Skip short strings (too noisy for LID) and single-token identifiers.
	if letters < 32 || non_space < 64 || whitespace == 0 {
		return false;
	}

	let density = letters as f32 / non_space as f32;
	density >= 0.60
}

fn is_confidently_non_english(input: &str) -> bool {
	let Some(info) = whatlang::detect(input) else {
		return false;
	};

	// Be conservative: only reject when the detector is confident.
	if !info.is_reliable() {
		return false;
	}
	if info.confidence() < 0.85 {
		return false;
	}

	info.lang() != whatlang::Lang::Eng
}

#[cfg(test)]
mod tests {
	use super::{
		EnglishGateKind, english_gate, is_english_identifier, is_english_natural_language,
	};

	#[test]
	fn accepts_basic_english() {
		assert!(is_english_natural_language("Preference: Use English."));
	}

	#[test]
	fn rejects_cyrillic_script() {
		assert!(!is_english_natural_language("Привет мир"));
	}

	#[test]
	fn rejects_zero_width_chars() {
		assert!(!is_english_natural_language("hello\u{200B}world"));
	}

	#[test]
	fn rejects_disallowed_control_chars() {
		assert!(!is_english_natural_language("hello\u{0007}world"));
	}

	#[test]
	fn nfkc_normalization_allows_fullwidth_latin() {
		assert!(is_english_natural_language("Ｆｕｌｌｗｉｄｔｈ latin letters should normalize."));
	}

	#[test]
	fn identifier_gate_skips_lid_but_still_rejects_disallowed_script() {
		assert!(is_english_identifier("preferred_language"));
		assert!(!is_english_identifier("ключ")); // Cyrillic
	}

	#[test]
	fn lid_is_applied_only_for_long_letter_dense_text() {
		let short_french = "Bonjour.";
		assert!(english_gate(short_french, EnglishGateKind::NaturalLanguage).is_ok());

		let long_french = "Bonjour, je veux m'assurer que ce texte est suffisamment long et riche en lettres pour declencher la detection de langue. Merci beaucoup.";
		assert!(english_gate(long_french, EnglishGateKind::NaturalLanguage).is_err());
	}

	#[test]
	fn code_like_text_is_not_rejected_by_lid_thresholds() {
		let codeish = "Error: expected `foo::bar()`; got `foo::baz()` at line 12.";
		assert!(is_english_natural_language(codeish));
	}
}
