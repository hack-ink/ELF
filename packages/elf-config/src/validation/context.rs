use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	if let Some(context) = cfg.context.as_ref()
		&& let Some(weight) = context.scope_boost_weight
	{
		if !weight.is_finite() {
			return Err(Error::Validation {
				message: "context.scope_boost_weight must be a finite number.".to_string(),
			});
		}
		if weight < 0.0 {
			return Err(Error::Validation {
				message: "context.scope_boost_weight must be zero or greater.".to_string(),
			});
		}
		if weight > 1.0 {
			return Err(Error::Validation {
				message: "context.scope_boost_weight must be 1.0 or less.".to_string(),
			});
		}
		if weight > 0.0
			&& context
				.scope_descriptions
				.as_ref()
				.map(|descriptions| descriptions.is_empty())
				.unwrap_or(true)
		{
			return Err(Error::Validation {
				message: "context.scope_descriptions must be non-empty when context.scope_boost_weight is greater than zero."
					.to_string(),
			});
		}
	}

	Ok(())
}
