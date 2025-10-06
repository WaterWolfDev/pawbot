use crate::{AppState, CooldownAction, Error};
use log::{debug, info};
use poise::serenity_prelude as serenity;
use rand::{random_bool, random_range};
use serenity::builder::CreateMessage;
use sqlx::{Pool, Postgres};
use std::ops::Add;
use std::time::Duration;
use time::OffsetDateTime;
use time::ext::NumericalDuration;
use tokio::time::sleep;

const WHITELIST_CHANNELS: &'static [&'static str] = &[
    "967074642076516367",
    "1074813849712214127",
    "1059235902771187793",
    "999826882461700258",
    "1042114977928065076",
    "1000506173713297431",
    "1008859522309312602",
    "1005528820075466964",
    "1028658894333038672",
    "979999280339247125",
    "1050847825723916382",
    "1041479219097645148",
    "1272161749155446794",
];

#[derive(sqlx::FromRow, Debug, PartialEq, Eq, Clone)]
struct RandomPaw {
    message_id: i64,
    channel_id: i64,
    created: OffsetDateTime,
    claimed: bool,
}

pub async fn handler(
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
            let channel_id = new_message.channel_id.get() as i64;
            if new_message.author.bot
                || !WHITELIST_CHANNELS.contains(&channel_id.to_string().as_str())
            {
                return Ok(());
            }
            debug!(
                "got message: \"{}\" in {}",
                new_message.content,
                new_message.channel_id.get()
            );
            let conn = &state.db;
            let paw = latest_random_paw(ctx, conn, channel_id).await;

            if new_message.content == "🐶" && paw.is_ok() {
                let paw = paw?;
                let claimed =
                    claim_random_paw(conn, paw.clone(), new_message.author.id.get() as i64).await;
                if claimed.is_ok() {
                    let claimed_message = new_message
                        .reply(
                            ctx,
                            format!(
                                "<@{}> has claimed a paw and now holds onto {}.\n-# This message will self destruct in 5 seconds.",
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
                    let c_ctx = ctx.clone();
                    tokio::spawn(async move {
                        debug!("deleting claim message after 5 seconds");
                        sleep(Duration::from_secs(5)).await;
                        claimed_message.delete(c_ctx).await.expect("unable to delete");
                    });
                } else {
                    println!("{}", claimed.err().unwrap())
                }
                return Ok(());
            }

            let expiry: Option<(OffsetDateTime,)> =
                sqlx::query_as("SELECT expires FROM cooldowns WHERE action = $1")
                    .bind(CooldownAction::Spawn)
                    .fetch_optional(conn)
                    .await?;
            let now = OffsetDateTime::now_utc();

            if expiry.is_some() && expiry.unwrap().0 > now {
                return Ok(());
            }
            if random_bool(1.0 / 3.0) && paw.is_err() {
                debug!(
                    "rolled random change, spawning paw in {}",
                    new_message.channel_id
                );
                let new_random = new_message
                    .channel_id
                    .send_message(ctx, CreateMessage::new().content("🐶"))
                    .await?;
                let mut tx = conn.begin().await?;
                sqlx::query("DELETE FROM cooldowns WHERE action = $1")
                    .bind(CooldownAction::Spawn)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query("INSERT INTO random_paws VALUES ($1, $2, $3)")
                    .bind(new_random.id.get() as i64)
                    .bind(new_random.channel_id.get() as i64)
                    .bind(OffsetDateTime::now_utc())
                    .execute(&mut *tx)
                    .await?;

                let timeout = random_range(2..20);
                let cooldown_expires = now.add(timeout.minutes());
                sqlx::query("INSERT INTO cooldowns (action, expires) VALUES ($1,$2)")
                    .bind(CooldownAction::Spawn)
                    .bind(cooldown_expires)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                debug!("random spawn expires in {} minutes", timeout);
            }
        }
        _ => {}
    }

    Ok(())
}

async fn latest_random_paw(
    ctx: &serenity::Context,
    conn: &Pool<Postgres>,
    channel_id: i64,
) -> Result<RandomPaw, Error> {
    let last_random: Option<RandomPaw> =
        sqlx::query_as("SELECT * FROM random_paws WHERE channel_id = $1 AND claimed = false;")
            .bind(channel_id)
            .fetch_optional(conn)
            .await?;

    match last_random {
        Some(paw) => {
            // If more than 10 minutes old, consider it expired.
            if paw.created < OffsetDateTime::now_utc().add(-10.minutes()) {
                debug!(
                    "random paw in {} is more than 10 minutes old, expiring",
                    channel_id
                );
                let mut tx = conn.begin().await?;
                sqlx::query("UPDATE random_paws SET claimed = true WHERE message_id = $1;")
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
                return Err(Error::from("expired paw"));
            }
            Ok(paw)
        }
        None => Err(Error::from("no existing paw")),
    }
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
