use crate::config::Config;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, Postgres};
use sqlx::Pool;

pub async fn create_pool(config: &Config) -> Result<Pool<Postgres>, sqlx::Error> {
    let pool_options = PgPoolOptions::new().max_connections(5);
    let pool = if let Some(ref database_url) = config.database_location {
        pool_options.connect(database_url).await?
    } else {
        let options = PgConnectOptions::new();
        pool_options.connect_with(options).await?
    };

    Ok(pool)
}
