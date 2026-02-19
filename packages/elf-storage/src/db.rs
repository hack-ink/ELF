use sqlx::{PgConnection, PgPool, Transaction, postgres::PgPoolOptions};

use crate::{Result, graph, schema};

pub struct Db {
	pub pool: PgPool,
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

		backfill_graph_fact_predicate_ids(&mut tx).await?;

		tx.commit().await?;

		Ok(())
	}
}

async fn backfill_graph_fact_predicate_ids(tx: &mut Transaction<'_, sqlx::Postgres>) -> Result<()> {
	loop {
		let conn: &mut PgConnection = &mut *tx;
		let rows: Vec<(String, String, String)> = sqlx::query_as(
			"\
SELECT DISTINCT tenant_id, project_id, predicate
FROM graph_facts
WHERE predicate_id IS NULL
LIMIT 200",
		)
		.fetch_all(conn)
		.await?;

		if rows.is_empty() {
			break;
		}

		for (tenant_id, project_id, predicate_surface) in rows {
			let conn: &mut PgConnection = &mut *tx;
			let predicate = graph::resolve_or_register_predicate(
				conn,
				tenant_id.as_str(),
				project_id.as_str(),
				predicate_surface.as_str(),
			)
			.await?;

			sqlx::query(
				"\
UPDATE graph_facts
SET predicate_id = $1
WHERE tenant_id = $2
	AND project_id = $3
	AND predicate = $4
	AND predicate_id IS NULL",
			)
			.bind(predicate.predicate_id)
			.bind(tenant_id.as_str())
			.bind(project_id.as_str())
			.bind(predicate_surface.as_str())
			.execute(conn)
			.await?;
		}
	}

	Ok(())
}
