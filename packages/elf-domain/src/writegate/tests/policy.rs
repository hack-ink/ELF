use crate::writegate::{
	self, WritePolicy, WritePolicyResult, WriteRedaction, WriteRedactionResult,
};

#[test]
fn applies_empty_policy_as_noop() {
	let policy = WritePolicy::default();

	assert_eq!(
		writegate::apply_write_policy("keep this", Some(&policy)),
		Ok(WritePolicyResult {
			transformed: "keep this".to_string(),
			..WritePolicyResult::default()
		})
	);
}

#[test]
fn applies_exclusion_span() {
	let policy = WritePolicy {
		exclusions: vec![crate::writegate::WriteSpan { start: 4, end: 9 }],
		redactions: vec![],
	};
	let actual = writegate::apply_write_policy("hello world", Some(&policy))
		.expect("policy apply should succeed");

	assert_eq!(actual.transformed, "hellld");
	assert_eq!(actual.audit.exclusions, vec![crate::writegate::WriteSpan { start: 4, end: 9 }]);
	assert!(actual.audit.redactions.is_empty());
}

#[test]
fn applies_simple_replacement_redaction() {
	let policy = WritePolicy {
		exclusions: vec![],
		redactions: vec![WriteRedaction::Replace {
			span: crate::writegate::WriteSpan { start: 4, end: 5 },
			replacement: "***".to_string(),
		}],
	};
	let actual = writegate::apply_write_policy("secret", Some(&policy))
		.expect("policy apply should succeed");

	assert_eq!(actual.transformed, "secr***t");
	assert_eq!(
		actual.audit.redactions,
		vec![WriteRedactionResult {
			span: crate::writegate::WriteSpan { start: 4, end: 5 },
			replacement: "***".to_string(),
		}]
	);
	assert!(actual.audit.exclusions.is_empty());
}
