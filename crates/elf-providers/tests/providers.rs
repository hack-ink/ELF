#[test]
fn builds_bearer_auth_header() {
    let headers = elf_providers::auth_headers("secret", &serde_json::Map::new())
        .expect("Failed to build headers.");
    let value = headers
        .get(reqwest::header::AUTHORIZATION)
        .expect("Missing authorization header.");
    assert_eq!(value, "Bearer secret");
}
