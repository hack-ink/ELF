use crate::app::server::tests::tool_definitions::catalog;

#[test]
fn searches_notes_tool_description_mentions_payload_level_shapes() {
	let tools = catalog::build_tools();
	let tool =
		tools.get("elf_searches_notes").expect("Missing elf_searches_notes tool definition.");
	let description = tool.description.to_lowercase();

	assert_eq!(tool.path, "/v2/searches/{search_id}/notes");
	assert!(description.contains("l0"));
	assert!(description.contains("l1"));
	assert!(description.contains("l2"));
	assert!(description.contains("source_ref"));
	assert!(description.contains("structured"));
}
