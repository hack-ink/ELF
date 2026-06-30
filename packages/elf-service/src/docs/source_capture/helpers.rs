mod identity;
mod spans;
mod time;

pub(super) use self::{
	identity::{source_identity_value, source_origin, source_type},
	spans::{source_policy_spans, source_spans_to_value},
	time::format_timestamp,
};

#[cfg(test)]
mod tests {
	use crate::docs::{DocType, Map, Value, source_capture::helpers};
	use elf_domain::writegate::{WritePolicyAudit, WriteRedactionResult, WriteSpan};

	fn source_ref(value: Value) -> Map<String, Value> {
		value.as_object().expect("source_ref should be an object").clone()
	}

	#[test]
	fn source_origin_prefers_canonical_uri_and_preserves_fallbacks() {
		let canonical = source_ref(serde_json::json!({
			"canonical_uri": "https://example.com/canonical",
			"url": "https://example.com/url",
			"uri": "file:///tmp/source.txt"
		}));
		let chat = source_ref(serde_json::json!({
			"thread_id": "thread-a",
			"message_id": "message-b"
		}));
		let dev = source_ref(serde_json::json!({
			"repo": "hack-ink/ELF",
			"path": "packages/elf-service/src/docs.rs",
			"pr_number": 278
		}));
		let knowledge = source_ref(serde_json::json!({
			"ts": "2026-02-25T12:00:00Z"
		}));

		assert_eq!(
			helpers::source_origin(&canonical, DocType::Knowledge),
			"https://example.com/canonical"
		);
		assert_eq!(helpers::source_origin(&chat, DocType::Chat), "thread:thread-a#message-b");
		assert_eq!(
			helpers::source_origin(&dev, DocType::Dev),
			"repo:hack-ink/ELF/packages/elf-service/src/docs.rs#pr-278"
		);
		assert_eq!(
			helpers::source_origin(&knowledge, DocType::Knowledge),
			"knowledge:2026-02-25T12:00:00Z"
		);
	}

	#[test]
	fn source_identity_value_prefers_canonical_uri_and_preserves_type_shape() {
		let canonical = source_ref(serde_json::json!({
			"canonical_uri": "https://example.com/canonical",
			"uri": "file:///tmp/source.txt"
		}));
		let dev = source_ref(serde_json::json!({
			"repo": "hack-ink/ELF",
			"path": "packages/elf-service/src/docs.rs",
			"commit_sha": "abc123",
			"pr_number": 278
		}));

		assert_eq!(
			helpers::source_identity_value(&canonical, DocType::Knowledge),
			serde_json::json!(["canonical_uri", "https://example.com/canonical"])
		);
		assert_eq!(
			helpers::source_identity_value(&dev, DocType::Dev),
			serde_json::json!([
				"dev",
				"hack-ink/ELF",
				"packages/elf-service/src/docs.rs",
				"abc123",
				278,
				null
			])
		);
	}

	#[test]
	fn source_policy_spans_preserve_write_policy_order_and_reasons() {
		let audit = WritePolicyAudit {
			exclusions: vec![WriteSpan { start: 4, end: 10 }],
			redactions: vec![WriteRedactionResult {
				span: WriteSpan { start: 16, end: 22 },
				replacement: "[redacted]".to_string(),
			}],
		};
		let spans = helpers::source_policy_spans("raw-content-hash", Some(&audit));

		assert_eq!(spans.len(), 2);
		assert_eq!(spans[0].status, "excluded");
		assert_eq!(spans[0].reason_code.as_deref(), Some("WRITE_POLICY_EXCLUSION"));
		assert_eq!(spans[0].start_offset, 4);
		assert_eq!(spans[0].end_offset, 10);
		assert_eq!(spans[1].status, "redacted");
		assert_eq!(spans[1].reason_code.as_deref(), Some("WRITE_POLICY_REDACTION"));
		assert_eq!(spans[1].start_offset, 16);
		assert_eq!(spans[1].end_offset, 22);
	}
}
