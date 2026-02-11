use sqlx::postgres::PgPoolOptions;

use crate::{Result, schema};

pub struct Db {
	pub pool: sqlx::PgPool,
}

impl Db {
	pub async fn connect(cfg: &elf_config::Postgres) -> Result<Self> {
		let pool =
			PgPoolOptions::new().max_connections(cfg.pool_max_conns).connect(&cfg.dsn).await?;

		Ok(Self { pool })
	}

	pub async fn ensure_schema(&self, vector_dim: u32) -> Result<()> {
		let sql = schema::render_schema(vector_dim);
		let lock_id: i64 = 7_120_114;
		// Advisory locks are held per connection. Use a single transaction so the lock is scoped to
		// one connection and automatically released when the transaction ends.
		let mut tx = self.pool.begin().await?;

		sqlx::query!("SELECT pg_advisory_xact_lock($1)", lock_id).execute(&mut *tx).await?;

		for statement in sql.split(';') {
			let trimmed = statement.trim();

			if trimmed.is_empty() {
				continue;
			}

			sqlx::query(trimmed).execute(&mut *tx).await?;
		}

		tx.commit().await?;

		Ok(())
	}
}
