use crate::writegate::Regex;

/// Returns whether the input appears to contain secret material.
pub fn contains_secrets(text: &str) -> bool {
	let patterns = [
		r"(?i)-----BEGIN (RSA|OPENSSH|EC|DSA) PRIVATE KEY-----",
		r"(?i)ssh-rsa",
		r"(?i)sk-[a-z0-9]{20,}",
		r"(?i)api[_-]?key\s*[:=]\s*\S+",
		r"(?i)password\s*[:=]\s*\S+",
		r"(?i)secret\s*[:=]\s*\S+",
		r"(?i)token\s*[:=]\s*\S+",
		r"(?i)seed phrase",
	];

	for pattern in patterns {
		if Regex::new(pattern).map(|re| re.is_match(text)).unwrap_or(false) {
			return true;
		}
	}

	false
}
