use deadpool_postgres::Pool;
use dotenvy::dotenv;
use std::error::Error;
use std::sync::Arc;
use std::{env, fs};
use teloxide::types::{InlineKeyboardButtonKind, User};
use teloxide::{
    payloads::SendMessageSetters,
    prelude::*,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup, InlineQueryResultArticle, InputMessageContent,
        InputMessageContentText, Me,
    },
    utils::command::BotCommands,
};
use tracing::{debug, error, info};
mod api;
mod config;
mod db;

use crate::api::run_server;
use crate::config::Config;
use crate::db::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
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
        .inspect_err(|e| error!("Ошибка создания пула БД: {}", e))
        .unwrap();

    let pool = Arc::new(pool);
    info!("Пул соединений с БД создан");

    let bot_handle = tokio::spawn(run_bot(pool.clone()));
    let server_handle = tokio::spawn(run_server(pool.clone(), config.server_address.clone()));

    let (bot_result, server_result) = tokio::try_join!(bot_handle, server_handle)?;

    if let Err(e) = bot_result {
        error!("Ошибка в задаче бота: {}", e);
    }

    if let Err(e) = server_result {
        error!("Ошибка в задаче сервера: {}", e);
    }

    info!("Приложение завершило работу");
    Ok(())
}

async fn run_bot(pool: Arc<Pool>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_map(|update: Update| update.from().cloned())
                .endpoint(process_message),
        )
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_inline_query().endpoint(inline_query_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![pool.clone()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Бот остановлен");
    Ok(())
}

#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Справка
    #[command(aliases = ["h", "?"])]
    Help,
    /// Start
    Start,
    /// Присоединиться
    Join,
    /// Отсоединиться
    Leave,
    /// Сведения об пользователе
    Me,
}

fn make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let debian_versions = [
        "Buzz", "Rex", "Bo", "Hamm", "Slink", "Potato", "Woody", "Sarge", "Etch", "Lenny",
        "Squeeze", "Wheezy", "Jessie", "Stretch", "Buster", "Bullseye",
    ];

    for versions in debian_versions.chunks(3) {
        let row = versions
            .iter()
            .map(|&version| InlineKeyboardButton::callback(version.to_owned(), version.to_owned()))
            .collect();

        keyboard.push(row);
    }

    InlineKeyboardMarkup::new(keyboard)
}

async fn process_message(
    bot: Bot,
    user: User,
    msg: Message,
    pool: Arc<PgPool>,
    me: Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Обработка сообщения: {:?}", msg);

    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                let keyboard = make_keyboard();
                bot.send_message(msg.chat.id, "Debian versions:")
                    .reply_markup(keyboard)
                    .await?;
            }
            Ok(Command::Help) => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Ok(Command::Join) => {
                let username = &user.username.unwrap_or_default();
                let first_name = &user.first_name;

                db::insert_user(&pool, msg.chat.id.0 as i32, username, first_name)
                    .await
                    .inspect_err(|e| error!("Ошибка БД: {}", e))
                    .unwrap();

                bot.send_message(msg.chat.id, format!("Добро пожаловать, {}! 🎉", first_name))
                    .await
                    .inspect_err(|e| error!("Ошибка отправки сообщения: {}", e))?;
            }
            Ok(Command::Leave) => {
                let first_name = user.first_name;

                db::delete_user(&pool, msg.chat.id.0 as i32)
                    .await
                    .inspect_err(|e| error!("Ошибка БД: {}", e))
                    .unwrap();

                bot.send_message(
                    msg.chat.id,
                    format!("Приходите к нам ещё, {}! 🎉", first_name),
                )
                .await?;
            }
            Ok(Command::Me) => {
                if let Some(db_user) = db::get_user(&pool, msg.chat.id.0 as i32).await.unwrap() {
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "Ваш профиль:\nID: {}\nUsername: @{}\nИмя: {}",
                            db_user.chat_id, db_user.username, db_user.first_name
                        ),
                    )
                    .await?;

                    let keyboard = InlineKeyboardMarkup::new(vec![
                        vec![
                            InlineKeyboardButton::new(
                                "Кнопка 1",
                                InlineKeyboardButtonKind::CallbackData("opt1".to_string()),
                            ),
                            InlineKeyboardButton::new(
                                "Кнопка 2",
                                InlineKeyboardButtonKind::CallbackData("opt2".to_string()),
                            ),
                        ],
                        vec![InlineKeyboardButton::new(
                            "Длинная",
                            InlineKeyboardButtonKind::CallbackData("opt3".to_string()),
                        )],
                    ]);

                    bot.send_message(user.id, "Выберите опцию:")
                        .reply_markup(keyboard)
                        .await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        "Малыш, команда только для членов общества. Напиши 'join'",
                    )
                    .reply_markup(make_keyboard())
                    .await?;
                }
            }
            Err(_) => {
                if let Some(user1) = db::get_user(&pool, msg.chat.id.0 as i32).await.unwrap() {
                    bot.send_message(
                        msg.chat.id,
                        format!("Привет, {}! Чем могу помочь?", user1.first_name),
                    )
                    .await?;
                } else {
                    bot.send_message(msg.chat.id, "Привет! Нажми 'Join' чтобы присоединиться")
                        .await?;
                }
            }
        }
    }

    Ok(())
}
async fn inline_query_handler(
    bot: Bot,
    q: InlineQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let choose_debian_version = InlineQueryResultArticle::new(
        "0",
        "Chose debian version",
        InputMessageContent::Text(InputMessageContentText::new("Debian versions:")),
    )
    .reply_markup(make_keyboard());

    bot.answer_inline_query(q.id, vec![choose_debian_version.into()])
        .await?;

    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref version) = q.data {
        let text = format!("You chose: {version}");

        bot.answer_callback_query(&q.id).await?;

        if let Some(message) = q.regular_message() {
            bot.edit_message_text(message.chat.id, message.id, text)
                .await?;
        } else if let Some(id) = q.inline_message_id {
            bot.edit_message_text_inline(id, text).await?;
        }

        info!("You chose: {}", version);
    }

    Ok(())
}
