use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{GraphQueryEntityRef, GraphQueryPredicateRef};

/// Request payload for the cross-layer recall/debug panel.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecallDebugPanelRequest {
	/// Tenant that owns the readback.
	pub tenant_id: String,
	/// Project that owns the readback.
	pub project_id: String,
	/// Agent requesting the readback.
	pub agent_id: String,
	/// Read profile used for memory, document, and graph visibility.
	pub read_profile: String,
	/// Optional search trace anchor for memory selected/dropped rows.
	pub trace_id: Option<Uuid>,
	/// Shared query used when docs_query or knowledge_query are omitted.
	pub query: Option<String>,
	/// Optional Source Library query.
	pub docs_query: Option<String>,
	/// Optional Knowledge Workspace page query.
	pub knowledge_query: Option<String>,
	/// Optional graph subject selector.
	pub graph_subject: Option<GraphQueryEntityRef>,
	/// Optional graph predicate selector.
	pub graph_predicate: Option<GraphQueryPredicateRef>,
	/// Whether to include Dreaming review queue proposals. Omitted means not requested.
	pub include_dreaming: Option<bool>,
	/// Maximum rows per layer.
	pub limit: Option<u32>,
	#[serde(skip)]
	/// Whether project-scoped trace anchors are allowed for an admin mirror request.
	pub allow_project_trace_debug: bool,
}

/// Stable request echo for panel responses.
#[derive(Clone, Debug, Serialize)]
pub struct RecallDebugPanelRequestEcho {
	/// Search trace anchor used for memory rows.
	pub trace_id: Option<Uuid>,
	/// Effective Source Library query.
	pub docs_query: Option<String>,
	/// Effective Knowledge Workspace query.
	pub knowledge_query: Option<String>,
	/// Whether a graph subject was supplied.
	pub graph_subject_supplied: bool,
	/// Whether Dreaming proposals were included.
	pub include_dreaming: bool,
	/// Effective row cap per layer.
	pub limit: u32,
}
