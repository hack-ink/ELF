use std::collections::HashSet;

use crate::{Config, Error, Result};

pub(super) fn validate(cfg: &Config) -> Result<()> {
	let mut seen_rules = HashSet::new();

	for (idx, rule) in cfg.memory.policy.rules.iter().enumerate() {
		let path = format!("memory.policy.rules[{idx}]");

		if let Some(note_type) = rule.note_type.as_ref() {
			if note_type.trim().is_empty() {
				return Err(Error::Validation {
					message: format!("{path}.note_type cannot be blank or whitespace-only."),
				});
			}
			if !matches!(
				note_type.as_str(),
				"preference" | "constraint" | "decision" | "profile" | "fact" | "plan"
			) {
				return Err(Error::Validation {
					message: format!(
						"{path}.note_type must be one of preference, constraint, decision, profile, fact, or plan."
					),
				});
			}
		}
		if let Some(scope) = rule.scope.as_ref() {
			if scope.trim().is_empty() {
				return Err(Error::Validation {
					message: format!("{path}.scope cannot be blank or whitespace-only."),
				});
			}
			if !cfg.scopes.allowed.iter().any(|allowed_scope| allowed_scope == scope) {
				return Err(Error::Validation {
					message: format!("{path}.scope must be one of allowed scopes."),
				});
			}
		}
		if let Some(min_confidence) = rule.min_confidence {
			if !min_confidence.is_finite() {
				return Err(Error::Validation {
					message: format!("{path}.min_confidence must be a finite number."),
				});
			}
			if !(0.0..=1.0).contains(&min_confidence) {
				return Err(Error::Validation {
					message: format!("{path}.min_confidence must be between 0.0 and 1.0."),
				});
			}
		}
		if let Some(min_importance) = rule.min_importance {
			if !min_importance.is_finite() {
				return Err(Error::Validation {
					message: format!("{path}.min_importance must be a finite number."),
				});
			}
			if !(0.0..=1.0).contains(&min_importance) {
				return Err(Error::Validation {
					message: format!("{path}.min_importance must be between 0.0 and 1.0."),
				});
			}
		}

		let rule_key = (rule.note_type.clone(), rule.scope.clone());

		if !seen_rules.insert(rule_key) {
			return Err(Error::Validation {
				message: format!("{path} has a duplicate note_type and scope pair."),
			});
		}
	}

	Ok(())
}
