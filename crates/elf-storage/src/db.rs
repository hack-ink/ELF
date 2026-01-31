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
        sqlx::query(&sql).execute(&self.pool).await?;
        Ok(())
    }
}
