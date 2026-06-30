use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

pub(super) fn hash_query(query: &str) -> String {
	let mut hasher = DefaultHasher::new();

	Hash::hash(query, &mut hasher);

	format!("{:x}", hasher.finish())
}
