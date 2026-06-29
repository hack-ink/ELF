use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct NotesIngestRequest {
	pub(in crate::routes) scope: String,
	pub(in crate::routes) notes: Vec<AddNoteInput>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct NotesListQuery {
	pub(in crate::routes) scope: Option<String>,
	pub(in crate::routes) status: Option<String>,
	pub(in crate::routes) r#type: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct NotePatchRequest {
	pub(in crate::routes) text: Option<String>,
	pub(in crate::routes) importance: Option<f32>,
	pub(in crate::routes) confidence: Option<f32>,
	pub(in crate::routes) ttl_days: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct AdminNoteCorrectionBody {
	pub(in crate::routes) action: MemoryCorrectionAction,
	pub(in crate::routes) reason: String,
	pub(in crate::routes) source_ref: Value,
	pub(in crate::routes) restore_version_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::routes) struct PublishResponseV2 {
	pub(in crate::routes) note_id: Uuid,
	pub(in crate::routes) space: String,
}
