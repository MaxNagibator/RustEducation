use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{env, fs};
use teloxide::prelude::*;
use tracing::{error, info};

mod config;
mod db;

use crate::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    dotenv().ok();

    info!("Starting application");

    let key = "TELOXIDE_TOKEN";
    if env::var(key).is_err() {
        let file_path = "E:\\bobgroup\\repo\\TelegramPomogatorBot\\token.txt";
        //file_path = "E:\\bobgroup\\projects\\Rust\\TestFile.txt";
        let test = fs::read_to_string(file_path).expect("Something went wrong reading the file");

        unsafe {
            env::set_var(key, test);
        }
    }

    let bot = Bot::from_env();
    let bot_task = teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        let config = Config::from_env().unwrap();
        let pool = db::create_pool(&config.database_url).unwrap();

        info!("Received message: {:?}", msg);
        bot.send_dice(msg.chat.id).await?;

        if let Some(text) = msg.text() {
            if text == "join" {
                if let Some(from) = msg.from {
                    let username = from.username.unwrap_or_default();
                    let first_name = from.first_name;

                    db::insert_user(&pool, msg.chat.id.0 as i32, username, first_name)
                        .await
                        .map_err(|e| {
                            error!("Database error: {}", e);
                            e
                        })
                        .unwrap();
                }
            }
            bot.send_message(msg.chat.id, "privet malish?").await?;
        } else {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }

        Ok(())
    });

    let server_task = async {
        let config = Config::from_env()?;
        let pool = db::create_pool(&config.database_url)?;

        let app = Router::new()
            .route("/", get(root))
            .route("/users", post(create_user))
            .with_state(pool);

        let server_addr = config.server_address.clone();
        let listener = tokio::net::TcpListener::bind(&server_addr).await?;
        info!("Server running on {}", server_addr);

        axum::serve(listener, app).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    tokio::select! {
        result = bot_task => result,
        result = server_task => result?,
    }

    Ok(())
}

async fn root() -> &'static str {
    let mut tasks = Vec::new();
    for num in 1..4 {
        let task = sleep_time_complex(num, num);
        tasks.push(task);
        let task = sleep_time_complex(num, 4 - num);
        tasks.push(task);
    }

    futures::future::join_all(tasks).await;
    "2!"
}

async fn sleep_time_complex(name: u64, secs: u64) {
    println!("bla {} start", name);
    sleep_time(secs).await;
    println!("bla {name} finish {secs}",);
}

async fn create_user(Json(payload): Json<CreateUser>) -> (StatusCode, Json<User>) {
    let user = User {
        id: 6062171111111,
        username: payload.username,
    };

    (StatusCode::CREATED, Json(user))
}

async fn sleep_time(seconds: u64) {
    tokio::time::sleep(Duration::from_secs(seconds)).await;
}

#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
