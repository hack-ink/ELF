#![allow(unused_crate_dependencies)]

//! Integration tests for the offline agentmemory fixture adapter.

use std::{path::Path, process::Command};

use color_eyre::{Result, eyre};
use serde_json::Value;

fn run_adapter() -> Result<Value> {
	let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("fixtures")
		.join("agentmemory")
		.join("sample_session.json");
	let output = Command::new(env!("CARGO_BIN_EXE_agentmemory_fixture_adapter"))
		.arg("--fixture")
		.arg(fixture)
		.output()?;

	assert!(
		output.status.success(),
		"agentmemory fixture adapter failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	Ok(serde_json::from_slice(&output.stdout)?)
}

fn array_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a Vec<Value>> {
	value
		.pointer(pointer)
		.and_then(Value::as_array)
		.ok_or_else(|| eyre::eyre!("missing array at {pointer}"))
}

fn find_by_field<'a>(items: &'a [Value], field: &str, expected: &str) -> Result<&'a Value> {
	items
		.iter()
		.find(|item| item.pointer(field).and_then(Value::as_str) == Some(expected))
		.ok_or_else(|| eyre::eyre!("missing item with {field} = {expected}"))
}

#[test]
fn fixture_maps_memories_observations_and_baselines() -> Result<()> {
	let output = run_adapter()?;

	assert_eq!(
		output.pointer("/schema").and_then(Value::as_str),
		Some("elf.agentmemory_adapter/v1")
	);
	assert_eq!(output.pointer("/summary/session_count").and_then(Value::as_u64), Some(1));
	assert_eq!(output.pointer("/summary/note_candidate_count").and_then(Value::as_u64), Some(2));
	assert_eq!(output.pointer("/summary/doc_candidate_count").and_then(Value::as_u64), Some(2));
	assert_eq!(output.pointer("/summary/baseline_query_count").and_then(Value::as_u64), Some(1));
	assert_eq!(output.pointer("/summary/ignored_count").and_then(Value::as_u64), Some(1));

	let notes = array_at(&output, "/note_candidates")?;
	let note = find_by_field(notes, "/source_memory_id", "mem-architecture-sot")?;

	assert_eq!(note.pointer("/notes_ingest_item/type").and_then(Value::as_str), Some("fact"));
	assert_eq!(
		note.pointer("/notes_ingest_item/key").and_then(Value::as_str),
		Some("architecture_sot"),
	);
	assert_eq!(
		note.pointer("/notes_ingest_item/source_ref/resolver").and_then(Value::as_str),
		Some("agentmemory_fixture/v1"),
	);

	let docs = array_at(&output, "/doc_candidates")?;
	let doc = find_by_field(docs, "/source_observation_id", "obs-architecture")?;

	assert_eq!(doc.pointer("/docs_put/doc_type").and_then(Value::as_str), Some("chat"));
	assert_eq!(
		doc.pointer("/docs_put/source_ref/schema").and_then(Value::as_str),
		Some("doc_source_ref/v1"),
	);
	assert_eq!(
		doc.pointer("/docs_put/source_ref/thread_id").and_then(Value::as_str),
		Some("am-session-2026-06-08"),
	);

	let baselines = array_at(&output, "/baseline_queries")?;
	let baseline = find_by_field(baselines, "/query_id", "q-architecture-sot")?;
	let expected_keys = array_at(baseline, "/expected_keys")?;

	assert_eq!(expected_keys.len(), 1);
	assert_eq!(expected_keys.first().and_then(Value::as_str), Some("architecture_sot"));

	Ok(())
}

#[test]
fn fixture_reports_unsupported_memory_kind_without_rewriting() -> Result<()> {
	let output = run_adapter()?;
	let ignored_items = array_at(&output, "/ignored_items")?;
	let ignored = find_by_field(ignored_items, "/source_id", "mem-raw-summary")?;

	assert_eq!(ignored.pointer("/reason").and_then(Value::as_str), Some("unsupported_memory_kind"));

	Ok(())
}
