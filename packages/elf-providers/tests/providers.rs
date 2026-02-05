use reqwest::header::AUTHORIZATION;
use serde_json::Map;

#[test]
fn builds_bearer_auth_header() {
	let headers =
		elf_providers::auth_headers("secret", &Map::new()).expect("Failed to build headers.");
	let value = headers.get(AUTHORIZATION).expect("Missing authorization header.");
	assert_eq!(value, "Bearer secret");
}
