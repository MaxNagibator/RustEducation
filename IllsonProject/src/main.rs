use dotenvy::dotenv;
use std::error::Error;
use std::sync::Arc;
use std::{env, fs};
use tracing::{debug, error, info};
mod api;
mod bot;
mod config;
mod db;

use crate::api::run_server;
use crate::bot::run_bot;
use crate::config::Config;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
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

    let pool = Arc::new(
        db::create_pool(&config.database_url)
            .inspect_err(|e| error!("Ошибка создания пула БД: {}", e))
            .unwrap(),
    );
    info!("Пул соединений с БД создан");

    crossbeam::scope(|s| {
        let server_pool = Arc::clone(&pool);
        let server_address = config.server_address.clone();
        let server_handle = s.spawn(|_| {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(run_server(server_pool, server_address))
        });

        let bot_handle = s.spawn(|_| {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(run_bot(Arc::clone(&pool)))
        });

        let _ = server_handle.join().unwrap();
        let _ = bot_handle.join().unwrap();
    })
    .unwrap();

    info!("Приложение завершило работу");
    Ok(())
}
