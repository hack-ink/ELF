use crate::env;

pub(crate) fn env_string(names: &[&str]) -> Option<String> {
	names.iter().find_map(|name| {
		env::var(name).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
	})
}

pub(super) fn apply_env_string(target: &mut String, names: &[&str]) {
	if let Some(value) = env_string(names) {
		*target = value;
	}
}

pub(super) fn env_u32(names: &[&str]) -> Option<u32> {
	env_string(names).and_then(|value| value.parse::<u32>().ok())
}

pub(super) fn env_u64(names: &[&str]) -> Option<u64> {
	env_string(names).and_then(|value| value.parse::<u64>().ok())
}
