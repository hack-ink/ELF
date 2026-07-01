pub(in crate::quantitative) fn aggregate_qrel_source(
	ranking_query_count: usize,
	explicit_qrel_query_count: usize,
) -> &'static str {
	if ranking_query_count == 0 {
		"not_encoded"
	} else if explicit_qrel_query_count == ranking_query_count {
		"explicit_qrels"
	} else if explicit_qrel_query_count == 0 {
		"expected_evidence_fallback"
	} else {
		"mixed"
	}
}
