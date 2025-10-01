mod paw_commands;
mod webserver;

use env_logger::Env;
use log::{debug, info};
use poise::serenity_prelude as serenity;
use rand::random_bool;
use serenity::builder::CreateMessage;
use sqlx::{Pool, Postgres};
use std::env;
use std::ops::Add;
use time::OffsetDateTime;
use time::ext::NumericalDuration;

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
            commands: vec![paw()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(random_paw_handler(ctx, event, framework, data))
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

#[poise::command(
    slash_command,
    subcommands(
        "paw_commands::daily::daily",
        "paw_commands::gamble::gamble",
        "paw_commands::balance::balance",
        "paw_commands::give::give",
        "paw_commands::steal::steal",
        "paw_commands::top::top",
    )
)]
pub async fn paw(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[derive(sqlx::FromRow, Debug, PartialEq, Eq, Clone)]
struct RandomPaw {
    message_id: i64,
    channel_id: i64,
    created: OffsetDateTime,
    claimed: bool,
}

pub async fn random_paw_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, AppState, Error>,
    state: &AppState,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            info!("Logged in as {}", data_about_bot.user.name);
        }
        serenity::FullEvent::Message { new_message } => {
            if new_message.author.bot {
                return Ok(());
            }
            debug!(
                "got message: \"{}\" in {}",
                new_message.content,
                new_message.channel_id.get()
            );
            let conn = &state.db;
            let last_random: Option<RandomPaw> = sqlx::query_as(
                "SELECT * FROM random_paws WHERE channel_id = $1 AND claimed = false;",
            )
            .bind(new_message.channel_id.get() as i64)
            .fetch_optional(conn)
            .await?;

            if new_message.content == "🐶" {
                match last_random {
                    Some(paw) => {
                        // If more than 10 minutes old, consider it expired.
                        if paw.created > OffsetDateTime::now_utc().add(10.minutes()) {
                            let mut tx = conn.begin().await?;
                            sqlx::query(
                                "UPDATE random_paws SET claimed = true WHERE message_id = $1;",
                            )
                            .bind(paw.message_id)
                            .execute(&mut *tx)
                            .await?;
                            tx.commit().await?;
                            ctx.http
                                .delete_message(
                                    paw.channel_id.to_string().parse().unwrap(),
                                    paw.message_id.to_string().parse().unwrap(),
                                    None,
                                )
                                .await?;
                            return Ok(());
                        }

                        let claimed =
                            claim_random_paw(conn, paw.clone(), new_message.author.id.get() as i64)
                                .await;
                        if claimed.is_ok() {
                            new_message
                                .reply(
                                    ctx,
                                    format!(
                                        "<@{}> has claimed a paw and now holds onto {}",
                                        new_message.author.id.get(),
                                        claimed.unwrap()
                                    ),
                                )
                                .await?;
                            new_message.delete(ctx).await?;
                            ctx.http
                                .delete_message(
                                    paw.channel_id.to_string().parse().unwrap(),
                                    paw.message_id.to_string().parse().unwrap(),
                                    None,
                                )
                                .await?;
                        } else {
                            println!("{}", claimed.err().unwrap())
                        }
                    }
                    None => {
                        // Do nothing.
                    }
                }
                return Ok(());
            }

            if random_bool(1.0 / 3.0) {
                let new_random = new_message
                    .channel_id
                    .send_message(ctx, CreateMessage::new().content("🐶"))
                    .await?;
                let mut tx = conn.begin().await?;
                sqlx::query("INSERT INTO random_paws VALUES ($1, $2, $3)")
                    .bind(new_random.id.get() as i64)
                    .bind(new_random.channel_id.get() as i64)
                    .bind(OffsetDateTime::now_utc())
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
            }
        }
        _ => {}
    }

    Ok(())
}

async fn claim_random_paw(
    conn: &Pool<Postgres>,
    paw: RandomPaw,
    user_id: i64,
) -> Result<i32, Error> {
    let mut tx = conn.begin().await?;
    let paws: (i32,) = sqlx::query_as("INSERT INTO paws (amount, user_id) VALUES (1, $1) ON CONFLICT(user_id) DO UPDATE SET amount = paws.amount + 1 RETURNING paws.amount;")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

    sqlx::query("UPDATE random_paws SET claimed = true WHERE message_id = $1;")
        .bind(paw.message_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(paws.0)
}
