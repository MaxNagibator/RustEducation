use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};
use teloxide::{prelude::*, types::User};
use tracing::{debug, error, info};
pub type Error = Box<dyn std::error::Error + Send + Sync>;

mod config;
mod db;

use crate::config::Config;
use crate::db::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(true)
        .with_timer(tracing_subscriber::fmt::time::time())
        .init();

    dotenv().ok();
    info!("Начало инициализации приложения");

    let key = "TELOXIDE_TOKEN";
    if env::var(key).is_err() {
        let file_path = "E:\\bobgroup\\repo\\TelegramPomogatorBot\\token.txt";
        debug!("Загрузка токена из файла: {}", file_path);
        //file_path = "E:\\bobgroup\\projects\\Rust\\TestFile.txt";
        let token = fs::read_to_string(file_path)
            .inspect_err(|e| error!("Ошибка чтения файла с токеном: {}", e))?;
        unsafe {
            env::set_var(key, token.trim());
        }
    }

    let config =
        Config::from_env().inspect_err(|e| error!("Ошибка загрузки конфигурации: {}", e))?;
    debug!("Конфигурация успешно загружена");

    let pool = db::create_pool(&config.database_url)
        .inspect_err(|e| error!("Ошибка создания пула БД: {}", e))?;
    let pool = Arc::new(pool);
    info!("Пул соединений с БД создан");

    run_bot(pool.clone())
        .await
        .inspect_err(|e| error!("Ошибка в работе бота: {}", e))?;
    run_server(pool.clone(), config.server_address.clone())
        .await
        .inspect_err(|e| error!("Ошибка в работе сервера: {}", e))?;

    info!("Приложение завершило работу");
    Ok(())
}

async fn run_bot(pool: Arc<PgPool>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Инициализация бота");

    let bot = Bot::from_env();
    let schema = Update::filter_message()
        .filter_map(|update: Update| update.from().cloned())
        .endpoint(process_message);

    Dispatcher::builder(bot, schema)
        .dependencies(dptree::deps![pool])
        .build()
        .dispatch()
        .await;

    info!("Бот остановлен");
    Ok(())
}

async fn run_server(pool: Arc<PgPool>, addr: String) -> Result<(), Box<dyn std::error::Error>> {
    info!("Запуск сервера на {}", addr);

    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user))
        .with_state(pool)
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(
                    tower_http::trace::DefaultMakeSpan::new()
                        .level(tracing::Level::INFO)
                        .include_headers(true),
                )
                .on_response(
                    tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO),
                ),
        );

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .inspect_err(|e| error!("Ошибка привязки к {}: {}", addr, e))?;

    info!("Сервер успешно запущен");
    axum::serve(listener, app)
        .await
        .inspect_err(|e| error!("Ошибка сервера: {}", e))?;

    info!("Сервер остановлен");
    Ok(())
}

async fn process_message(
    bot: Bot,
    user: User,
    pool: Arc<PgPool>,
    msg: Message,
) -> Result<(), Error> {
    info!("Обработка сообщения: {:?}", msg);

    if let Some(text) = msg.text() {
        let command = text.trim().to_lowercase();
        debug!("Получена команда: '{}'", command);

        match command.as_str() {
            "join" => {
                let username = user.username.unwrap_or_default();
                let first_name = user.first_name;

                db::insert_user(&pool, user.id.0 as i32, username, first_name.clone())
                    .await
                    .inspect_err(|e| error!("Ошибка БД: {}", e))
                    .unwrap();

                bot.send_message(user.id, format!("Добро пожаловать, {}! 🎉", first_name))
                    .await
                    .inspect_err(|e| error!("Ошибка отправки сообщения: {}", e))
                    .unwrap();
            }
            "me" => {
                if let Some(user1) = db::get_user(&pool, user.id.0 as i32).await.unwrap() {
                    bot.send_message(
                        user.id,
                        format!(
                            "Ваш профиль:\nID: {}\nUsername: @{}\nИмя: {}",
                            user1.chat_id, user1.username, user1.first_name
                        ),
                    )
                    .await?;
                } else {
                    bot.send_message(
                        user.id,
                        "Малыш, команда только для членов общества. Напиши 'join'",
                    )
                    .await?;
                }
            }
            _ => {
                if let Some(user1) = db::get_user(&pool, user.id.0 as i32).await.unwrap() {
                    bot.send_message(
                        user.id,
                        format!("Привет, {}! Чем могу помочь?", user1.first_name),
                    )
                    .await?;
                } else {
                    bot.send_message(user.id, "Привет, малыш! Напиши 'join' для пополнения рядов")
                        .await?;
                }
            }
        }
    } else {
        bot.send_message(user.id, "Пожалуйста, отправляй текстовые сообщения")
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

    futures::future::join_all(tasks).await;
    "2!"
}

async fn sleep_time_complex(name: u64, secs: u64) {
    println!("bla {} start", name);
    sleep_time(secs).await;
    println!("bla {name} finish {secs}",);
}

async fn create_user(Json(payload): Json<CreateUser>) -> (StatusCode, Json<User2>) {
    let user = User2 {
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
struct User2 {
    id: u64,
    username: String,
}
