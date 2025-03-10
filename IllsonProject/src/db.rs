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
pub async fn delete_user(pool: &PgPool, chat_id: i32) -> Result<(), Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    client
        .execute("DELETE FROM users WHERE chat_id = $1", &[&chat_id])
        .await?;
    Ok(())
}

#[derive(Debug)]
pub struct UserInfo {
    pub chat_id: i32,
    pub username: String,
    pub first_name: String,
}

pub async fn get_user(
    pool: &PgPool,
    chat_id: i32,
) -> Result<Option<UserInfo>, Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    let row = client
        .query_opt(
            "SELECT chat_id, name, first_name FROM users WHERE chat_id = $1",
            &[&chat_id],
        )
        .await?;

    Ok(row.map(|r| UserInfo {
        chat_id: r.get(0),
        username: r.get(1),
        first_name: r.get(2),
    }))
}
