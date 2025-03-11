use crate::db;
use crate::db::PgPool;
use deadpool_postgres::Pool;
use std::error::Error;
use std::sync::Arc;
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
use tracing::{error, info};

pub async fn run_bot(pool: Arc<Pool>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bot = Bot::from_env();
    bot.set_my_commands(Command::bot_commands()).await?;

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_map(|update: Update| update.from().cloned())
                .endpoint(process_message),
        )
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![pool.clone()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Показать справку по командам", aliases = ["h", "?"])]
    Help,
    #[command(description = "Начать работу с ботом")]
    Start,
    #[command(description = "Присоединиться к системе")]
    Join,
    #[command(description = "Покинуть систему")]
    Leave,
    #[command(description = "Показать информацию о себе")]
    Me,
}

fn make_welcome_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("🎯 Присоединиться", "command_join"),
            InlineKeyboardButton::callback("❓ Помощь", "command_help"),
        ],
        vec![InlineKeyboardButton::callback(
            "📌 Мой профиль",
            "command_me",
        )],
    ])
}

async fn handle_join(
    bot: Bot,
    chat_id: ChatId,
    user: &User,
    pool: Arc<PgPool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let username = &user.username.clone().unwrap_or_default();
    let first_name = &user.first_name;

    db::insert_user(&pool, chat_id.0 as i32, username, first_name)
        .await
        .inspect_err(|e| error!("Ошибка БД: {}", e))
        .unwrap();

    bot.send_message(chat_id, format!("Добро пожаловать, {}! 🎉", first_name))
        .await?;

    Ok(())
}

async fn handle_help(bot: Bot, chat_id: ChatId) -> Result<(), Box<dyn Error + Send + Sync>> {
    bot.send_message(chat_id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn handle_me(
    bot: Bot,
    chat_id: ChatId,
    user_id: i32,
    pool: Arc<PgPool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(db_user) = db::get_user(&pool, user_id).await.unwrap() {
        let profile_text = format!(
            "📋 Ваш профиль:\nID: {}\nUsername: @{}\nИмя: {}",
            db_user.chat_id, db_user.username, db_user.first_name
        );
        bot.send_message(chat_id, profile_text).await?;
    } else {
        bot.send_message(
            chat_id,
            "Малыш, команда только для членов общества.\nИспользуй /join",
        )
        .await?;
    }
    Ok(())
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
                let welcome_text = "👋 Добро пожаловать! Я ваш помощник.\n\n\
                    🚀 Чтобы начать:\n\
                    1. Используйте /join для регистрации\n\
                    2. Посмотрите /help для списка команд\n\
                    3. Используйте /me для вашего профиля";

                bot.send_message(msg.chat.id, welcome_text)
                    .reply_markup(make_welcome_keyboard())
                    .await?;
            }
            Ok(Command::Help) => handle_help(bot, msg.chat.id).await?,
            Ok(Command::Join) => handle_join(bot, msg.chat.id, &user, pool).await?,
            Ok(Command::Me) => handle_me(bot, msg.chat.id, msg.chat.id.0 as i32, pool).await?,
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

async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    pool: Arc<PgPool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(data) = q.data {
        let chat_id = q.message.unwrap().chat().id;
        let user = q.from;

        match data.as_str() {
            "command_join" => handle_join(bot.clone(), chat_id, &user, pool.clone()).await?,
            "command_help" => handle_help(bot.clone(), chat_id).await?,
            "command_me" => handle_me(bot.clone(), chat_id, user.id.0 as i32, pool.clone()).await?,
            _ => {}
        }

        bot.answer_callback_query(q.id).await?;
    }

    Ok(())
}
