use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, StructuredFields, structured_fields::StructuredEntity};
use elf_storage::graph;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn persist_graph_fields_tx(
	tx: &mut Transaction<'_, Postgres>,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
	scope: &str,
	note_id: Uuid,
	structured: &StructuredFields,
	now: OffsetDateTime,
) -> crate::Result<()> {
	if !structured.has_graph_fields() {
		return Ok(());
	}

	if let Some(entities) = structured.entities.as_ref() {
		for (entity_idx, entity) in entities.iter().enumerate() {
			let base_path = format!("structured.entities[{entity_idx}]");
			upsert_graph_entity_and_aliases(tx, tenant_id, project_id, entity, base_path.as_str())
				.await?;
		}
	}

	let relations = structured.relations.as_deref().unwrap_or_default();
	for (relation_idx, relation) in relations.iter().enumerate() {
		let relation_path = format!("structured.relations[{relation_idx}]");
		let subject = relation.subject.as_ref().ok_or_else(|| Error::InvalidRequest {
			message: format!("{relation_path}.subject is required."),
		})?;
		let predicate = relation.predicate.as_deref().ok_or_else(|| Error::InvalidRequest {
			message: format!("{relation_path}.predicate is required."),
		})?;

		let subject_entity_id = upsert_graph_entity_and_aliases(
			tx,
			tenant_id,
			project_id,
			subject,
			&format!("{relation_path}.subject"),
		)
		.await?;

		let valid_from = relation.valid_from.unwrap_or(now);
		let valid_to = relation.valid_to;
		if let Some(valid_to) = valid_to
			&& valid_to <= valid_from
		{
			return Err(Error::InvalidRequest {
				message: format!("{relation_path}.valid_to must be greater than valid_from."),
			});
		}

		let object = relation.object.as_ref().ok_or_else(|| Error::InvalidRequest {
			message: format!("{relation_path}.object is required."),
		})?;

		let (object_entity_id, object_value) = match (&object.entity, &object.value) {
			(Some(entity), None) => {
				let entity_id = upsert_graph_entity_and_aliases(
					tx,
					tenant_id,
					project_id,
					entity,
					&format!("{relation_path}.object.entity"),
				)
				.await?;
				(Some(entity_id), None)
			},
			(None, Some(value)) => (None, Some(value.as_str())),
			_ => {
				return Err(Error::InvalidRequest {
					message: format!(
						"{relation_path}.object must provide exactly one of entity or value.",
					),
				});
			},
		};

		graph::upsert_fact_with_evidence(
			tx,
			tenant_id,
			project_id,
			agent_id,
			scope,
			subject_entity_id,
			predicate,
			object_entity_id,
			object_value,
			valid_from,
			valid_to,
			&[note_id],
		)
		.await
		.map_err(|err| Error::Storage { message: err.to_string() })?;
	}

	Ok(())
}

async fn upsert_graph_entity_and_aliases(
	tx: &mut Transaction<'_, Postgres>,
	tenant_id: &str,
	project_id: &str,
	entity: &StructuredEntity,
	context_path: &str,
) -> crate::Result<Uuid> {
	let canonical = entity.canonical.as_deref().ok_or_else(|| Error::InvalidRequest {
		message: format!("{context_path}.canonical is required."),
	})?;

	let canonical = canonical.trim();
	let entity_id =
		graph::upsert_entity(tx, tenant_id, project_id, canonical, entity.kind.as_deref())
			.await
			.map_err(|err| Error::Storage { message: err.to_string() })?;

	if let Some(aliases) = entity.aliases.as_ref() {
		for (alias_idx, alias) in aliases.iter().enumerate() {
			let alias = alias.trim();
			if alias.is_empty() {
				return Err(Error::InvalidRequest {
					message: format!("{context_path}.aliases[{alias_idx}] must not be empty."),
				});
			}

			graph::upsert_entity_alias(tx, entity_id, alias)
				.await
				.map_err(|err| Error::Storage { message: err.to_string() })?;
		}
	}

	Ok(entity_id)
}
