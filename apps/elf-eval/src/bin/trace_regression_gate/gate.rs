use std::{fs, path::PathBuf};

use color_eyre::Result;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct GateThresholds {
	pub(super) max_positional_churn_at_k: Option<f64>,
	pub(super) max_set_churn_at_k: Option<f64>,
	pub(super) min_retrieval_top_rank_retention: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct GateTrace {
	pub(super) trace_id: Uuid,
	pub(super) top_k: Option<u32>,
	pub(super) retrieval_retention_rank: Option<u32>,
	#[serde(flatten)]
	pub(super) thresholds: GateThresholds,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct GateFile {
	#[serde(default)]
	pub(super) defaults: GateThresholds,
	pub(super) top_k: Option<u32>,
	pub(super) retrieval_retention_rank: Option<u32>,
	pub(super) traces: Vec<GateTrace>,
}

pub(super) fn load_gate_file(path: &PathBuf) -> Result<GateFile> {
	let raw = fs::read_to_string(path)?;
	let out: GateFile = serde_json::from_str(&raw)?;

	Ok(out)
}

pub(super) fn merge_thresholds(
	defaults: GateThresholds,
	overrides: GateThresholds,
) -> GateThresholds {
	GateThresholds {
		max_positional_churn_at_k: overrides
			.max_positional_churn_at_k
			.or(defaults.max_positional_churn_at_k),
		max_set_churn_at_k: overrides.max_set_churn_at_k.or(defaults.max_set_churn_at_k),
		min_retrieval_top_rank_retention: overrides
			.min_retrieval_top_rank_retention
			.or(defaults.min_retrieval_top_rank_retention),
	}
}
