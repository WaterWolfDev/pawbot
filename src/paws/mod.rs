use log::{error, info};
use sqlx::{Pool, Postgres};
use crate::{AppState, Context, Error};
use poise::serenity_prelude as serenity;
use tokio::sync::watch::Receiver;

pub mod balance;
pub mod daily;
pub mod gamble;
pub mod give;
pub mod steal;
pub mod top;
pub mod event_handler;

pub async fn client(conn: Pool<Postgres>, mut receiver: Receiver<()>, token: String) -> () {
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![paw()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler::handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(AppState { db: conn })
            })
        })
        .build();


    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Err creating client");

    let shard_manager = client.shard_manager.clone();
    let graceful_shutdown_future = async move {
        receiver.changed().await.ok();
        info!("shutting down...");
        // Use the shard manager to gracefully shut down the bot.
        shard_manager.shutdown_all().await;
    };

    tokio::select! {
        result = client.start() => {
            if let Err(why) = result {
                error!("Poise client error: {:?}", why);
            }
        },
        _ = graceful_shutdown_future => {},
    }
}

#[poise::command(
    slash_command,
    subcommands(
        "daily::daily",
        "gamble::gamble",
        "balance::balance",
        "give::give",
        "steal::steal",
        "top::top",
    )
)]
pub async fn paw(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
