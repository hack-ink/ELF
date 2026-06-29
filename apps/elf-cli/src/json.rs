use std::io::{self, Write as _};

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::args::{PayloadLevel, SearchMode};

pub(crate) fn search_body(
	query: String,
	mode: SearchMode,
	top_k: Option<u32>,
	candidate_k: Option<u32>,
	payload_level: PayloadLevel,
	filter_json: Option<&str>,
) -> Result<Value> {
	let mut body = serde_json::json!({
		"mode": mode.as_str(),
		"query": query,
		"top_k": top_k,
		"candidate_k": candidate_k,
		"payload_level": payload_level.as_str(),
	});

	if let Some(filter_json) = filter_json {
		body["filter"] = parse_json_object(filter_json, "--filter-json")?;
	}

	Ok(body)
}

pub(crate) fn source_ref(
	source_id: &Option<String>,
	source_ref_json: Option<&str>,
) -> Result<Value> {
	if let Some(source_ref_json) = source_ref_json {
		return parse_json_object(source_ref_json, "--source-ref-json");
	}

	Ok(source_id.as_ref().map_or_else(
		|| serde_json::json!({}),
		|source_id| serde_json::json!({"schema": "elf_cli/v1", "ref": {"source_id": source_id}}),
	))
}

fn parse_json_object(raw: &str, flag: &str) -> Result<Value> {
	let value: Value =
		serde_json::from_str(raw).map_err(|err| eyre::eyre!("{flag} must be valid JSON: {err}"))?;

	if !value.is_object() {
		return Err(eyre::eyre!("{flag} must be a JSON object."));
	}

	Ok(value)
}

pub(crate) fn write_json(value: &Value, pretty: bool) -> Result<()> {
	if pretty {
		serde_json::to_writer_pretty(io::stdout(), value)?;
	} else {
		serde_json::to_writer(io::stdout(), value)?;
	}

	writeln!(io::stdout())?;

	Ok(())
}
