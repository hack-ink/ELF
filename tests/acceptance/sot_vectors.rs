#[tokio::test]
async fn active_notes_have_vectors() {
    if std::env::var("ELF_TEST_PG_DSN").is_err() || std::env::var("ELF_TEST_QDRANT_URL").is_err()
    {
        eprintln!("Skipping active_notes_have_vectors; requires ELF_TEST_PG_DSN and ELF_TEST_QDRANT_URL.");
        return;
    }
    // TODO: Add an integration test that writes a note, runs the outbox worker, and
    // asserts a note_embeddings row exists with the configured vector_dim.
}
