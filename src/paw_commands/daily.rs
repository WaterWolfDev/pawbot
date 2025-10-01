use crate::{Context, CooldownAction, Error};
use poise::CreateReply;
use std::ops::Add;
use time::ext::NumericalDuration;
use time::macros::time;

#[poise::command(slash_command)]
pub async fn daily(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get() as i64;
    let conn = &ctx.data().db;

    let expiry: Option<(time::OffsetDateTime,)> =
        sqlx::query_as("SELECT expires FROM cooldowns WHERE user_id = $1 AND action = $2")
            .bind(user_id)
            .bind(CooldownAction::Paw)
            .fetch_optional(conn)
            .await?;
    let now = time::OffsetDateTime::now_utc();

    if expiry.is_some() && expiry.unwrap().0 > now {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .content("You've already claimed your daily paw!"),
        )
        .await?;
        return Ok(());
    }
    let mut tx = conn.begin().await?;
    sqlx::query("DELETE FROM cooldowns WHERE user_id = $1 AND action = $2")
        .bind(user_id)
        .bind(CooldownAction::Paw)
        .bind(ctx.guild_id().unwrap().get() as i64)
        .execute(&mut *tx)
        .await?;

    let cooldown_expires = now.add(1.days()).replace_time(time!(00:00));
    sqlx::query("INSERT INTO cooldowns (user_id, action, expires) VALUES ($1,$2,$3)")
        .bind(user_id)
        .bind(CooldownAction::Paw)
        .bind(cooldown_expires)
        .execute(&mut *tx)
        .await?;

    let paws: (i32,) = sqlx::query_as("INSERT INTO paws (user_id, amount) VALUES ($1, $2) ON CONFLICT(user_id) DO UPDATE SET amount = paws.amount + 1 RETURNING paws.amount;")
        .bind(user_id)
        .bind(1)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;

    ctx.reply(format!(
        "You claimed your daily paw, and now hold onto {} paws!",
        paws.0
    ))
    .await?;

    Ok(())
}
