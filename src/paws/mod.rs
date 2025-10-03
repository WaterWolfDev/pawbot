use crate::{AppState, Context, Error};
use log::{error, info};
use poise::serenity_prelude as serenity;
use sqlx::{Pool, Postgres};
use tokio::sync::watch::Receiver;

pub mod balance;
pub mod daily;
pub mod event_handler;
pub mod gamble;
pub mod give;
pub mod steal;
pub mod top;

pub async fn client(conn: Pool<Postgres>, mut receiver: Receiver<()>, token: String, guild_id: String) -> () {
    let guild_id = guild_id.parse::<serenity::GuildId>().unwrap();
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![paw(), register()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler::handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id).await?;
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

#[poise::command(prefix_command)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
