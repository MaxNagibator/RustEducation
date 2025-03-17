use chrono::{DateTime, Utc};
use deadpool_postgres::{Manager, Pool, Runtime};
use std::str::FromStr;
use tokio_postgres::{NoTls, Row};

pub type PgPool = Pool;

pub fn create_pool(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
    let config = tokio_postgres::Config::from_str(database_url)?;
    let manager = Manager::new(config, NoTls);
    Ok(Pool::builder(manager).runtime(Runtime::Tokio1).build()?)
}

#[derive(Debug, Clone)]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Row> for User {
    fn from(row: Row) -> Self {
        User {
            user_id: row.get("user_id"),
            username: row.get("username"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            created_at: row.get("created_at"),
        }
    }
}

pub async fn insert_user(pool: &PgPool, user: &User) -> Result<(), Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    let stmt = client
        .prepare(
            "INSERT INTO users (user_id, username, first_name, last_name) 
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id) DO UPDATE SET
             username = EXCLUDED.username,
             first_name = EXCLUDED.first_name,
             last_name = EXCLUDED.last_name",
        )
        .await?;

    client
        .execute(
            &stmt,
            &[
                &user.user_id,
                &user.username,
                &user.first_name,
                &user.last_name,
            ],
        )
        .await?;

    Ok(())
}

pub async fn delete_user(pool: &PgPool, user_id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    let stmt = client
        .prepare("DELETE FROM users WHERE user_id = $1")
        .await?;

    client.execute(&stmt, &[&user_id]).await?;
    Ok(())
}

pub async fn exists_user(pool: &PgPool, user_id: i64) -> Result<bool, Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    let stmt = client
        .prepare("SELECT EXISTS(SELECT 1 FROM users WHERE user_id = $1)")
        .await?;

    let exists: bool = client.query_one(&stmt, &[&user_id]).await?.get(0);

    Ok(exists)
}

pub async fn get_user(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<User>, Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    let stmt = client
        .prepare("SELECT * FROM users WHERE user_id = $1")
        .await?;

    let row = client.query_opt(&stmt, &[&user_id]).await?;
    Ok(row.map(User::from))
}

pub async fn get_users(pool: &PgPool) -> Result<Vec<User>, Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    let stmt = client.prepare("SELECT * FROM users").await?;

    let rows = client.query(&stmt, &[]).await?;
    Ok(rows.into_iter().map(User::from).collect())
}
