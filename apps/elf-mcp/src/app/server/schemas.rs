mod admin;
mod docs;
mod events;
mod graph;
mod memory;
mod notes;
mod search;
mod sharing;
mod work_journal;

pub(in crate::app::server) use self::{
	admin::{
		admin_ingestion_profile_default_get_schema, admin_ingestion_profile_default_set_schema,
		admin_ingestion_profile_get_schema, admin_ingestion_profile_versions_list_schema,
		admin_ingestion_profiles_create_schema, admin_ingestion_profiles_list_schema,
		admin_memory_history_get_schema, admin_note_provenance_get_schema,
		admin_trace_bundle_get_schema, admin_trace_get_schema, admin_trace_item_get_schema,
		admin_traces_recent_list_schema, admin_trajectory_get_schema,
	},
	docs::{docs_excerpts_get_schema, docs_get_schema, docs_put_schema, docs_search_l0_schema},
	events::events_ingest_schema,
	graph::{graph_query_schema, graph_report_schema},
	memory::{
		core_blocks_get_schema, dreaming_review_queue_schema, entity_memory_get_schema,
		recall_debug_panel_schema,
	},
	notes::{
		notes_get_schema, notes_ingest_schema, notes_list_schema, notes_patch_schema,
		notes_publish_schema, notes_unpublish_schema,
	},
	search::{
		searches_create_schema, searches_get_schema, searches_notes_schema,
		searches_timeline_schema,
	},
	sharing::{space_grant_revoke_schema, space_grant_upsert_schema, space_grants_list_schema},
	work_journal::{
		work_journal_entry_create_schema, work_journal_entry_get_schema,
		work_journal_session_readback_schema,
	},
};
