use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use axum::serve::Serve;
use log::{error, info};
use sqlx::{Pool, Postgres};
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub async fn new(conn: Pool<Postgres>) -> Serve<TcpListener, Router, Router> {
    let router = Router::new()
        .route("/", get(root_handler))
        .route("/healthz", get(health_handler))
        .with_state(conn);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("Webserver listening on {}", addr);

    axum::serve(listener, router)
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
        "<h1>Waterwolf Pawbot</h1>{} total paws<br />{} total users",
        total_paws.0, total_users.0
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
