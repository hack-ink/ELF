use crate::{Error, Result};

pub(super) const MAX_LIST_ITEMS: usize = 64;
pub(super) const MAX_ENTITIES: usize = 32;
pub(super) const MAX_RELATIONS: usize = 64;
pub(super) const MAX_ALIASES: usize = 16;

pub(super) fn validate_list_field(items: &[String], label: &str) -> Result<()> {
	if items.len() > MAX_LIST_ITEMS {
		return Err(Error::InvalidRequest {
			message: format!("{label} must have at most {MAX_LIST_ITEMS} items."),
		});
	}

	Ok(())
}

pub(super) fn validate_list_field_count(len: usize, max: usize, label: &str) -> Result<()> {
	if len > max {
		return Err(Error::InvalidRequest {
			message: format!("{label} must have at most {max} items."),
		});
	}

	Ok(())
}
