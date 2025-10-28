use std::env;
use axum::{Router, extract::State, http::StatusCode, response::Html, routing::get};
use log::{error, info};
use sqlx::{Pool, Postgres};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::watch::Receiver;

pub async fn new(conn: Pool<Postgres>, mut receiver: Receiver<()>) -> () {
    let router = Router::new()
        .route("/", get(root_handler))
        .route("/healthz", get(health_handler))
        .with_state(conn);

    let graceful_shutdown_future = async move {
        // `changed()` resolves when a new value is sent, or the sender is dropped.
        receiver.changed().await.ok();
        info!("shutting down...");
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("Webserver listening on {}", addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(graceful_shutdown_future)
        .await
        .expect("Could not start webserver");
}

async fn root_handler(State(conn): State<Pool<Postgres>>) -> Html<String> {
    if let Err(e) = conn.acquire().await {
        error!("Failed to connect to DB: {}", e);
        return Html("Unable to connect to the database".to_string()).into();
    }
    let total_paws: (i64,) = sqlx::query_as("SELECT SUM(amount) FROM PAWS;")
        .fetch_one(&conn)
        .await
        .unwrap_or((0i64,));

    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM PAWS;")
        .fetch_one(&conn)
        .await
        .unwrap_or((0i64,));
    Html(format!(
        "<h1>Waterwolf Pawbot</h1>{} total paws<br />{} total users<br />commit {}",
        total_paws.0, total_users.0, env::var("GIT_HASH").unwrap_or(String::from("unknown"))
    ))
    .into()
}

async fn health_handler(State(conn): State<Pool<Postgres>>) -> StatusCode {
    if let Err(e) = conn.acquire().await {
        error!("Failed to connect to DB: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}
