pub(in crate::materialization) fn is_elf_dreaming_readback_live_adapter(
	adapter_id: &str,
	suite: &str,
) -> bool {
	matches!(suite, "memory_summary" | "proactive_brief" | "scheduled_memory")
		&& matches!(adapter_id, "elf_service_native_dreaming" | "elf_live_real_world")
}

pub(in crate::materialization) fn is_operator_debug_live_adapter(
	adapter_id: &str,
	suite: &str,
) -> bool {
	suite == "operator_debugging_ux"
		&& matches!(
			adapter_id,
			"elf_live_real_world"
				| "qmd_live_real_world"
				| "elf_operator_debug_live"
				| "qmd_operator_debug_live"
		)
}

pub(in crate::materialization) fn is_elf_consolidation_live_adapter(
	adapter_id: &str,
	suite: &str,
) -> bool {
	suite == "consolidation" && adapter_id == "elf_live_real_world"
}

pub(in crate::materialization) fn is_elf_knowledge_live_adapter(
	adapter_id: &str,
	suite: &str,
) -> bool {
	suite == "knowledge_compilation" && adapter_id == "elf_live_real_world"
}

pub(in crate::materialization) fn is_elf_capture_live_adapter(
	adapter_id: &str,
	suite: &str,
) -> bool {
	suite == "capture_integration"
		&& matches!(adapter_id, "elf_live_real_world" | "elf_capture_write_policy_live")
}
