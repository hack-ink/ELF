use qdrant_client::qdrant::{
	CreateFieldIndexCollection, FieldType, GetPointsBuilder, PayloadSchemaType, RetrievedPoint,
};
use uuid::Uuid;

use elf_service::ElfService;

const DOCS_SEARCH_FILTER_INDEXES: [(&str, PayloadSchemaType, FieldType); 9] = [
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

pub(crate) async fn fetch_first_doc_chunk_id(db: &ElfService, doc_id: Uuid) -> Option<Uuid> {
	sqlx::query_scalar::<_, Uuid>(
		"SELECT chunk_id FROM doc_chunks WHERE doc_id = $1 ORDER BY chunk_index LIMIT 1",
	)
	.bind(doc_id)
	.fetch_optional(&db.db.pool)
	.await
	.expect("Failed to fetch doc chunk id.")
}

pub(crate) async fn fetch_first_doc_chunk_point(
	service: &ElfService,
	doc_id: Uuid,
) -> Option<RetrievedPoint> {
	let chunk_id = fetch_first_doc_chunk_id(service, doc_id).await?;
	let response = service
		.qdrant
		.client
		.get_points(
			GetPointsBuilder::new(
				service.cfg.storage.qdrant.docs_collection.clone(),
				vec![chunk_id.to_string().into()],
			)
			.with_payload(true),
		)
		.await
		.expect("Failed to fetch doc chunk point from Qdrant.");

	response.result.into_iter().next()
}

pub(crate) async fn verify_docs_qdrant_filter_indexes(service: &ElfService) {
	let mut payload_schema = service
		.qdrant
		.client
		.collection_info(&service.cfg.storage.qdrant.docs_collection)
		.await
		.expect("Failed to fetch Qdrant docs collection info.")
		.result
		.expect("Qdrant collection info is missing.")
		.payload_schema;

	for (field_name, payload_type, index_type) in DOCS_SEARCH_FILTER_INDEXES {
		let missing_or_wrong = match payload_schema.get(field_name) {
			Some(schema) => schema.data_type != payload_type as i32,
			None => true,
		};

		if missing_or_wrong {
			let request = CreateFieldIndexCollection {
				collection_name: service.cfg.storage.qdrant.docs_collection.clone(),
				wait: Some(true),
				field_name: field_name.to_string(),
				field_type: Some(index_type as i32),
				field_index_params: None,
				ordering: None,
				timeout: None,
			};

			service
				.qdrant
				.client
				.create_field_index(request)
				.await
				.expect("Failed to create required Qdrant payload index.");
		}
	}

	payload_schema = service
		.qdrant
		.client
		.collection_info(&service.cfg.storage.qdrant.docs_collection)
		.await
		.expect("Failed to fetch Qdrant docs collection info.")
		.result
		.expect("Qdrant collection info is missing.")
		.payload_schema;

	for (field_name, payload_type, _) in DOCS_SEARCH_FILTER_INDEXES {
		let schema = payload_schema.get(field_name).expect("Expected required payload field.");

		assert_eq!(
			schema.data_type, payload_type as i32,
			"Unexpected payload type for {field_name}."
		);
	}
}
