use std::{collections::HashSet, str::FromStr, sync::Mutex, thread};

use sqlx::{AssertSqlSafe, ConnectOptions, postgres::PgConnectOptions};
use tokio::runtime::Builder;
use uuid::Uuid;

use crate::{Error, Result, cleanup};

/// Ephemeral test database handle with tracked Qdrant collections for cleanup.
pub struct TestDatabase {
	name: String,
	dsn: String,
	admin_options: PgConnectOptions,
	cleaned: bool,
	collections: Mutex<HashSet<String>>,
}
impl TestDatabase {
	/// Creates a fresh temporary Postgres database from a base admin DSN.
	pub async fn new(base_dsn: &str) -> Result<Self> {
		let base_options: PgConnectOptions = PgConnectOptions::from_str(base_dsn)
			.map_err(|err| Error::Message(format!("Failed to parse ELF_PG_DSN: {err}.")))?;
		let (admin_options, mut admin_conn) = cleanup::connect_admin(&base_options).await?;
		let name = format!("elf_test_{}", Uuid::new_v4().simple());
		let create_sql = format!(r#"CREATE DATABASE "{}""#, name);

		sqlx::raw_sql(AssertSqlSafe(create_sql))
			.execute(&mut admin_conn)
			.await
			.map_err(|err| Error::Message(format!("Failed to create test database: {err}.")))?;

		let dsn = base_options.clone().database(&name).to_url_lossy().to_string();

		Ok(Self {
			name,
			dsn,
			admin_options,
			cleaned: false,
			collections: Mutex::new(HashSet::new()),
		})
	}

	/// Returns the DSN for the temporary test database.
	pub fn dsn(&self) -> &str {
		&self.dsn
	}

	/// Returns the generated database name.
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Returns a unique collection prefix and tracks the related Qdrant collections.
	pub fn collection_name(&self, prefix: &str) -> String {
		let collection = format!("{prefix}_{}", self.name);
		let docs_collection = format!("{collection}_docs");
		let mut tracked = self.collections.lock().unwrap_or_else(|err| err.into_inner());

		tracked.insert(collection.clone());
		tracked.insert(docs_collection);

		collection
	}

	/// Drops the temporary database and any tracked Qdrant collections.
	pub async fn cleanup(mut self) -> Result<()> {
		self.cleanup_inner().await
	}

	pub(crate) async fn cleanup_inner(&mut self) -> Result<()> {
		if self.cleaned {
			return Ok(());
		}

		let collections = {
			let tracked = self.collections.lock().unwrap_or_else(|err| err.into_inner());

			tracked.iter().cloned().collect::<Vec<_>>()
		};
		let db_result = cleanup::cleanup_database(&self.name, &self.admin_options).await;
		let qdrant_result = cleanup::cleanup_qdrant_collections(&collections).await;

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

			if let Err(err) = runtime.block_on(cleanup::cleanup_qdrant_collections(&collections)) {
				eprintln!("Test Qdrant cleanup failed: {err}.");
			}
			if let Err(err) = runtime.block_on(cleanup::cleanup_database(&name, &admin_options)) {
				eprintln!("Test database cleanup failed: {err}.");
			}
		});
		let _ = cleanup_thread.join();
	}
}
