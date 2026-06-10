//! Graph retrieval and mutation APIs.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result};
use elf_storage::graph;

/// Temporal state for a graph relation fact relative to a read timestamp.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationTemporalStatus {
	/// The fact's validity window starts after the read timestamp.
	Future,
	/// The fact is valid at the read timestamp.
	#[default]
	Current,
	/// The fact was invalidated before or at the read timestamp.
	Historical,
}

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

impl ElfService {
	#[allow(dead_code)]
	pub(crate) async fn graph_upsert_fact(&self, args: GraphUpsertFactArgs<'_>) -> Result<Uuid> {
		let mut tx = self.db.pool.begin().await?;
		let predicate = graph::resolve_or_register_predicate(
			&mut tx,
			args.tenant_id,
			args.project_id,
			args.predicate,
		)
		.await
		.map_err(|err| Error::Storage { message: err.to_string() })?;
		let fact_id = graph::insert_fact_with_evidence(
			&mut tx,
			args.tenant_id,
			args.project_id,
			args.agent_id,
			args.scope,
			args.subject_entity_id,
			args.predicate,
			predicate.predicate_id,
			args.object_entity_id,
			args.object_value,
			args.valid_from,
			args.valid_to,
			args.evidence_note_ids,
		)
		.await
		.map_err(|err| Error::Storage { message: err.to_string() })?;

		tx.commit().await?;

		Ok(fact_id)
	}
}

pub(crate) fn relation_temporal_status(
	valid_from: OffsetDateTime,
	valid_to: Option<OffsetDateTime>,
	read_at: OffsetDateTime,
) -> RelationTemporalStatus {
	if valid_from > read_at {
		return RelationTemporalStatus::Future;
	}
	if valid_to.is_some_and(|valid_to| valid_to <= read_at) {
		return RelationTemporalStatus::Historical;
	}

	RelationTemporalStatus::Current
}
