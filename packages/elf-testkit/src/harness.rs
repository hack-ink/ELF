use std::future::Future;

use crate::{Result, TestDatabase};

/// Runs an async test closure with a temporary database and guaranteed cleanup.
pub async fn with_test_db<F, Fut, T>(base_dsn: &str, f: F) -> Result<T>
where
	F: FnOnce(&TestDatabase) -> Fut,
	Fut: Future<Output = Result<T>>,
{
	let db = TestDatabase::new(base_dsn).await?;
	let result = f(&db).await;
	let mut db = db;

	if let Err(err) = db.cleanup_inner().await {
		eprintln!("Test database cleanup warning: {err}.");

		if result.is_ok() {
			return Err(err);
		}
	}

	result
}
