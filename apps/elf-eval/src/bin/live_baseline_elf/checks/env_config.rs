use crate::env;

pub(crate) fn parse_env_u64(name: &str) -> Option<u64> {
	env::var(name).ok()?.parse::<u64>().ok()
}

pub(crate) fn parse_env_usize(name: &str) -> Option<usize> {
	env::var(name).ok()?.parse::<usize>().ok()
}
