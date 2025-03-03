use std::time::Duration;
use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use futures::future::join_all;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user));

    let listener = tokio::net::TcpListener::bind("localhost:3000").await.unwrap();
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

async fn sleep_time_complex(name:u64, secs: u64) {
    println!("bla {} start", name);
    sleep_time(secs).await;
    println!("bla {name} finish {secs}",);
}

async fn create_user(
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
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
