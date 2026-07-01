use crate::app::server::tests::tool_definitions::catalog;

#[test]
fn recall_debug_tool_uses_public_agent_route() {
	let tools = catalog::build_tools();
	let tool = tools.get("elf_recall_debug_panel").expect("Missing recall debug panel tool.");

	assert_eq!(tool.path, "/v2/recall-debug/panel");
	assert!(tool.description.contains("recall_trace"));
}
