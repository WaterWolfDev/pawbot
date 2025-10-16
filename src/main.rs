mod paws;
mod webserver;

use log::{error, info};
use sqlx::{Pool, Postgres};
use std::time::Duration;
use std::{env, error};
use sqlx::pool::PoolOptions;
use tokio::sync::watch;

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type)]
#[sqlx(type_name = "cooldown_action", rename_all = "lowercase")]
pub enum CooldownAction {
    Paw,
    Steal,
    Gamble,
    Spawn,
}

pub struct AppState {
    db: Pool<Postgres>,
}
type Error = Box<dyn error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, AppState, Error>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let (shutdown_tx, shutdown_rx) = watch::channel(());

    let token = env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let guild_id = env::var("DISCORD_GUILD_ID").expect("missing DISCORD_GUILD_ID");
    let db_url = env::var("DATABASE_URL").expect("missing DATABASE_URL");

    let conn: Pool<Postgres> = PoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .idle_timeout(Some(Duration::from_secs(300)))
        .connect(db_url.as_str())
        .await
        .expect("couldn't connect to database");

    sqlx::migrate!("./migrations")
        .run(&conn)
        .await
        .expect("Couldn't run migrations");

    let poise_handle = tokio::spawn(paws::client(conn.clone(), shutdown_rx.clone(), token, guild_id));
    let webserver_handle = tokio::spawn(webserver::new(conn.clone(), shutdown_rx.clone()));

    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            info!("Ctrl+C received. Initiating graceful shutdown...");
        }
        Err(err) => {
            error!(
                "Failed to listen for Ctrl+C: {}. Shutting down immediately.",
                err
            );
        }
    }
    drop(shutdown_tx);

    let _ = tokio::time::timeout(Duration::from_secs(1), poise_handle).await;
    let _ = tokio::time::timeout(Duration::from_secs(1), webserver_handle).await;

    info!("exiting");
}
