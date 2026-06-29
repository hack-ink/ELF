use super::*;

pub(in crate::knowledge) fn hash_text(text: &str) -> String {
	blake3::hash(text.as_bytes()).to_hex().to_string()
}

pub(in crate::knowledge) fn hash_json_lossy(value: &Value) -> String {
	serde_json::to_vec(value)
		.map(|raw| blake3::hash(&raw).to_hex().to_string())
		.unwrap_or_else(|_| hash_text(value.to_string().as_str()))
}

pub(in crate::knowledge) fn hash_json(value: &Value) -> Result<String> {
	let raw = serde_json::to_vec(value).map_err(|err| Error::InvalidRequest {
		message: format!("failed to serialize knowledge page payload: {err}"),
	})?;

	Ok(blake3::hash(&raw).to_hex().to_string())
}
