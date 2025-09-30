use std::ops::Add;
use poise::CreateReply;
use rand::{random_bool, random_range};
use time::ext::NumericalDuration;
use crate::{Context, CooldownAction, Error};

#[poise::command(slash_command)]
pub async fn gamble(
    ctx: Context<'_>,
    #[description = "Number of paws to gamble"] count: i32,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get() as i64;
    let conn = &ctx.data().db;

    let expiry: Option<(time::OffsetDateTime,)> = sqlx::query_as("SELECT expires FROM cooldowns WHERE user_id = $1 AND action = $2")
        .bind(user_id)
        .bind(CooldownAction::Gamble)
        .fetch_optional(conn)
        .await?;
    let now = time::OffsetDateTime::now_utc();

    if expiry.is_some() && expiry.unwrap().0 > now {
        ctx.send(CreateReply::default()
            .ephemeral(true)
            .content("⛔🐶 Gambling addiction is a serious problem. Regulations require a wait. Try again later...")
        ).await?;
        return Ok(());
    }

    let current_paws: Option<(i32,)> = sqlx::query_as("SELECT amount FROM paws WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(conn)
        .await?;

    let can_gamble: bool = match current_paws {
        Some(c) => c.0 > 0 || c.0 >= count,
        None => false
    };
    if !can_gamble {
        ctx.send(CreateReply::default()
            .ephemeral(true)
            .content("You can only gamble as many paws as you have! (Up to 10)")
        ).await?;
        return Ok(());
    }

    let result = random_bool(0.5);
    let current_paws = current_paws.unwrap().0;
    let mut new_paws = current_paws + count;
    if !result {
        new_paws = current_paws + (count * -1);
    }
    if new_paws < 0 { new_paws = 0; /* How did you get here. */ }

    let mut tx = conn.begin().await?;
    sqlx::query("DELETE FROM cooldowns WHERE user_id = $1 AND action = $2")
        .bind(user_id)
        .bind(CooldownAction::Gamble)
        .bind(ctx.guild_id().unwrap().get() as i64)
        .execute(&mut *tx)
        .await?;

    let timeout = random_range(2..5);
    let cooldown_expires = now.add(timeout.minutes());
    sqlx::query("INSERT INTO cooldowns (user_id, action, expires) VALUES ($1,$2,$3)")
        .bind(user_id)
        .bind(CooldownAction::Gamble)
        .bind(cooldown_expires)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE paws SET amount = $1 WHERE user_id = $2;")
        .bind(new_paws)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let message = build_result(result, count, new_paws);
    ctx.reply(message).await?;

    Ok(())
}

fn build_result(won: bool, count: i32, new_count: i32) -> String {
    let outcome_phrase = if won { "paid off" } else { "sucked" };
    let result_verb = if won { "won" } else { "lost" };
    let preposition_phrase = if won { "giving you" } else { "leaving you with" };
    let trend_emoji = if won { '📈' } else { '📉' };

    let plural_s_count = if count == 1 { "" } else { "s" };
    let plural_s_new_count = if new_count == 1 { "" } else { "s" };

    format!(
        "Your gambling {outcome_phrase}, you {result_verb} {count} paw{plural_s_count}, \
        {preposition_phrase} a total of {new_count} paw{plural_s_new_count}. \
        {trend_emoji}"
    )
}