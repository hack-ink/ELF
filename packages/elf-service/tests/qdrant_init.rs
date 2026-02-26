use std::{fs, path::PathBuf};

#[test]
fn qdrant_init_script_creates_docs_payload_indexes() {
	let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
	let script_path = script_path.join("..").join("qdrant").join("init.sh");
	let script = fs::read_to_string(&script_path)
		.unwrap_or_else(|err| panic!("Failed to read {}: {err}", script_path.display()));
	let script = script.chars().filter(|ch| !ch.is_whitespace()).collect::<String>();

	for (field, field_schema) in [
		("scope", "keyword"),
		("status", "keyword"),
		("doc_type", "keyword"),
		("agent_id", "keyword"),
		("updated_at", "datetime"),
		("doc_ts", "datetime"),
		("thread_id", "keyword"),
		("domain", "keyword"),
		("repo", "keyword"),
	] {
		let needle = format!("\"field_name\":\"{field}\",\"field_schema\":\"{field_schema}\"");

		assert!(
			script.contains(&needle),
			"Missing payload index for docs field {field} with schema {field_schema} in qdrant/init.sh"
		);
	}

	assert!(
		script.contains("\"${collection}\"==\"${ELF_QDRANT_DOCS_COLLECTION"),
		"Docs payload indexing is not gated to ELF_QDRANT_DOCS_COLLECTION."
	);
}
