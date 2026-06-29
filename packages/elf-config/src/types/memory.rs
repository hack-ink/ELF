use serde::Deserialize;

/// Write-path limits and policy controls for note ingestion.
#[derive(Debug, Deserialize)]
pub struct Memory {
	/// Maximum number of notes accepted per `add_event` request.
	pub max_notes_per_add_event: u32,
	/// Maximum character length for an individual note.
	pub max_note_chars: u32,
	/// Similarity threshold for duplicate detection.
	pub dup_sim_threshold: f32,
	/// Similarity threshold for update-vs-insert decisions.
	pub update_sim_threshold: f32,
	/// Candidate pool size used before final top-k selection.
	pub candidate_k: u32,
	/// Final top-k size for note retrieval.
	pub top_k: u32,
	/// Optional downgrade rules applied after base memory decisions.
	pub policy: MemoryPolicy,
}

/// Collection of memory-policy downgrade rules.
#[derive(Debug, Deserialize)]
pub struct MemoryPolicy {
	/// Ordered policy rules evaluated against note type, scope, and scores.
	pub rules: Vec<MemoryPolicyRule>,
}

/// A single memory-policy rule matched by note metadata and confidence/importance thresholds.
#[derive(Debug, Default, Deserialize)]
pub struct MemoryPolicyRule {
	/// Optional note type selector.
	pub note_type: Option<String>,
	/// Optional scope selector.
	pub scope: Option<String>,
	/// Optional minimum confidence required for the rule to match.
	pub min_confidence: Option<f32>,
	/// Optional minimum importance required for the rule to match.
	pub min_importance: Option<f32>,
}
