use super::super::*;

pub(in crate::docs) fn docs_excerpt_locator(
	req: &DocsExcerptsGetRequest,
	selector_kind: &ExcerptsSelectorKind,
	match_start_offset: usize,
	match_end_offset: usize,
	content_hash: &str,
) -> DocsExcerptLocator {
	DocsExcerptLocator {
		span_id: source_span_id(
			content_hash,
			match_start_offset,
			match_end_offset,
			selector_kind.span_kind(),
		),
		selector_kind: selector_kind.as_str().to_string(),
		match_start_offset,
		match_end_offset,
		chunk_id: req.chunk_id,
		quote: req.quote.clone(),
		position: req.position.clone(),
	}
}

pub(in crate::docs) fn build_docs_l0_pointer(
	row: &DocSearchRow,
	chunk_id: Uuid,
) -> DocsSearchL0ItemPointer {
	let hashes = DocsSearchL0ItemHashes {
		content_hash: row.content_hash.clone(),
		chunk_hash: row.chunk_hash.clone(),
	};

	DocsSearchL0ItemPointer {
		schema: DOC_SOURCE_REF_SCHEMA_V1.to_string(),
		resolver: DOC_SOURCE_REF_RESOLVER_V1.to_string(),
		reference: DocsSearchL0ItemReference {
			doc_id: row.doc_id,
			chunk_id,
			source_record_id: row.doc_id,
			source_span_id: source_span_id(
				row.content_hash.as_str(),
				row.start_offset.max(0) as usize,
				row.end_offset.max(0) as usize,
				"captured",
			),
		},
		state: DocsSearchL0ItemState {
			content_hash: hashes.content_hash.clone(),
			chunk_hash: hashes.chunk_hash.clone(),
			doc_updated_at: row.updated_at,
		},
		hashes,
		locator: DocsSearchL0ItemLocator {
			span_id: source_span_id(
				row.content_hash.as_str(),
				row.start_offset.max(0) as usize,
				row.end_offset.max(0) as usize,
				"captured",
			),
			position: TextPositionSelector {
				start: row.start_offset.max(0) as usize,
				end: row.end_offset.max(0) as usize,
			},
		},
	}
}
