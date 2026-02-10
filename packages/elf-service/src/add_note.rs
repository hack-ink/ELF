use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use elf_domain::{cjk, ttl, writegate};
use elf_storage::models::MemoryNote;

use crate::{
	ElfService, Error, InsertVersionArgs, NoteOp, ResolveUpdateArgs, Result, UpdateDecision,
	structured_fields::{
		StructuredFields, upsert_structured_fields_tx, validate_structured_fields,
	},
};

const REJECT_STRUCTURED_INVALID: &str = "REJECT_STRUCTURED_INVALID";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddNoteRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: String,
	pub notes: Vec<AddNoteInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddNoteInput {
	pub r#type: String,
	pub key: Option<String>,
	pub text: String,
	#[serde(default)]
	pub structured: Option<StructuredFields>,
	pub importance: f32,
	pub confidence: f32,
	pub ttl_days: Option<i64>,
	pub source_ref: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddNoteResult {
	pub note_id: Option<Uuid>,
	pub op: NoteOp,
	pub reason_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddNoteResponse {
	pub results: Vec<AddNoteResult>,
}

impl ElfService {
	pub async fn add_note(&self, req: AddNoteRequest) -> Result<AddNoteResponse> {
		if req.notes.is_empty() {
			return Err(Error::InvalidRequest { message: "Notes list is empty.".to_string() });
		}
		if req.tenant_id.trim().is_empty()
			|| req.project_id.trim().is_empty()
			|| req.agent_id.trim().is_empty()
			|| req.scope.trim().is_empty()
		{
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, agent_id, and scope are required.".to_string(),
			});
		}

		for (idx, note) in req.notes.iter().enumerate() {
			if cjk::contains_cjk(&note.text) {
				return Err(Error::NonEnglishInput { field: format!("$.notes[{idx}].text") });
			}

			if let Some(key) = &note.key
				&& cjk::contains_cjk(key)
			{
				return Err(Error::NonEnglishInput { field: format!("$.notes[{idx}].key") });
			}
			if let Some(path) = find_cjk_path_in_structured(
				note.structured.as_ref(),
				&format!("$.notes[{idx}].structured"),
			) {
				return Err(Error::NonEnglishInput { field: path });
			}
			if let Some(path) =
				find_cjk_path(&note.source_ref, &format!("$.notes[{idx}].source_ref"))
			{
				return Err(Error::NonEnglishInput { field: path });
			}
		}

		let now = OffsetDateTime::now_utc();
		let embed_version = crate::embedding_version(&self.cfg);
		let mut results = Vec::with_capacity(req.notes.len());

		for note in req.notes {
			if let Some(structured) = note.structured.as_ref()
				&& let Err(err) =
					validate_structured_fields(structured, &note.text, &note.source_ref, None)
			{
				results.push(AddNoteResult {
					note_id: None,
					op: NoteOp::Rejected,
					reason_code: Some(REJECT_STRUCTURED_INVALID.to_string()),
				});
				tracing::info!(error = %err, "Rejecting note due to invalid structured fields.");

				continue;
			}

			let gate_input = writegate::NoteInput {
				note_type: note.r#type.clone(),
				scope: req.scope.clone(),
				text: note.text.clone(),
			};

			if let Err(code) = writegate::writegate(&gate_input, &self.cfg) {
				results.push(AddNoteResult {
					note_id: None,
					op: NoteOp::Rejected,
					reason_code: Some(crate::writegate_reason_code(code).to_string()),
				});

				continue;
			}

			let mut tx = self.db.pool.begin().await?;
			let decision = crate::resolve_update(
				&mut *tx,
				ResolveUpdateArgs {
					cfg: &self.cfg,
					providers: &self.providers,
					tenant_id: &req.tenant_id,
					project_id: &req.project_id,
					agent_id: &req.agent_id,
					scope: &req.scope,
					note_type: &note.r#type,
					key: note.key.as_deref(),
					text: &note.text,
					now,
				},
			)
			.await?;

			match decision {
				UpdateDecision::Add { note_id } => {
					let expires_at =
						ttl::compute_expires_at(note.ttl_days, &note.r#type, &self.cfg, now);
					let memory_note = MemoryNote {
						note_id,
						tenant_id: req.tenant_id.clone(),
						project_id: req.project_id.clone(),
						agent_id: req.agent_id.clone(),
						scope: req.scope.clone(),
						r#type: note.r#type.clone(),
						key: note.key.clone(),
						text: note.text.clone(),
						importance: note.importance,
						confidence: note.confidence,
						status: "active".to_string(),
						created_at: now,
						updated_at: now,
						expires_at,
						embedding_version: embed_version.clone(),
						source_ref: note.source_ref.clone(),
						hit_count: 0,
						last_hit_at: None,
					};

					sqlx::query!(
						"\
INSERT INTO memory_notes (
	note_id,
	tenant_id,
	project_id,
	agent_id,
	scope,
	type,
	key,
	text,
	importance,
	confidence,
	status,
	created_at,
	updated_at,
	expires_at,
	embedding_version,
	source_ref,
	hit_count,
	last_hit_at
)
VALUES (
	$1,
	$2,
	$3,
	$4,
	$5,
	$6,
	$7,
	$8,
	$9,
	$10,
	$11,
	$12,
	$13,
	$14,
	$15,
	$16,
	$17,
	$18
)",
						memory_note.note_id,
						memory_note.tenant_id.as_str(),
						memory_note.project_id.as_str(),
						memory_note.agent_id.as_str(),
						memory_note.scope.as_str(),
						memory_note.r#type.as_str(),
						memory_note.key.as_deref(),
						memory_note.text.as_str(),
						memory_note.importance,
						memory_note.confidence,
						memory_note.status.as_str(),
						memory_note.created_at,
						memory_note.updated_at,
						memory_note.expires_at,
						memory_note.embedding_version.as_str(),
						&memory_note.source_ref,
						memory_note.hit_count,
						memory_note.last_hit_at,
					)
					.execute(&mut *tx)
					.await?;

					crate::insert_version(
						&mut *tx,
						InsertVersionArgs {
							note_id: memory_note.note_id,
							op: "ADD",
							prev_snapshot: None,
							new_snapshot: Some(crate::note_snapshot(&memory_note)),
							reason: "add_note",
							actor: "add_note",
							ts: now,
						},
					)
					.await?;

					if let Some(structured) = note.structured.as_ref()
						&& !structured.is_effectively_empty()
					{
						upsert_structured_fields_tx(&mut tx, memory_note.note_id, structured, now)
							.await?;
					}

					crate::enqueue_outbox_tx(
						&mut *tx,
						memory_note.note_id,
						"UPSERT",
						&memory_note.embedding_version,
						now,
					)
					.await?;

					tx.commit().await?;
					results.push(AddNoteResult {
						note_id: Some(note_id),
						op: NoteOp::Add,
						reason_code: None,
					});
				},
				UpdateDecision::Update { note_id } => {
					let mut existing: MemoryNote = sqlx::query_as!(
						MemoryNote,
						"SELECT * FROM memory_notes WHERE note_id = $1 FOR UPDATE",
						note_id,
					)
					.fetch_one(&mut *tx)
					.await?;
					let prev_snapshot = crate::note_snapshot(&existing);
					let requested_ttl = note.ttl_days.filter(|days| *days > 0);
					let expires_at = match requested_ttl {
						Some(ttl) =>
							ttl::compute_expires_at(Some(ttl), &note.r#type, &self.cfg, now),
						None => existing.expires_at,
					};

					let expires_match = if let Some(ttl_days) = requested_ttl {
						match existing.expires_at {
							Some(existing_expires_at) => {
								let existing_ttl =
									(existing_expires_at - existing.updated_at).whole_days() as i64;
								existing_ttl == ttl_days
							},
							None => false,
						}
					} else {
						existing.expires_at == expires_at
					};
					let unchanged = existing.text == note.text
						&& (existing.importance - note.importance).abs() <= f32::EPSILON
						&& (existing.confidence - note.confidence).abs() <= f32::EPSILON
						&& expires_match && existing.source_ref == note.source_ref;

					if unchanged {
						tx.commit().await?;
						results.push(AddNoteResult {
							note_id: Some(note_id),
							op: NoteOp::None,
							reason_code: None,
						});

						continue;
					}

					existing.text = note.text.clone();
					existing.importance = note.importance;
					existing.confidence = note.confidence;
					existing.updated_at = now;
					existing.expires_at = expires_at;
					existing.source_ref = note.source_ref.clone();

					sqlx::query!(
						"\
UPDATE memory_notes
SET
	text = $1,
importance = $2,
confidence = $3,
updated_at = $4,
	expires_at = $5,
	source_ref = $6
WHERE note_id = $7",
						existing.text.as_str(),
						existing.importance,
						existing.confidence,
						existing.updated_at,
						existing.expires_at,
						&existing.source_ref,
						existing.note_id,
					)
					.execute(&mut *tx)
					.await?;

					crate::insert_version(
						&mut *tx,
						InsertVersionArgs {
							note_id: existing.note_id,
							op: "UPDATE",
							prev_snapshot: Some(prev_snapshot),
							new_snapshot: Some(crate::note_snapshot(&existing)),
							reason: "add_note",
							actor: "add_note",
							ts: now,
						},
					)
					.await?;

					if let Some(structured) = note.structured.as_ref()
						&& !structured.is_effectively_empty()
					{
						upsert_structured_fields_tx(&mut tx, existing.note_id, structured, now)
							.await?;
					}

					crate::enqueue_outbox_tx(
						&mut *tx,
						existing.note_id,
						"UPSERT",
						&existing.embedding_version,
						now,
					)
					.await?;

					tx.commit().await?;
					results.push(AddNoteResult {
						note_id: Some(note_id),
						op: NoteOp::Update,
						reason_code: None,
					});
				},
				UpdateDecision::None { note_id } => {
					if let Some(structured) = note.structured.as_ref()
						&& !structured.is_effectively_empty()
					{
						upsert_structured_fields_tx(&mut tx, note_id, structured, now).await?;

						crate::enqueue_outbox_tx(
							&mut *tx,
							note_id,
							"UPSERT",
							embed_version.as_str(),
							now,
						)
						.await?;

						tx.commit().await?;
						results.push(AddNoteResult {
							note_id: Some(note_id),
							op: NoteOp::Update,
							reason_code: None,
						});
						continue;
					}

					tx.commit().await?;
					results.push(AddNoteResult {
						note_id: Some(note_id),
						op: NoteOp::None,
						reason_code: None,
					});
				},
			}
		}

		Ok(AddNoteResponse { results })
	}
}

fn find_cjk_path_in_structured(
	structured: Option<&StructuredFields>,
	base: &str,
) -> Option<String> {
	let structured = structured?;

	if let Some(summary) = structured.summary.as_ref()
		&& cjk::contains_cjk(summary)
	{
		return Some(format!("{base}.summary"));
	}
	if let Some(items) = structured.facts.as_ref() {
		for (idx, item) in items.iter().enumerate() {
			if cjk::contains_cjk(item) {
				return Some(format!("{base}.facts[{idx}]"));
			}
		}
	}
	if let Some(items) = structured.concepts.as_ref() {
		for (idx, item) in items.iter().enumerate() {
			if cjk::contains_cjk(item) {
				return Some(format!("{base}.concepts[{idx}]"));
			}
		}
	}

	None
}

fn find_cjk_path(value: &Value, path: &str) -> Option<String> {
	match value {
		Value::String(text) =>
			if cjk::contains_cjk(text) {
				Some(path.to_string())
			} else {
				None
			},
		Value::Array(items) => {
			for (idx, item) in items.iter().enumerate() {
				let child_path = format!("{path}[{idx}]");

				if let Some(found) = find_cjk_path(item, &child_path) {
					return Some(found);
				}
			}
			None
		},
		Value::Object(map) => {
			for (key, value) in map.iter() {
				let child_path = format!("{path}[\"{}\"]", escape_json_path_key(key));

				if let Some(found) = find_cjk_path(value, &child_path) {
					return Some(found);
				}
			}
			None
		},
		_ => None,
	}
}

fn escape_json_path_key(key: &str) -> String {
	key.replace('\\', "\\\\").replace('"', "\\\"")
}
