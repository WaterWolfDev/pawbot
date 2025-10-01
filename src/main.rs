mod paws;
mod webserver;

use env_logger::Env;
use poise::serenity_prelude as serenity;
use sqlx::{Pool, Postgres};
use std::env;
use crate::paws::event_handler;

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
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let conn = Pool::connect(&db_url)
        .await
        .expect("Can't connect to database");

    let webserver = webserver::new(conn.clone()).await;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![paws::paw()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler::handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(AppState { db: conn.clone() })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Err creating client");

    tokio::join!(webserver, client.start())
        .1
        .expect("Discord client failed");
}
