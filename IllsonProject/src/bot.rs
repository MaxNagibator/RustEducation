use crate::db;
use crate::db::PgPool;
use chrono::Utc;
use deadpool_postgres::Pool;
use std::error::Error;
use std::sync::Arc;
use teloxide::types::{MessageId, User};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me},
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

fn make_welcome_keyboard(user_exists: bool) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();

    if user_exists {
        rows.push(vec![
            InlineKeyboardButton::callback("📌 Мой профиль", "command_me"),
            InlineKeyboardButton::callback("🚪 Покинуть", "command_leave"),
        ]);
    } else {
        rows.push(vec![InlineKeyboardButton::callback(
            "🎯 Присоединиться",
            "command_join",
        )]);
    }

    rows.push(vec![InlineKeyboardButton::callback(
        "❓ Помощь",
        "command_help",
    )]);

    InlineKeyboardMarkup::new(rows)
}

async fn handle_join(
    chat_id: ChatId,
    user: &User,
    pool: &Arc<PgPool>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let db_user = db::User {
        user_id: chat_id.0,
        username: user
            .username
            .clone()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Аноним".to_string()),
        first_name: user.first_name.to_string(),
        last_name: user.last_name.clone().map(|s| s.to_string()),
        created_at: Utc::now(),
    };

    db::insert_user(&pool, &db_user)
        .await
        .inspect_err(|e| error!("Ошибка БД: {}", e))
        .unwrap();

    Ok(format!("Добро пожаловать, {}! 🎉", db_user.first_name))
}

async fn handle_help() -> Result<String, Box<dyn Error + Send + Sync>> {
    Ok(Command::descriptions().to_string())
}

async fn handle_me(user_id: i64, pool: &Arc<PgPool>) -> String {
    if let Some(db_user) = db::get_user(&pool, user_id).await.unwrap() {
        format!(
            "📋 Ваш профиль:\nID: {}\nUsername: @{}\nИмя: {}",
            db_user.user_id, db_user.username, db_user.first_name
        )
    } else {
        "Малыш, команда только для членов общества.\nИспользуй /join".to_string()
    }
}
async fn handle_leave(chat_id: ChatId, user: &User, pool: &Arc<PgPool>) -> String {
    let first_name = user.clone().first_name;
    db::delete_user(&pool, chat_id.0)
        .await
        .inspect_err(|e| error!("Ошибка БД: {}", e))
        .unwrap();

    format!("Приходите к нам ещё, {}! 🎉", first_name)
}

async fn edit_or_send_message(
    bot: Bot,
    chat_id: ChatId,
    user_id: i64,
    pool: Arc<PgPool>,
    text: String,
    message_id: Option<MessageId>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let user_exists = db::exists_user(&pool, user_id).await.unwrap();

    if let Some(id) = message_id {
        bot.edit_message_text(chat_id, id, text)
            .reply_markup(make_welcome_keyboard(user_exists))
            .await?;
    } else {
        bot.send_message(chat_id, text)
            .reply_markup(make_welcome_keyboard(user_exists))
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

    let mut answer = String::new();
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                answer = "👋 Добро пожаловать! Я ваш помощник.\n\n\
                    🚀 Чтобы начать:\n\
                    1. Используйте /join для регистрации\n\
                    2. Посмотрите /help для списка команд\n\
                    3. Используйте /me для вашего профиля"
                    .to_string();
            }
            Ok(Command::Help) => answer = handle_help().await?,
            Ok(Command::Join) => answer = handle_join(msg.chat.id, &user, &pool).await?,
            Ok(Command::Me) => answer = handle_me(msg.chat.id.0, &pool).await,
            Ok(Command::Leave) => answer = handle_leave(msg.chat.id, &user, &pool).await,
            Err(_) => {
                if let Some(db_user) = db::get_user(&pool, msg.chat.id.0).await.unwrap() {
                    answer = format!("Привет, {}! Чем могу помочь?", db_user.first_name);
                } else {
                    answer = "Привет! Нажми 'Join' чтобы присоединиться".to_string();
                }
            }
        }
    }

    if !answer.is_empty() {
        edit_or_send_message(bot, msg.chat.id, user.id.0 as i64, pool, answer, None).await?;
    }

    Ok(())
}

async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    pool: Arc<PgPool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let (Some(data), Some(message)) = (q.data, q.message) {
        let chat_id = message.chat().id;
        let user = q.from;

        let text = match data.as_str() {
            "command_join" => handle_join(chat_id, &user, &pool).await?,
            "command_help" => handle_help().await?,
            "command_me" => handle_me(user.id.0 as i64, &pool).await,
            "command_leave" => handle_leave(chat_id, &user, &pool).await,
            &_ => "".to_string(),
        };

        if !text.is_empty() {
            edit_or_send_message(
                bot.clone(),
                chat_id,
                user.id.0 as i64,
                pool,
                text,
                Some(message.id()),
            )
            .await?;
        }
        bot.answer_callback_query(q.id).await?;
    }

    Ok(())
}
