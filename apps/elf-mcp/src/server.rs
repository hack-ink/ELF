use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub method: HttpMethod,
    pub path: &'static str,
    pub description: &'static str,
    pub streaming: bool,
}

impl ToolDefinition {
    pub const fn new(
        name: &'static str,
        method: HttpMethod,
        path: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            method,
            path,
            description,
            streaming: true,
        }
    }
}

pub const TOOL_MEMORY_ADD_NOTE: &str = "memory_add_note";
pub const TOOL_MEMORY_ADD_EVENT: &str = "memory_add_event";
pub const TOOL_MEMORY_SEARCH: &str = "memory_search";
pub const TOOL_MEMORY_LIST: &str = "memory_list";
pub const TOOL_MEMORY_UPDATE: &str = "memory_update";
pub const TOOL_MEMORY_DELETE: &str = "memory_delete";

pub fn build_tools() -> HashMap<&'static str, ToolDefinition> {
    let tools = [
        ToolDefinition::new(
            TOOL_MEMORY_ADD_NOTE,
            HttpMethod::Post,
            "/v1/memory/add_note",
            "Add memory notes.",
        ),
        ToolDefinition::new(
            TOOL_MEMORY_ADD_EVENT,
            HttpMethod::Post,
            "/v1/memory/add_event",
            "Add memory extracted from event messages.",
        ),
        ToolDefinition::new(
            TOOL_MEMORY_SEARCH,
            HttpMethod::Post,
            "/v1/memory/search",
            "Search memory notes.",
        ),
        ToolDefinition::new(
            TOOL_MEMORY_LIST,
            HttpMethod::Get,
            "/v1/memory/list",
            "List memory notes.",
        ),
        ToolDefinition::new(
            TOOL_MEMORY_UPDATE,
            HttpMethod::Post,
            "/v1/memory/update",
            "Update memory notes.",
        ),
        ToolDefinition::new(
            TOOL_MEMORY_DELETE,
            HttpMethod::Post,
            "/v1/memory/delete",
            "Delete memory notes.",
        ),
    ];

    tools
        .into_iter()
        .map(|tool| (tool.name, tool))
        .collect()
}

pub fn serve_mcp(base_url: &str) -> color_eyre::Result<()> {
    let _tools = build_tools();
    // TODO: Replace this stub with a real MCP server once a library is available.
    Err(color_eyre::eyre::eyre!(
        "MCP server is not available for base URL {base_url} because no MCP library is configured."
    ))
}
