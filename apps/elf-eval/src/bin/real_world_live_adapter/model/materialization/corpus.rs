use crate::{HashMap, Serialize, Uuid, model::LiveCapturePolicy};

use super::CaptureMaterializationEvidence;

#[derive(Debug)]
pub(crate) struct CorpusText {
	pub(crate) evidence_id: String,
	pub(crate) text: String,
	pub(crate) capture: LiveCapturePolicy,
}

#[derive(Debug, Default)]
pub(crate) struct IngestedCorpus {
	pub(crate) capture: CaptureMaterializationEvidence,
	pub(crate) note_ids_by_evidence: HashMap<String, Vec<Uuid>>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SourceMappingEvidence {
	pub(crate) source: String,
	pub(crate) evidence_ids: Vec<String>,
	pub(crate) mapping_status: String,
	pub(crate) content_count: usize,
}
