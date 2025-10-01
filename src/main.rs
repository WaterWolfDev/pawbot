mod paws;
mod webserver;

use env_logger::Env;
use sqlx::{Pool, Postgres};
use std::env;

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type)]
#[sqlx(type_name = "cooldown_action", rename_all = "lowercase")]
pub enum CooldownAction {
    Paw,
    Steal,
    Gamble,
}

pub struct AppState {
    db: Pool<Postgres>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, AppState, Error>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let token = env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let db_url = env::var("DATABASE_URL").expect("missing DATABASE_URL");

    let conn = Pool::connect(&db_url)
        .await
        .expect("Can't connect to database");

    let webserver = webserver::new(conn.clone()).await;
    let mut client = paws::client(conn.clone(), token).await;

    tokio::join!(webserver, client.start())
        .1
        .expect("Discord client failed");
}
