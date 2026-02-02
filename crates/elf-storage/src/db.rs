use color_eyre::Result;

pub struct Db {
	pub pool: sqlx::PgPool,
}

impl Db {
	pub async fn connect(cfg: &elf_config::Postgres) -> Result<Self> {
		let pool = sqlx::postgres::PgPoolOptions::new()
			.max_connections(cfg.pool_max_conns)
			.connect(&cfg.dsn)
			.await?;
		Ok(Self { pool })
	}

	pub async fn ensure_schema(&self, vector_dim: u32) -> Result<()> {
		let sql = crate::schema::render_schema(vector_dim);
		let lock_id: i64 = 7_120_114;
		sqlx::query("SELECT pg_advisory_lock($1)").bind(lock_id).execute(&self.pool).await?;

		let mut failure: Option<color_eyre::Report> = None;
		for statement in sql.split(';') {
			let trimmed = statement.trim();
			if trimmed.is_empty() {
				continue;
			}
			if let Err(err) = sqlx::query(trimmed).execute(&self.pool).await {
				failure = Some(err.into());
				break;
			}
		}
		let _ =
			sqlx::query("SELECT pg_advisory_unlock($1)").bind(lock_id).execute(&self.pool).await;
		if let Some(err) = failure {
			return Err(err);
		}
		Ok(())
	}
}
