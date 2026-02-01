#[tokio::test]
async fn outbox_retries_to_done() {
    if std::env::var("ELF_TEST_PG_DSN").is_err() || std::env::var("ELF_TEST_QDRANT_URL").is_err()
    {
        eprintln!("Skipping outbox_retries_to_done; requires ELF_TEST_PG_DSN and ELF_TEST_QDRANT_URL.");
        return;
    }
    // TODO: Add an integration test that simulates embedding provider failure and verifies
    // outbox retries to DONE once the provider recovers. This requires a controllable
    // embedder endpoint or a mock worker harness.
}
