mod paw_commands;

use sqlx::{Connection, Pool, Postgres};
use std::env;
use poise::serenity_prelude as serenity;

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type)]
#[sqlx(type_name = "cooldown_action", rename_all = "lowercase")]
pub enum CooldownAction {
    Paw,
    Steal,
    Gamble
}

struct AppState {
    db: Pool<Postgres>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, AppState, Error>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let db_url = env::var("DATABASE_URL").expect("missing DATABASE_URL");
    let intents = serenity::GatewayIntents::non_privileged();

    let conn = Pool::connect(&db_url).await.expect("Can't connect to database");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![paw()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(AppState {
                    db: conn,
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}

#[poise::command(slash_command, subcommands(
    "paw_commands::daily::daily",
    "paw_commands::gamble::gamble",
))]
pub async fn paw(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}