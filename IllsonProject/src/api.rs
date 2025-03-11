use crate::db::PgPool;
use std::error::Error;

use axum::routing::{get, post};
use axum::{Json, Router};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

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
