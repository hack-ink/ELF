use crate::routes::{ADMIN_VIEWER_PATH, VIEWER_HTML};

#[test]
fn admin_viewer_uses_admin_operator_routes_without_raw_memory_bypasses() {
	let html = VIEWER_HTML;

	assert_eq!(ADMIN_VIEWER_PATH, "/viewer");
	assert!(html.contains("/v2/admin/searches"));
	assert!(html.contains("/v2/admin/docs/search/l0"));
	assert!(html.contains("/v2/admin/docs/excerpts"));
	assert!(html.contains("/v2/admin/docs/${encodeURIComponent(item.doc_id)}"));
	assert!(html.contains("/v2/admin/dreaming/review-queue"));
	assert!(
		html.contains("/v2/admin/consolidation/proposals/${encodeURIComponent(proposalId)}/review")
	);
	assert!(html.contains("/v2/admin/notes/${encodeURIComponent(noteId)}/history"));
	assert!(html.contains("/v2/admin/notes/${encodeURIComponent(noteId)}/corrections"));
	assert!(html.contains("/v2/admin/recall-debug/panel"));
	assert!(html.contains("/v2/admin/traces/recent"));
	assert!(html.contains("/v2/admin/traces/${encodeURIComponent(traceId)}/bundle"));
	assert!(html.contains("/v2/admin/notes/"));
	assert!(html.contains("/v2/admin/knowledge/pages/search"));
	assert!(html.contains("mode: \"full\""));
	assert!(html.contains("candidates_limit: 200"));
	assert!(html.contains("Replay Candidates"));
	assert!(html.contains("Selected Final Results"));
	assert!(html.contains("Providers And Ranking"));
	assert!(html.contains("Relation Context"));
	assert!(html.contains("Knowledge Page Snippets"));
	assert!(html.contains("Derived page: source documents"));
	assert!(html.contains("Source Library"));
	assert!(html.contains("Memory Inbox"));
	assert!(html.contains("Memory History"));
	assert!(html.contains("Recall Debug"));
	assert!(html.contains("Apply Ledger Correction"));
	assert!(html.contains("Apply / Supersede"));
	assert!(html.contains("directTraceId"));
	assert!(html.contains("trace_id"));
	assert!(html.contains("loadInitialTrace"));
	assert!(!html.contains("method: \"PATCH\""));
	assert!(!html.contains("method: \"PUT\""));
	assert!(!html.contains("method: \"DELETE\""));
	assert!(!html.contains("/v2/notes/ingest"));
	assert!(!html.contains("/v2/events/ingest"));
	assert!(!html.contains("/publish"));
}
