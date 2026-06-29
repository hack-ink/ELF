use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Providers, Result};
use elf_config::Config;

const RESOLVE_UPDATE_QUERY: &str = "\
WITH key_match AS (
	SELECT note_id
	FROM memory_notes
	WHERE tenant_id = $1
		AND project_id = $2
		AND agent_id = $3
		AND scope = $4
		AND type = $5
		AND $6::text IS NOT NULL
		AND key = $6
		AND status = 'active'
		AND (expires_at IS NULL OR expires_at > $7)
	LIMIT 1
),
existing AS (
	SELECT note_id
	FROM memory_notes
	WHERE tenant_id = $1
		AND project_id = $2
		AND agent_id = $3
		AND scope = $4
		AND type = $5
		AND status = 'active'
		AND (expires_at IS NULL OR expires_at > $7)
),
best AS (
	SELECT
		note_id,
		(1 - (vec <=> $8::text::vector))::real AS similarity
	FROM note_embeddings
	WHERE note_id = ANY(ARRAY(SELECT note_id FROM existing))
		AND embedding_version = $9
	ORDER BY similarity DESC
	LIMIT 1
)
	SELECT
		(SELECT note_id FROM key_match) AS key_note_id,
		(SELECT note_id FROM best) AS best_note_id,
		(SELECT similarity FROM best) AS best_similarity";

#[derive(Clone, Copy, Debug)]
pub(crate) enum UpdateDecision {
	Add { note_id: Uuid, metadata: UpdateDecisionMetadata },
	Update { note_id: Uuid, metadata: UpdateDecisionMetadata },
	None { note_id: Uuid, metadata: UpdateDecisionMetadata },
}
impl UpdateDecision {
	pub(crate) fn note_id(&self) -> Uuid {
		match self {
			Self::Add { note_id, .. }
			| Self::Update { note_id, .. }
			| Self::None { note_id, .. } => *note_id,
		}
	}

	pub(crate) fn metadata(&self) -> UpdateDecisionMetadata {
		match self {
			Self::Add { metadata, .. }
			| Self::Update { metadata, .. }
			| Self::None { metadata, .. } => *metadata,
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct UpdateDecisionMetadata {
	pub similarity_best: Option<f32>,
	pub key_match: bool,
	pub matched_dup: bool,
}

pub(crate) struct ResolveUpdateArgs<'a> {
	pub(crate) cfg: &'a Config,
	pub(crate) providers: &'a Providers,
	pub(crate) tenant_id: &'a str,
	pub(crate) project_id: &'a str,
	pub(crate) agent_id: &'a str,
	pub(crate) scope: &'a str,
	pub(crate) note_type: &'a str,
	pub(crate) key: Option<&'a str>,
	pub(crate) text: &'a str,
	pub(crate) now: OffsetDateTime,
}

pub(crate) async fn resolve_update<'e, E>(
	executor: E,
	args: ResolveUpdateArgs<'_>,
) -> Result<UpdateDecision>
where
	E: PgExecutor<'e>,
{
	let ResolveUpdateArgs {
		cfg,
		providers,
		tenant_id,
		project_id,
		agent_id,
		scope,
		note_type,
		key,
		text,
		now,
	} = args;
	let embeddings =
		providers.embedding.embed(&cfg.providers.embedding, &[text.to_string()]).await?;
	let Some(vec) = embeddings.into_iter().next() else {
		return Err(Error::Provider {
			message: "Embedding provider returned no vectors.".to_string(),
		});
	};

	if vec.len() != cfg.storage.qdrant.vector_dim as usize {
		return Err(Error::Provider {
			message: "Embedding vector dimension mismatch.".to_string(),
		});
	}

	let vec_text = crate::vector_to_pg(&vec);
	let embed_version = crate::embedding_version(cfg);
	let key = key.map(|value| value.trim()).filter(|value| !value.is_empty());
	let row: (Option<Uuid>, Option<Uuid>, Option<f32>) = sqlx::query_as(RESOLVE_UPDATE_QUERY)
		.bind(tenant_id)
		.bind(project_id)
		.bind(agent_id)
		.bind(scope)
		.bind(note_type)
		.bind(key)
		.bind(now)
		.bind(vec_text.as_str())
		.bind(embed_version.as_str())
		.fetch_one(executor)
		.await?;
	let (key_note_id, best_note_id, best_similarity) = row;

	if let Some(note_id) = key_note_id {
		return Ok(UpdateDecision::Update {
			note_id,
			metadata: UpdateDecisionMetadata {
				similarity_best: None,
				key_match: true,
				matched_dup: false,
			},
		});
	}

	let Some(best_id) = best_note_id else {
		return Ok(UpdateDecision::Add {
			note_id: Uuid::new_v4(),
			metadata: UpdateDecisionMetadata {
				similarity_best: None,
				key_match: false,
				matched_dup: false,
			},
		});
	};
	let Some(best_score) = best_similarity else {
		return Ok(UpdateDecision::Add {
			note_id: Uuid::new_v4(),
			metadata: UpdateDecisionMetadata {
				similarity_best: None,
				key_match: false,
				matched_dup: false,
			},
		});
	};

	if best_score >= cfg.memory.dup_sim_threshold {
		return Ok(UpdateDecision::None {
			note_id: best_id,
			metadata: UpdateDecisionMetadata {
				similarity_best: Some(best_score),
				key_match: false,
				matched_dup: true,
			},
		});
	}
	if best_score >= cfg.memory.update_sim_threshold {
		return Ok(UpdateDecision::Update {
			note_id: best_id,
			metadata: UpdateDecisionMetadata {
				similarity_best: Some(best_score),
				key_match: false,
				matched_dup: false,
			},
		});
	}

	Ok(UpdateDecision::Add {
		note_id: Uuid::new_v4(),
		metadata: UpdateDecisionMetadata {
			similarity_best: Some(best_score),
			key_match: false,
			matched_dup: false,
		},
	})
}
