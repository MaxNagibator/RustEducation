use deadpool_postgres::{Manager, Pool, Runtime};
use std::str::FromStr;
use tokio_postgres::NoTls;

pub type PgPool = Pool;

pub fn create_pool(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
    let config = tokio_postgres::Config::from_str(database_url)?;
    let manager = Manager::new(config, NoTls);
    Ok(Pool::builder(manager).runtime(Runtime::Tokio1).build()?)
}

pub async fn insert_user(
    pool: &PgPool,
    chat_id: i32,
    username: String,
    first_name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    client
        .execute(
            "INSERT INTO users (chat_id, name, first_name) VALUES ($1, $2, $3) 
             ON CONFLICT (chat_id) DO NOTHING",
            &[&chat_id, &username, &first_name],
        )
        .await?;
    Ok(())
}
