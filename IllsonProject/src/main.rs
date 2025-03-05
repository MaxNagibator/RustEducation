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
use tokio_postgres::{Error, NoTls};

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

    println!("start");
    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        println!("Telegram bot: {:?}", msg);
        bot.send_dice(msg.chat.id).await?;
        match msg.text() {
            Some(text) => {
                // println!("{}", text);
                if text == "join" {
                    let from = msg.from.unwrap();
                    let username = from.username.unwrap();
                    let first_name = from.first_name;
                    insert_user(msg.chat.id.0 as i32, username, first_name)
                        .await
                        .unwrap();
                }
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

    println!("start api");
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

async fn insert_user(chat_id: i32, username: String, first_name: String) -> Result<(), Error> {
    /*let mut client = Client::connect(
        "postgresql://postgres:RjirfLeyz@localhost:5432/rust-dev",
        NoTls,
    )
    .await?;*/

    let (client, connection) = tokio_postgres::connect(
        "postgresql://postgres:RjirfLeyz@localhost:5432/rust-dev",
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let mut is_found = false;

    for row in client
        .query(
            "SELECT name, first_name FROM users WHERE chat_id = $1;",
            &[&chat_id],
        )
        .await?
    {
        is_found = true;
        let username2: &str = row.get(0);
        let first_name2: &str = row.get(1);
        println!("username exists: {} {}", username2, first_name2);
    }

    if !is_found {
        client
            .query(
                "INSERT INTO users (chat_id, name, first_name) VALUES ($1, $2, $3);",
                &[&chat_id, &username, &first_name],
            )
            .await?;
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
