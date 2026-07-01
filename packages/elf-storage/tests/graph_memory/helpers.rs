use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_storage::{models::MemoryNote, queries};

pub(in crate::graph_memory) async fn insert_memory_note(
	executor: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
) -> Uuid {
	let note_id = Uuid::new_v4();
	let note = MemoryNote {
		note_id,
		tenant_id: tenant_id.to_string(),
		project_id: project_id.to_string(),
		agent_id: "agent-a".to_string(),
		scope: "scope-a".to_string(),
		r#type: "fact".to_string(),
		key: None,
		text: "graph note evidence".to_string(),
		importance: 1.0,
		confidence: 1.0,
		status: "active".to_string(),
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		expires_at: None,
		embedding_version: "test:vec:1".to_string(),
		source_ref: serde_json::json!({}),
		hit_count: 0,
		last_hit_at: None,
	};

	queries::insert_note(executor, &note).await.expect("Failed to insert evidence note.");

	note_id
}
