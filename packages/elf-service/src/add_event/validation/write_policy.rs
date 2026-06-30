use crate::{
	Error, Result,
	add_event::types::{EventMessage, ProcessedEventOutput},
};
use elf_domain::writegate::{self, WritePolicyAudit, WritePolicyError};

pub(in crate::add_event) fn apply_write_policies_to_messages(
	messages: &[EventMessage],
) -> Result<ProcessedEventOutput> {
	let mut message_policy_applied = Vec::with_capacity(messages.len());
	let mut write_policy_audits = Vec::new();
	let mut transformed_messages = Vec::with_capacity(messages.len());

	for message in messages {
		let (transformed_message, audit) = apply_write_policy_to_message(message)?;

		message_policy_applied.push(audit.is_some());

		if let Some(audit) = audit {
			write_policy_audits.push(audit);
		}

		transformed_messages.push(transformed_message);
	}

	Ok((
		transformed_messages,
		message_policy_applied,
		if write_policy_audits.is_empty() { None } else { Some(write_policy_audits) },
	))
}

fn apply_write_policy_to_message(
	message: &EventMessage,
) -> Result<(EventMessage, Option<WritePolicyAudit>)> {
	let result =
		writegate::apply_write_policy(message.content.as_str(), message.write_policy.as_ref())
			.map_err(|err| {
				let message = match err {
					WritePolicyError::InvalidSpan => "Invalid write_policy span provided.",
					WritePolicyError::OverlappingOps => "Overlapping write_policy spans provided.",
				};

				Error::InvalidRequest { message: message.to_string() }
			})?;
	let has_policy = message.write_policy.is_some();
	let mut transformed = message.clone();

	transformed.content = result.transformed;

	Ok((transformed, if has_policy { Some(result.audit) } else { None }))
}
