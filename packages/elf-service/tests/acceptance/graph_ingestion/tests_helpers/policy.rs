use elf_domain::memory_policy::MemoryPolicyDecision;
use elf_service::NoteOp;

pub(in crate::acceptance::graph_ingestion) fn assert_graph_policy_from_op(
	op: NoteOp,
	policy_decision: MemoryPolicyDecision,
) {
	match op {
		NoteOp::Add => assert_eq!(policy_decision, MemoryPolicyDecision::Remember),
		NoteOp::Update => assert_eq!(policy_decision, MemoryPolicyDecision::Update),
		_ => {},
	}
}
