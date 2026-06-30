use crate::{
	Result,
	structured_fields::{
		types::StructuredEntity,
		validation::{
			bounds::{self, MAX_ALIASES},
			text,
		},
	},
};

pub(super) fn validate_structured_entity(
	entity: &StructuredEntity,
	base: &str,
	require_canonical: bool,
) -> Result<()> {
	if require_canonical {
		text::validate_required_text_field(
			entity.canonical.as_ref(),
			&format!("{base}.canonical"),
		)?;
	}

	if let Some(kind) = entity.kind.as_ref() {
		text::validate_text_field(kind, &format!("{base}.kind"))?;
	}
	if let Some(aliases) = entity.aliases.as_ref() {
		bounds::validate_list_field_count(aliases.len(), MAX_ALIASES, &format!("{base}.aliases"))?;

		for (alias_idx, alias) in aliases.iter().enumerate() {
			text::validate_text_field(alias, &format!("{base}.aliases[{alias_idx}]"))?;
		}
	}

	Ok(())
}
