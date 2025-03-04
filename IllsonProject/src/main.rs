use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{env, fs};
use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    let file_path = "E:\\bobgroup\\repo\\TelegramPomogatorBot\\token.txt";
    //file_path = "E:\\bobgroup\\projects\\Rust\\TestFile.txt";
    //let test =  fs::read_to_string(file_path).unwrap();
    let test = fs::read_to_string(file_path).unwrap();
    let key = "TELOXIDE_TOKEN";
    unsafe {
        env::set_var(key, test);
    }
    let bot = Bot::from_env();

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        bot.send_dice(msg.chat.id).await?;
        match msg.text() {
            Some(text) => {
                println!("{}", text);
                bot.send_message(msg.chat.id, "privet malish?").await?;
                //dialogue.update(State::ReceiveAge { full_name: text.into() }).await?;
            }
            None => {
                bot.send_message(msg.chat.id, "Send me plain text.").await?;
            }
        }

        Ok(())
    })
    .await;

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user));

    let listener = tokio::net::TcpListener::bind("localhost:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    println!("running on port 3000");
}

async fn root() -> &'static str {
    let mut tasks = Vec::new();
    for num in 1..4 {
        let task = sleep_time_complex(num, num);
        tasks.push(task);
        let task = sleep_time_complex(num, 4 - num);
        tasks.push(task);
    }
    join_all(tasks).await;
    //tokio::join!(tasks);

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
