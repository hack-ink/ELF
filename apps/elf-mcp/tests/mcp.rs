#[path = "../src/server.rs"]
mod server;

#[test]
fn registers_all_tools() {
	let tools = server::build_tools();
	let expected = [
		server::TOOL_MEMORY_ADD_NOTE,
		server::TOOL_MEMORY_ADD_EVENT,
		server::TOOL_MEMORY_SEARCH,
		server::TOOL_MEMORY_LIST,
		server::TOOL_MEMORY_UPDATE,
		server::TOOL_MEMORY_DELETE,
	];

	for name in expected {
		assert!(tools.contains_key(name), "Missing tool registration: {name}.");
	}

	assert_eq!(tools.len(), expected.len(), "Unexpected tool count for MCP registration.");
}
