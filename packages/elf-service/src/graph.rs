use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use elf_storage::graph;

#[allow(dead_code)]
pub(crate) struct GraphUpsertFactArgs<'a> {
	pub tenant_id: &'a str,
	pub project_id: &'a str,
	pub agent_id: &'a str,
	pub scope: &'a str,
	pub subject_entity_id: Uuid,
	pub predicate: &'a str,
	pub object_entity_id: Option<Uuid>,
	pub object_value: Option<&'a str>,
	pub valid_from: OffsetDateTime,
	pub valid_to: Option<OffsetDateTime>,
	pub evidence_note_ids: &'a [Uuid],
}

impl crate::ElfService {
	#[allow(dead_code)]
	pub(crate) async fn graph_upsert_fact(&self, args: GraphUpsertFactArgs<'_>) -> Result<Uuid> {
		let mut tx = self.db.pool.begin().await?;
		let fact_id = graph::insert_fact_with_evidence(
			&mut tx,
			args.tenant_id,
			args.project_id,
			args.agent_id,
			args.scope,
			args.subject_entity_id,
			args.predicate,
			args.object_entity_id,
			args.object_value,
			args.valid_from,
			args.valid_to,
			args.evidence_note_ids,
		)
		.await
		.map_err(|err| crate::Error::Storage { message: err.to_string() })?;

		tx.commit().await?;

		Ok(fact_id)
	}
}
