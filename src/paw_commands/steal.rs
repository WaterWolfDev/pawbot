use std::ops::Add;
use poise::CreateReply;
use rand::{random_bool, random_range};
use serenity::all::User;
use time::ext::NumericalDuration;
use crate::{Context, CooldownAction, Error};

#[poise::command(slash_command)]
pub async fn steal(
    ctx: Context<'_>,
    who: User,
    count: i32
) -> Result<(), Error> {
    if count > 10 {
        ctx.reply("You can only steal a maximum of 10 paws!".to_string()).await?;
        return Ok(());
    }

    let user_id = ctx.author().id.get() as i64;
    let target_id = who.id.get() as i64;
    let conn = &ctx.data().db;

    let expiry: Option<(time::OffsetDateTime,)> = sqlx::query_as("SELECT expires FROM cooldowns WHERE user_id = $1 AND action = $2")
        .bind(user_id)
        .bind(CooldownAction::Steal)
        .fetch_optional(conn)
        .await?;
    let now = time::OffsetDateTime::now_utc();

    if expiry.is_some() && expiry.unwrap().0 > now {
        ctx.send(CreateReply::default()
            .ephemeral(true)
            .content("The fuzz is hot on your tail, lay low for a while.")
        ).await?;
        return Ok(());
    }

    let paws: Option<(i32,)> = sqlx::query_as("SELECT amount FROM paws WHERE user_id = $1;")
        .bind(user_id)
        .fetch_optional(conn)
        .await?;

    let paws: i32 = match paws {
        Some(c) => c.0,
        None => 0
    };

    if paws == 0 || paws < count {
        ctx.reply("You can only steal as many paws as you have!".to_string()).await?;
        return Ok(());
    }

    let target_paws: Option<(i32,)> = sqlx::query_as("SELECT amount FROM paws WHERE user_id = $1;")
        .bind(target_id)
        .fetch_optional(conn)
        .await?;

    let target_paws: i32 = match target_paws {
        Some(c) => c.0,
        None => 0
    };

    if target_paws == 0 || target_paws < count {
        ctx.reply("That user doesn't have enough paws!".to_string()).await?;
        return Ok(());
    }

    let result = random_bool(0.5);
    let mut new_paws = paws + count;
    let (sender, receiver) = match result {
        true => (target_id, user_id),
        false => (user_id, target_id)
    };
    if !result {
        new_paws = paws - count;
    }

    let mut tx = conn.begin().await?;

    sqlx::query("DELETE FROM cooldowns WHERE user_id = $1 AND action = $2")
        .bind(user_id)
        .bind(CooldownAction::Steal)
        .bind(ctx.guild_id().unwrap().get() as i64)
        .execute(&mut *tx)
        .await?;

    let timeout = random_range(3..10);
    let cooldown_expires = now.add(timeout.minutes());
    sqlx::query("INSERT INTO cooldowns (user_id, action, expires) VALUES ($1,$2,$3)")
        .bind(user_id)
        .bind(CooldownAction::Steal)
        .bind(cooldown_expires)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE paws SET amount = amount - $1 WHERE user_id = $2;")
        .bind(count)
        .bind(sender)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE paws SET amount = amount + $1 WHERE user_id = $2;")
        .bind(count)
        .bind(receiver)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let message = build_steal_result(result, count, new_paws, who);
    ctx.reply(message).await?;

    Ok(())
}

fn build_steal_result(won: bool, count: i32, new_count: i32, target: User) -> String {
    let name = target.name.clone();
    let outcome_phrase = if won { "paid off" } else { "sucked" };
    let result_verb = if won { "stole" } else { "gave" };
    let result_target_prefix = if won { "from" } else { "to" };
    let preposition_phrase = if won { "giving you" } else { "leaving you with" };
    let trend_emoji = if won { '📈' } else { '📉' };

    let plural_s_count = if count == 1 { "" } else { "s" };
    let plural_s_new_count = if new_count == 1 { "" } else { "s" };

    format!(
        "Your thievery {outcome_phrase}, you {result_verb} {count} paw{plural_s_count} {result_target_prefix} {name}, \
        {preposition_phrase} a total of {new_count} paw{plural_s_new_count}. \
        {trend_emoji}"
    )
}