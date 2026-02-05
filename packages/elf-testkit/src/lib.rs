use std::{env, future::Future, str::FromStr, thread};

use color_eyre::eyre::{self, WrapErr};
use sqlx::{
	ConnectOptions, Connection, Executor,
	postgres::{PgConnectOptions, PgConnection},
};
use tokio::runtime::Builder;
use uuid::Uuid;

const ADMIN_DATABASES: [&str; 2] = ["postgres", "template1"];

pub fn env_dsn() -> Option<String> {
	env::var("ELF_PG_DSN").ok()
}

pub struct TestDatabase {
	name: String,
	dsn: String,
	admin_options: PgConnectOptions,
	cleaned: bool,
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

		Ok(Self { name, dsn, admin_options, cleaned: false })
	}

	pub fn dsn(&self) -> &str {
		&self.dsn
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn collection_name(&self, prefix: &str) -> String {
		format!("{prefix}_{}", self.name)
	}

	pub async fn cleanup(mut self) -> color_eyre::Result<()> {
		self.cleanup_inner().await
	}

	async fn cleanup_inner(&mut self) -> color_eyre::Result<()> {
		if self.cleaned {
			return Ok(());
		}
		cleanup_database(&self.name, &self.admin_options).await?;
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
		let _ = thread::spawn(move || {
			let runtime = match Builder::new_current_thread().enable_all().build() {
				Ok(runtime) => runtime,
				Err(err) => {
					eprintln!("Test database cleanup failed: {err}.");
					return;
				},
			};
			if let Err(err) = runtime.block_on(cleanup_database(&name, &admin_options)) {
				eprintln!("Test database cleanup failed: {err}.");
			}
		});
	}
}

pub async fn with_test_db<F, Fut, T>(base_dsn: &str, f: F) -> color_eyre::Result<T>
where
	F: FnOnce(&TestDatabase) -> Fut,
	Fut: Future<Output = color_eyre::Result<T>>,
{
	let mut db = TestDatabase::new(base_dsn).await?;
	let result = f(&db).await;
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
	let mut conn = PgConnection::connect_with(admin_options)
		.await
		.wrap_err("Failed to connect to admin database for cleanup.")?;
	let _ = sqlx::query(
		"SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
		 WHERE datname = $1 AND pid <> pg_backend_pid()",
	)
	.bind(name)
	.execute(&mut conn)
	.await;
	let drop_sql = format!(r#"DROP DATABASE IF EXISTS "{}""#, name);
	sqlx::query(drop_sql.as_str())
		.execute(&mut conn)
		.await
		.wrap_err("Failed to drop test database.")?;
	Ok(())
}
