use crate::config::Config;
use sqlx::postgres::{PgPoolOptions, Postgres};
use sqlx::Pool;

pub async fn create_pool(config: &Config) -> Result<Pool<Postgres>, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(config.database_location.as_ref().unwrap())
        .await?;
    Ok(pool)
}
