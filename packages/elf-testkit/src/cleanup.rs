use std::{collections::HashSet, time::Duration};

use qdrant_client::Qdrant;
use sqlx::{
	AssertSqlSafe, Connection,
	postgres::{PgConnectOptions, PgConnection},
};
use tokio::time;

use crate::{Error, Result};

const ADMIN_DATABASES: [&str; 2] = ["postgres", "template1"];

pub(crate) async fn connect_admin(
	base_options: &PgConnectOptions,
) -> Result<(PgConnectOptions, PgConnection)> {
	let mut last_err = None;

	for database in ADMIN_DATABASES {
		let options = base_options.clone().database(database);

		match PgConnection::connect_with(&options).await {
			Ok(conn) => return Ok((options, conn)),
			Err(err) => {
				last_err = Some(err);
			},
		}
	}

	Err(Error::Message(format!("Failed to connect to an admin database: {last_err:?}.")))
}

pub(crate) async fn cleanup_database(name: &str, admin_options: &PgConnectOptions) -> Result<()> {
	let conn = PgConnection::connect_with(admin_options).await.map_err(|err| {
		Error::Message(format!("Failed to connect to admin database for cleanup: {err}."))
	})?;
	let drop_sql = format!(r#"DROP DATABASE IF EXISTS "{}""#, name);
	let mut conn = conn;
	let _ = sqlx::query(
		"\
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE datname = $1 AND pid <> pg_backend_pid()",
	)
	.bind(name)
	.fetch_all(&mut conn)
	.await;

	sqlx::raw_sql(AssertSqlSafe(drop_sql))
		.execute(&mut conn)
		.await
		.map_err(|err| Error::Message(format!("Failed to drop test database: {err}.")))?;

	Ok(())
}

pub(crate) async fn cleanup_qdrant_collections(collections: &[String]) -> Result<()> {
	if collections.is_empty() {
		return Ok(());
	}

	let Some(qdrant_url) = crate::env_qdrant_url() else {
		eprintln!(
			"Skipping Qdrant cleanup; set ELF_QDRANT_GRPC_URL (or ELF_QDRANT_URL) to delete test collections."
		);

		return Ok(());
	};
	let client = Qdrant::from_url(&qdrant_url)
		.build()
		.map_err(|err| Error::Message(format!("Failed to build Qdrant client: {err}.")))?;
	let max_attempts = 6;
	let mut remaining = collections.iter().cloned().collect::<HashSet<_>>();
	let mut backoff = Duration::from_millis(100);

	for attempt in 1..=max_attempts {
		let existing = time::timeout(Duration::from_secs(10), client.list_collections())
			.await
			.map_err(|_| Error::Message("Qdrant list_collections timed out.".to_string()))?
			.map_err(|err| Error::Message(format!("Failed to list Qdrant collections: {err}.")))?;
		let existing = existing.collections.into_iter().map(|c| c.name).collect::<HashSet<_>>();

		remaining.retain(|collection| existing.contains(collection));

		if remaining.is_empty() {
			return Ok(());
		}

		for collection in remaining.iter().cloned().collect::<Vec<_>>() {
			let result = time::timeout(
				Duration::from_secs(10),
				client.delete_collection(collection.clone()),
			)
			.await;

			match result {
				Ok(Ok(_)) => {},
				Ok(Err(err)) =>
					if attempt == max_attempts {
						return Err(Error::Message(format!(
							"Failed to delete Qdrant collection {collection:?} after {attempt} attempts: {err}."
						)));
					},
				Err(_) =>
					if attempt == max_attempts {
						return Err(Error::Message(format!(
							"Timed out deleting Qdrant collection {collection:?} after {attempt} attempts."
						)));
					},
			}
		}

		time::sleep(backoff).await;

		backoff = backoff.saturating_mul(2).min(Duration::from_secs(2));
	}

	Ok(())
}
