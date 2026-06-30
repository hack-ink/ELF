use crate::ELF_NOTE_CHUNK_CHARS;

pub(crate) fn note_text_chunks(text: &str) -> Vec<String> {
	let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

	if normalized.chars().count() <= ELF_NOTE_CHUNK_CHARS {
		return vec![normalized];
	}

	let mut chunks = Vec::new();
	let mut current = String::new();

	for word in normalized.split_whitespace() {
		if word.chars().count() > ELF_NOTE_CHUNK_CHARS {
			if !current.is_empty() {
				chunks.push(current);

				current = String::new();
			}

			chunks.extend(split_long_token(word));

			continue;
		}

		let separator = usize::from(!current.is_empty());

		if current.chars().count() + separator + word.chars().count() > ELF_NOTE_CHUNK_CHARS
			&& !current.is_empty()
		{
			chunks.push(current);

			current = String::new();
		}
		if !current.is_empty() {
			current.push(' ');
		}

		current.push_str(word);
	}

	if !current.is_empty() {
		chunks.push(current);
	}

	chunks
}

fn split_long_token(token: &str) -> Vec<String> {
	let mut chunks = Vec::new();
	let mut current = String::new();

	for ch in token.chars() {
		if current.chars().count() >= ELF_NOTE_CHUNK_CHARS {
			chunks.push(current);

			current = String::new();
		}

		current.push(ch);
	}

	if !current.is_empty() {
		chunks.push(current);
	}

	chunks
}
