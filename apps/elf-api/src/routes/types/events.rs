use crate::routes::types::{Deserialize, EventMessage, IngestionProfileSelector};

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct EventsIngestRequest {
	pub(in crate::routes) scope: Option<String>,
	pub(in crate::routes) dry_run: Option<bool>,
	pub(in crate::routes) ingestion_profile: Option<IngestionProfileSelector>,
	pub(in crate::routes) messages: Vec<EventMessage>,
}
