use std::{
	collections::HashSet, env, future::Future, str::FromStr, sync::Mutex, thread, time::Duration,
};

use color_eyre::eyre::{self, WrapErr};
use qdrant_client::Qdrant;
use sqlx::{
	ConnectOptions, Connection, Executor,
	postgres::{PgConnectOptions, PgConnection},
};
use tokio::{runtime::Builder, time};
use uuid::Uuid;

const ADMIN_DATABASES: [&str; 2] = ["postgres", "template1"];

pub struct TestDatabase {
	name: String,
	dsn: String,
	admin_options: PgConnectOptions,
	cleaned: bool,
	collections: Mutex<HashSet<String>>,
}
impl TestDatabase {
	pub async fn new(base_dsn: &str) -> color_eyre::Result<Self> {
		let base_options: PgConnectOptions =
			PgConnectOptions::from_str(base_dsn).wrap_err("Failed to parse ELF_PG_DSN.")?;
		let (admin_options, mut admin_conn) = connect_admin(&base_options).await?;
		let name = format!("elf_test_{}", Uuid::new_v4().simple());
		let create_sql = format!(r#"CREATE DATABASE "{}""#, name);

		admin_conn
			.execute(create_sql.as_str())
			.await
			.wrap_err("Failed to create test database.")?;

		let dsn = base_options.clone().database(&name).to_url_lossy().to_string();

		Ok(Self {
			name,
			dsn,
			admin_options,
			cleaned: false,
			collections: Mutex::new(HashSet::new()),
		})
	}

	pub fn dsn(&self) -> &str {
		&self.dsn
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn collection_name(&self, prefix: &str) -> String {
		let collection = format!("{prefix}_{}", self.name);

		let mut tracked = self.collections.lock().unwrap_or_else(|err| err.into_inner());
		tracked.insert(collection.clone());

		collection
	}

	pub async fn cleanup(mut self) -> color_eyre::Result<()> {
		self.cleanup_inner().await
	}

	async fn cleanup_inner(&mut self) -> color_eyre::Result<()> {
		if self.cleaned {
			return Ok(());
		}

		let collections = {
			let tracked = self.collections.lock().unwrap_or_else(|err| err.into_inner());

			tracked.iter().cloned().collect::<Vec<_>>()
		};
		let db_result = cleanup_database(&self.name, &self.admin_options).await;
		let qdrant_result = cleanup_qdrant_collections(&collections).await;

		db_result?;
		qdrant_result?;

		self.cleaned = true;

		Ok(())
	}
}
impl Drop for TestDatabase {
	fn drop(&mut self) {
		if self.cleaned {
			return;
		}

		let name = self.name.clone();
		let admin_options = self.admin_options.clone();
		let collections = self
			.collections
			.lock()
			.unwrap_or_else(|err| err.into_inner())
			.iter()
			.cloned()
			.collect::<Vec<_>>();
		let cleanup_thread = thread::spawn(move || {
			let runtime = match Builder::new_current_thread().enable_all().build() {
				Ok(runtime) => runtime,
				Err(err) => {
					eprintln!("Test database cleanup failed: {err}.");

					return;
				},
			};

			if let Err(err) = runtime.block_on(cleanup_qdrant_collections(&collections)) {
				eprintln!("Test Qdrant cleanup failed: {err}.");
			}
			if let Err(err) = runtime.block_on(cleanup_database(&name, &admin_options)) {
				eprintln!("Test database cleanup failed: {err}.");
			}
		});
		let _ = cleanup_thread.join();
	}
}

pub fn env_dsn() -> Option<String> {
	env::var("ELF_PG_DSN").ok()
}

pub fn env_qdrant_url() -> Option<String> {
	env::var("ELF_QDRANT_URL").ok()
}

pub async fn with_test_db<F, Fut, T>(base_dsn: &str, f: F) -> color_eyre::Result<T>
where
	F: FnOnce(&TestDatabase) -> Fut,
	Fut: Future<Output = color_eyre::Result<T>>,
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

async fn connect_admin(
	base_options: &PgConnectOptions,
) -> color_eyre::Result<(PgConnectOptions, PgConnection)> {
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

	Err(eyre::eyre!("Failed to connect to an admin database: {:?}", last_err))
}

async fn cleanup_database(name: &str, admin_options: &PgConnectOptions) -> color_eyre::Result<()> {
	let conn = PgConnection::connect_with(admin_options)
		.await
		.wrap_err("Failed to connect to admin database for cleanup.")?;
	let drop_sql = format!(r#"DROP DATABASE IF EXISTS "{}""#, name);

	let mut conn = conn;

	let _ = sqlx::query!(
		"\
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE datname = $1 AND pid <> pg_backend_pid()",
		name,
	)
	.fetch_all(&mut conn)
	.await;

	sqlx::query(drop_sql.as_str())
		.execute(&mut conn)
		.await
		.wrap_err("Failed to drop test database.")?;

	Ok(())
}

async fn cleanup_qdrant_collections(collections: &[String]) -> color_eyre::Result<()> {
	if collections.is_empty() {
		return Ok(());
	}

	let Some(qdrant_url) = env_qdrant_url() else {
		eprintln!("Skipping Qdrant cleanup; set ELF_QDRANT_URL to delete test collections.");

		return Ok(());
	};
	let client =
		Qdrant::from_url(&qdrant_url).build().wrap_err("Failed to build Qdrant client.")?;
	let max_attempts = 6;

	let mut remaining = collections.iter().cloned().collect::<HashSet<_>>();
	let mut backoff = Duration::from_millis(100);

	for attempt in 1..=max_attempts {
		let existing = time::timeout(Duration::from_secs(10), client.list_collections())
			.await
			.wrap_err("Qdrant list_collections timed out.")?
			.wrap_err("Failed to list Qdrant collections.")?;
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
						return Err(err).wrap_err_with(|| {
							format!(
								"Failed to delete Qdrant collection {collection:?} after {attempt} attempts."
							)
						});
					},
				Err(_) =>
					if attempt == max_attempts {
						return Err(eyre::eyre!(
							"Timed out deleting Qdrant collection {collection:?} after {attempt} attempts."
						));
					},
			}
		}

		time::sleep(backoff).await;

		backoff = backoff.saturating_mul(2).min(Duration::from_secs(2));
	}

	Ok(())
}
