use std::time::Duration;

use qdrant_client::{
	QdrantError,
	qdrant::{
		CreateCollectionBuilder, CreateFieldIndexCollection, Distance, FieldType, Modifier,
		PayloadSchemaType, SparseVectorParamsBuilder, SparseVectorsConfigBuilder,
		VectorParamsBuilder, VectorsConfigBuilder,
	},
};

use crate::{Error, Result};

pub const DENSE_VECTOR_NAME: &str = "dense";
pub const BM25_VECTOR_NAME: &str = "bm25";
pub const BM25_MODEL: &str = "qdrant/bm25";
pub const DOCS_SEARCH_FILTER_INDEXES: [(&str, PayloadSchemaType, FieldType); 9] = [
	("scope", PayloadSchemaType::Keyword, FieldType::Keyword),
	("status", PayloadSchemaType::Keyword, FieldType::Keyword),
	("doc_type", PayloadSchemaType::Keyword, FieldType::Keyword),
	("agent_id", PayloadSchemaType::Keyword, FieldType::Keyword),
	("updated_at", PayloadSchemaType::Datetime, FieldType::Datetime),
	("doc_ts", PayloadSchemaType::Datetime, FieldType::Datetime),
	("thread_id", PayloadSchemaType::Keyword, FieldType::Keyword),
	("domain", PayloadSchemaType::Keyword, FieldType::Keyword),
	("repo", PayloadSchemaType::Keyword, FieldType::Keyword),
];

const DEFAULT_QDRANT_CLIENT_TIMEOUT_SECS: u64 = 60;
const DEFAULT_QDRANT_OPERATION_TIMEOUT_SECS: u64 = 60;

pub struct QdrantStore {
	pub client: qdrant_client::Qdrant,
	pub collection: String,
	pub vector_dim: u32,
}
impl QdrantStore {
	pub fn new(cfg: &elf_config::Qdrant) -> Result<Self> {
		Self::new_with_collection(cfg, cfg.collection.as_str())
	}

	pub fn new_with_collection(cfg: &elf_config::Qdrant, collection: &str) -> Result<Self> {
		let client = qdrant_client::Qdrant::from_url(&cfg.url)
			.timeout(Duration::from_secs(DEFAULT_QDRANT_CLIENT_TIMEOUT_SECS))
			.build()?;

		Ok(Self { client, collection: collection.to_string(), vector_dim: cfg.vector_dim })
	}

	pub async fn ensure_collection(&self) -> Result<()> {
		match self.client.collection_info(&self.collection).await {
			Ok(_) => return Ok(()),
			Err(err) if is_qdrant_not_found(&err) => {},
			Err(err) => return Err(err.into()),
		}

		let mut vectors_config = VectorsConfigBuilder::default();

		vectors_config.add_named_vector_params(
			DENSE_VECTOR_NAME,
			VectorParamsBuilder::new(self.vector_dim.into(), Distance::Cosine),
		);

		let mut sparse_vectors_config = SparseVectorsConfigBuilder::default();

		sparse_vectors_config.add_named_vector_params(
			BM25_VECTOR_NAME,
			SparseVectorParamsBuilder::default().modifier(Modifier::Idf as i32),
		);

		let builder = CreateCollectionBuilder::new(self.collection.clone())
			.vectors_config(vectors_config)
			.sparse_vectors_config(sparse_vectors_config)
			.timeout(DEFAULT_QDRANT_OPERATION_TIMEOUT_SECS);

		match self.client.create_collection(builder).await {
			Ok(_) => Ok(()),
			Err(err) if is_qdrant_already_exists(&err) => Ok(()),
			Err(err) => Err(err.into()),
		}
	}

	pub async fn ensure_payload_indexes(
		&self,
		required_indexes: &[(&str, PayloadSchemaType, FieldType)],
	) -> Result<()> {
		let payload_schema = self
			.client
			.collection_info(&self.collection)
			.await?
			.result
			.map(|info| info.payload_schema)
			.unwrap_or_default();

		for (field_name, payload_type, field_type) in required_indexes.iter() {
			let existing = payload_schema.get(*field_name);

			if let Some(existing) = existing
				&& existing.data_type != *payload_type as i32
			{
				return Err(Error::Conflict(format!(
					"Qdrant collection {:?} has payload field {:?} with unexpected type (expected {:?}).",
					self.collection, field_name, payload_type
				)));
			}

			if existing.is_some() {
				continue;
			}

			let request = CreateFieldIndexCollection {
				collection_name: self.collection.clone(),
				wait: Some(true),
				field_name: (*field_name).to_string(),
				field_type: Some(*field_type as i32),
				field_index_params: None,
				ordering: None,
			};

			match self.client.create_field_index(request).await {
				Ok(_) => {},
				Err(err) if is_qdrant_already_exists(&err) => {},
				Err(err) => return Err(err.into()),
			}
		}

		Ok(())
	}
}

fn qdrant_error_code(err: &QdrantError) -> Option<String> {
	match err {
		QdrantError::ResponseError { status } => Some(format!("{:?}", status.code())),
		QdrantError::ResourceExhaustedError { status, .. } => Some(format!("{:?}", status.code())),
		_ => None,
	}
}

fn is_qdrant_not_found(err: &QdrantError) -> bool {
	qdrant_error_code(err).as_deref() == Some("NotFound")
}

fn is_qdrant_already_exists(err: &QdrantError) -> bool {
	qdrant_error_code(err).as_deref() == Some("AlreadyExists")
}
