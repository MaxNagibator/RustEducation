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

fn make_welcome_keyboard(user: Option<db::User>) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();

    if user.is_some() {
        let obj = user.unwrap();
        let subscribes = obj.subscribes.unwrap_or(0);
        let subscribes_string = i32_to_bit_string(subscribes);
        let mut subscribes_chars = subscribes_string.chars();

        let is_test4 = subscribes_chars.nth(28) == Some('1');
        let is_test3 = subscribes_chars.nth(0) == Some('1');
        let is_memasi = subscribes_chars.nth(0) == Some('1');
        let is_stream = subscribes_chars.nth(0) == Some('1');

        let test: InlineKeyboardButton;
        let test2: InlineKeyboardButton;
        let test3: InlineKeyboardButton;
        let test4: InlineKeyboardButton;
        // todo сделать через цикл по массиву, туды сюды
        // todo хуёвый код, 31 и текст куданить в константы, массив айди нейм
        if is_stream {
            test = InlineKeyboardButton::callback("Выкл нап.стримов", "command_enable_0_31");
        } else {
            test = InlineKeyboardButton::callback("Вкл нап.стримов", "command_enable_1_31");
        }
        if is_memasi {
            test2 = InlineKeyboardButton::callback("Выкл мемасы", "command_enable_0_30");
        } else {
            test2 = InlineKeyboardButton::callback("Вкл мемасы", "command_enable_1_30");
        }
        if is_test3 {
            test3 = InlineKeyboardButton::callback("Выкл 3тест", "command_enable_0_29");
        } else {
            test3 = InlineKeyboardButton::callback("Вкл 3тест", "command_enable_1_29");
        }
        if is_test4 {
            test4 = InlineKeyboardButton::callback("Выкл 4тест", "command_enable_0_28");
        } else {
            test4 = InlineKeyboardButton::callback("Вкл 4тест", "command_enable_1_28");
        }

        rows.push(vec![
            InlineKeyboardButton::callback("📌 Мой профиль", "command_me"),
            InlineKeyboardButton::callback("🚪 Покинуть", "command_leave"),
        ]);
        rows.push(vec![test, test2]);
        rows.push(vec![test3, test4]);
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

fn i32_to_bit_string(n: i32) -> String {
    (0..32)
        .rev()
        .map(|i| if (n >> i) & 1 == 1 { '1' } else { '0' })
        .collect()
}

fn bit_string_to_i32(s: &str) -> Option<i32> {
    if s.len() != 32 {
        return None;
    }

    let mut result: i32 = 0;

    for (i, c) in s.chars().enumerate() {
        match c {
            '0' => {}
            '1' => {
                if i == 0 {
                    // MSB is set - negative number
                    result = !0 ^ ((1 << (31 - i)) - 1);
                }
                result |= 1 << (31 - i);
            }
            _ => return None,
        }
    }

    Some(result)
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
        subscribes: Some(0),
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
    //let user_exists = db::exists_user(&pool, user_id).await.unwrap();
    let user = db::get_user(&pool, user_id).await.unwrap();

    if let Some(id) = message_id {
        bot.edit_message_text(chat_id, id, text)
            .reply_markup(make_welcome_keyboard(user))
            .await?;
    } else {
        bot.send_message(chat_id, text)
            .reply_markup(make_welcome_keyboard(user))
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

fn replace_char_at_index(s: &str, index: usize, new_char: char) -> String {
    s.chars()
        .enumerate()
        .map(|(i, c)| if i == index { new_char } else { c })
        .collect()
}

async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    pool: Arc<PgPool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let (Some(data), Some(message)) = (q.data, q.message) {
        let chat_id = message.chat().id;
        let user = q.from;

        let comandText = data.as_str();
        if comandText.starts_with("command_enable") {
            let substring = &comandText["command_enable".len() + 1..];
            let parts: Vec<&str> = substring.split('_').collect();
            let enable = parts.get(0).unwrap().chars().nth(0);
            if let Some(part) = parts.get(1) {
                // Step 3: Convert to integer
                match part.parse::<i32>() {
                    Ok(num) => {
                        println!("Value at index {}: {}", 1, num);
                        if let Some(mut db_user) = db::get_user(&pool, chat_id.0).await.unwrap() {
                            let subs = db_user.subscribes;
                            let subs_str = i32_to_bit_string(subs.unwrap_or(0));

                            let n_us = usize::try_from(num).unwrap();
                            let subs = replace_char_at_index(&subs_str, n_us, enable.unwrap());
                            let new_subs = bit_string_to_i32(&subs);
                            db_user.subscribes = new_subs;
                            db::insert_user(&pool, &db_user).await.unwrap();
                        } else {
                            println!("user with id {} not found", chat_id.0);
                        }
                    }

                    Err(_) => println!("Failed to parse '{}' as an integer", part),
                }
            } else {
                println!("Index {} is out of bounds", 1);
            }
        }
        if comandText.starts_with("command_disable") {}

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
