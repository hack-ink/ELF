use elf_config::{Memory, MemoryPolicy, MemoryPolicyRule};

pub(crate) fn memory_policy_memory_config() -> Memory {
	Memory {
		max_notes_per_add_event: 3,
		max_note_chars: 240,
		dup_sim_threshold: 0.92,
		update_sim_threshold: 0.85,
		candidate_k: 60,
		top_k: 12,
		policy: MemoryPolicy {
			rules: vec![
				MemoryPolicyRule {
					note_type: Some("fact".to_string()),
					scope: Some("agent_private".to_string()),
					min_confidence: Some(0.9),
					min_importance: Some(0.1),
				},
				MemoryPolicyRule {
					note_type: Some("preference".to_string()),
					scope: Some("agent_private".to_string()),
					min_confidence: Some(0.75),
					min_importance: None,
				},
				MemoryPolicyRule {
					note_type: Some("preference".to_string()),
					scope: None,
					min_confidence: Some(0.6),
					min_importance: None,
				},
				MemoryPolicyRule {
					note_type: None,
					scope: None,
					min_confidence: None,
					min_importance: None,
				},
			],
		},
	}
}
