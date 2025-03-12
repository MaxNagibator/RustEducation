use crate::db::PgPool;
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, io};
use tracing::{debug, error, info};

pub async fn run_server(
    pool: Arc<PgPool>,
    addr: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
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

async fn root() -> Html<String> {
    let full_path = format!(
        "{}\\src\\html\\index.html",
        std::env::current_dir().unwrap().to_str().unwrap()
    );
    let page = fs::read_to_string(full_path);

    if page.is_err() {
        page.inspect_err(|e| error!("Ошибка чтения бла {} по пути", e));
        let string2 = "<h1>Error loading HTML file</h1>".to_string();
        return Html(string2);
    }

    let page = page.unwrap();
    Html(page)
}

async fn create_user(Json(payload): Json<CreateUser>) -> (StatusCode, Json<UserDto>) {
    let user = UserDto {
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
struct UserDto {
    id: u64,
    username: String,
}
