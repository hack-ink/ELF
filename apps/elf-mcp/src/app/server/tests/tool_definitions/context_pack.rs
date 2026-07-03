use crate::app::server::tests::tool_definitions::catalog;

#[test]
fn context_pack_tool_uses_public_agent_route() {
	let tools = catalog::build_tools();
	let tool = tools.get("elf_context_pack_build").expect("Missing context pack tool.");

	assert_eq!(tool.path, "/v2/context-packs");
	assert!(tool.description.contains("read-time scoped view"));
}
